mod support;
use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read, Write};

fn source(header_markup: &str) -> Vec<u8> {
    let mut section = Section::new(vec![Paragraph::new("{{X}}").into()]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("Constant header").into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::new("Constant footer").into()]).into(),
    );
    let source = block_on(Document::new(vec![section.into()]).resolve())
        .unwrap()
        .to_docx()
        .unwrap();
    let mut input = zip::ZipArchive::new(Cursor::new(source)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..input.len() {
        let mut part = input.by_index(index).unwrap();
        let mut bytes = Vec::new();
        part.read_to_end(&mut bytes).unwrap();
        output
            .start_file(part.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        if part.name() == "word/header1.xml" {
            let xml = String::from_utf8(bytes).unwrap();
            bytes = xml
                .replace("</w:hdr>", &format!("{header_markup}</w:hdr>"))
                .into_bytes();
        }
        if part.name() == "word/header1.xml" || part.name() == "word/footer1.xml" {
            bytes.extend_from_slice(b"\n");
        }
        output.write_all(&bytes).unwrap();
    }
    output.finish().unwrap().into_inner()
}

fn compile(source: &[u8], edits: &template::TemplateEdits) -> rocx::Result<Vec<u8>> {
    let empty = block_on(Document::default().resolve()).unwrap();
    application::compile_with_edits(&empty, source, edits, &Default::default())?.to_bytes()
}

#[test]
fn scoped_replacement_preserves_unmatched_story_payloads() {
    let source = source("");
    let edits = template::TemplateEdits {
        replacements: vec![("{{X}}".into(), "changed".into())],
        ..Default::default()
    };
    let result = compile(&source, &edits).unwrap();
    assert!(support::part(&result, "word/document.xml").contains("changed"));
    for path in ["word/header1.xml", "word/footer1.xml"] {
        assert_eq!(
            support::part(&source, path),
            support::part(&result, path),
            "{path}"
        );
    }
}

#[test]
fn matched_controls_and_revisions_are_written_and_missing_tags_rejected() {
    let source = source(
        r#"<w:p><w:sdt><w:sdtPr><w:tag w:val="target"/><w:text/></w:sdtPr><w:sdtContent><w:r><w:t>old</w:t></w:r></w:sdtContent></w:sdt></w:p><w:p><w:ins w:id="1" w:author="A"><w:r><w:t>inserted</w:t></w:r></w:ins></w:p>"#,
    );
    let edits = template::TemplateEdits {
        content_control_replacements: [("target".into(), "new".into())].into(),
        text_revision_action: Some(template::TextRevisionAction::Accept),
        ..Default::default()
    };
    let result = compile(&source, &edits).unwrap();
    let header = support::part(&result, "word/header1.xml");
    assert!(header.contains(">new</w:t>"));
    assert!(header.contains(">inserted</w:t>"));
    assert!(!header.contains("<w:ins"));
    assert_eq!(
        support::part(&source, "word/footer1.xml"),
        support::part(&result, "word/footer1.xml")
    );
    let missing = template::TemplateEdits {
        scope: template::EditScope::Body,
        ..edits
    };
    assert!(compile(&source, &missing).is_err());
}
