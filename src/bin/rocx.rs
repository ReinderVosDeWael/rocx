use rocx::{
    Alignment,
    editor::{BodyBlock, ImportedDocument, ParagraphAlignmentEdit},
};
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Edit {
    target: usize,
    alignment: String,
}

fn alignment(value: &str) -> Result<Alignment, String> {
    match value {
        "left" => Ok(Alignment::Left),
        "center" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        "justify" => Ok(Alignment::Justify),
        "distribute" => Ok(Alignment::Distribute),
        "justify-medium" => Ok(Alignment::JustifyMedium),
        "justify-high" => Ok(Alignment::JustifyHigh),
        "justify-low" => Ok(Alignment::JustifyLow),
        "thai-justify" => Ok(Alignment::ThaiJustify),
        _ => Err(format!("unknown alignment: {value}")),
    }
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
        let body: Vec<Value> = doc
            .body()
            .iter()
            .enumerate()
            .map(|(index, block)| match block {
                BodyBlock::Paragraph(p) => {
                    json!({"type":"paragraph", "target":index, "text":p.text,
                "alignment_xml":p.alignment_xml, "has_opaque_content":p.has_opaque_content})
                }
                BodyBlock::Table(_) => json!({"type":"table", "index":index}),
                BodyBlock::Opaque(name) => json!({"type":"opaque", "index":index, "name":name}),
                _ => json!({"type":"unknown", "index":index}),
            })
            .collect();
        return Ok(json!({"snapshot":snapshot, "body":body}));
    };
    let edits: Vec<ParagraphAlignmentEdit> = edits
        .into_iter()
        .map(|edit| {
            let Some(BodyBlock::Paragraph(paragraph)) = doc.body().get(edit.target) else {
                return Err(format!(
                    "target {} is not a direct body paragraph",
                    edit.target
                ));
            };
            Ok(ParagraphAlignmentEdit {
                target: paragraph.anchor.clone(),
                alignment: alignment(&edit.alignment)?,
            })
        })
        .collect::<Result<_, String>>()?;
    let preview = doc.preview(&edits).map_err(|e| format!("preview: {e}"))?;
    let changes: Vec<Value> = preview.changes().iter().map(|change| json!({
        "target": change.body_index, "before": change.before, "after": format!("{:?}", change.after)
    })).collect();
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
    Ok(json!({"snapshot":snapshot, "changes":changes}))
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
