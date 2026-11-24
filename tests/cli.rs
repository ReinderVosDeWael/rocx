use serde_json::{Value, json};
use std::{
    fs,
    io::{Cursor, Read, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn parts(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut entry = zip.by_index(i).unwrap();
            let name = entry.name().unwrap().into_owned();
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            (name, contents)
        })
        .collect()
}

fn run(request: Value) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rocx"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn inspect_preview_save_and_reject_invalid_batches() {
    let dir = std::env::temp_dir().join(format!(
        "rocx-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("source.docx");
    let output = dir.join("copy.docx");
    let bytes = include_bytes!("fixtures/imported-table-cells.docx");
    fs::write(&source, bytes).unwrap();
    let source = source.to_str().unwrap();
    let output = output.to_str().unwrap();
    let (ok, inspected) = run(json!({"command":"inspect","source":source}));
    assert!(ok, "{inspected}");
    let snapshot = inspected["result"]["snapshot"].as_str().unwrap();
    let target = inspected["result"]["body"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["type"] == "paragraph")
        .unwrap()["target"]
        .as_u64()
        .unwrap();
    let edits = json!([{"target":target,"alignment":"center"}]);
    let (ok, preview) =
        run(json!({"command":"preview","source":source,"snapshot":snapshot,"edits":edits}));
    assert!(ok, "{preview}");
    assert_eq!(preview["result"]["changes"][0]["after"], "Center");
    assert_eq!(fs::read(source).unwrap(), bytes);
    let (ok, saved) = run(
        json!({"command":"save-copy","source":source,"snapshot":snapshot,"edits":edits,"output":output}),
    );
    assert!(ok, "{saved}");
    let before = parts(bytes);
    let after = parts(&fs::read(output).unwrap());
    assert_eq!(before.len(), after.len());
    for (name, value) in &before {
        if name != "word/document.xml" {
            assert_eq!(after.get(name), Some(value), "{name}");
        }
    }
    let (ok, reread) = run(json!({"command":"inspect","source":output}));
    assert!(ok, "{reread}");
    assert_eq!(
        reread["result"]["body"][target as usize]["alignment_xml"],
        "center"
    );
    assert_eq!(fs::read(source).unwrap(), bytes);
    let (ok, _) = run(
        json!({"command":"save-copy","source":source,"snapshot":snapshot,"edits":edits,"output":source}),
    );
    assert!(!ok);
    let (ok, _) = run(
        json!({"command":"save-copy","source":source,"snapshot":snapshot,"edits":edits,"output":output}),
    );
    assert!(!ok);
    let invalid_output = dir.join("invalid.docx");
    let invalid_output = invalid_output.to_str().unwrap();
    for (snap, batch) in [
        ("stale", edits.clone()),
        (
            snapshot,
            json!([{"target":target,"alignment":"left"},{"target":target,"alignment":"right"}]),
        ),
        (snapshot, json!([{"target":999999,"alignment":"left"}])),
        (snapshot, json!([{"target":target,"alignment":"invalid"}])),
    ] {
        let (ok, error) = run(
            json!({"command":"save-copy","source":source,"snapshot":snap,"edits":batch,"output":invalid_output}),
        );
        assert!(!ok, "{error}");
        assert!(!std::path::Path::new(invalid_output).exists());
    }
    let (ok, _) =
        run(json!({"command":"preview","source":output,"snapshot":snapshot,"edits":edits}));
    assert!(!ok, "a changed document must reject the original snapshot");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unsafe_edit_rejects_whole_batch_before_creating_output() {
    let dir = std::env::temp_dir().join(format!(
        "rocx-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("unsafe.docx");
    let output = dir.join("copy.docx");
    let mut entries = parts(include_bytes!("fixtures/imported-table-cells.docx"));
    let xml = String::from_utf8(entries.remove("word/document.xml").unwrap()).unwrap();
    let xml = xml.replacen(
        "<w:body><w:p>",
        r#"<w:body><w:p><w:pPr><mc:AlternateContent><mc:Fallback><w:jc w:val="left"/></mc:Fallback></mc:AlternateContent></w:pPr>"#,
        1,
    );
    let xml = xml.replacen("<w:sectPr", r#"<w:sdt><w:sdtPr><w:tag w:val="review"/></w:sdtPr><w:sdtContent><w:p/></w:sdtContent></w:sdt><w:sectPr"#, 1);
    entries.insert("word/document.xml".into(), xml.into_bytes());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&contents).unwrap();
    }
    fs::write(&source, zip.finish().unwrap().into_inner()).unwrap();
    let source = source.to_str().unwrap();
    let (ok, inspected) = run(json!({"command":"inspect","source":source}));
    assert!(ok, "{inspected}");
    let snapshot = &inspected["result"]["snapshot"];
    let control = inspected["result"]["body"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "content-control")
        .unwrap();
    assert_eq!(control["tag"], "review");
    assert!(control.get("target").is_none());
    let (ok, result) = run(json!({
        "command":"save-copy", "source":source, "snapshot":snapshot,
        "edits":[{"target":0,"alignment":"center"}], "output":output
    }));
    assert!(!ok, "{result}");
    assert!(result["error"].as_str().unwrap().contains("ambiguous"));
    assert!(!output.exists());
    fs::remove_dir_all(dir).unwrap();
}
