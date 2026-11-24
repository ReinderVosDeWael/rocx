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
    let (source, expected, edits, output) = match request {
        Request::Inspect { source } => (source, None, None, None),
        Request::Preview {
            source,
            snapshot,
            edits,
        } => (source, Some(snapshot), Some(edits), None),
        Request::SaveCopy {
            source,
            snapshot,
            edits,
            output,
        } => (source, Some(snapshot), Some(edits), Some(output)),
    };
    let bytes = fs::read(&source).map_err(|e| format!("read source: {e}"))?;
    let snapshot = format!("{:x}", Sha256::digest(&bytes));
    if expected.as_ref().is_some_and(|value| value != &snapshot) {
        return Err("stale snapshot: source bytes differ from inspection".into());
    }
    let doc = ImportedDocument::open(&bytes).map_err(|e| format!("open document: {e}"))?;
    let Some(edits) = edits else {
        return Ok(rocx_protocol::inspect(&doc, snapshot));
    };
    let preview = rocx_protocol::preview(&doc, edits)?;
    if let Some(path) = output {
        let result = preview.to_docx().map_err(|e| format!("encode: {e}"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("create output: {e}"))?;
        file.write_all(&result)
            .map_err(|e| format!("write output (file may be incomplete): {e}"))?;
    }
    Ok(rocx_protocol::report(&preview, snapshot))
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
