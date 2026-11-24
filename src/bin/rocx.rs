mod rocx_protocol;
use rocx::editor::ImportedDocument;
use rocx_protocol::Edit;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
};

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case", deny_unknown_fields)]
enum Request {
    Inspect {
        source: String,
        #[serde(default)]
        metadata_fields: Vec<String>,
        #[serde(default)]
        custom_properties: Vec<String>,
        table_owner: Option<String>,
        #[serde(default)]
        numbering_targets: Vec<usize>,
    },
    Preview {
        source: String,
        snapshot: String,
        edits: Vec<Edit>,
    },
    SaveCopy {
        source: String,
        snapshot: String,
        edits: Vec<Edit>,
        output: String,
    },
}

fn execute(request: Request) -> Result<Value, String> {
    let source = match &request {
        Request::Inspect { source, .. }
        | Request::Preview { source, .. }
        | Request::SaveCopy { source, .. } => source,
    };
    let bytes = fs::read(source).map_err(|e| format!("read source: {e}"))?;
    let snapshot = format!("{:x}", Sha256::digest(&bytes));
    let open = || ImportedDocument::open(&bytes).map_err(|e| format!("open document: {e}"));
    let (expected, edits, output) = match request {
        Request::Inspect {
            metadata_fields,
            custom_properties,
            table_owner,
            numbering_targets,
            ..
        } => {
            return rocx_protocol::inspect(
                &open()?,
                snapshot,
                metadata_fields,
                custom_properties,
                table_owner.as_deref(),
                numbering_targets,
            );
        }
        Request::Preview {
            snapshot, edits, ..
        } => (snapshot, edits, None),
        Request::SaveCopy {
            snapshot,
            edits,
            output,
            ..
        } => (snapshot, edits, Some(output)),
    };
    if expected != snapshot {
        return Err("stale snapshot: source bytes differ from inspection".into());
    }
    let (result, mut report) = rocx_protocol::preview(&open()?, edits, snapshot)?;
    report["output_snapshot"] = json!(format!("{:x}", Sha256::digest(&result)));
    if let Some(path) = output {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("create output: {e}"))?;
        file.write_all(&result)
            .map_err(|e| format!("write output (file may be incomplete): {e}"))?;
    }
    Ok(report)
}

fn main() {
    let result = (|| {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|e| e.to_string())?;
        let request: Request =
            serde_json::from_str(&input).map_err(|e| format!("invalid request: {e}"))?;
        execute(request)
    })();
    match result {
        Ok(value) => println!("{}", json!({"ok":true, "result":value})),
        Err(error) => {
            println!("{}", json!({"ok":false, "error":error}));
            std::process::exit(1);
        }
    }
}
