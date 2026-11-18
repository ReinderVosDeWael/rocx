mod support;
use futures::executor::block_on;
use rv_docx_rs::*;
use std::{
    io::{Cursor, Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use support::{W, part, texts};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rv-docx-template-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("template.docx")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn resolved(children: BlockChildren) -> ResolvedDocument {
    block_on(Document::new(vec![Section::new(children).into()]).resolve()).unwrap()
}
fn empty() -> ResolvedDocument {
    block_on(Document::default().resolve()).unwrap()
}
fn cell(text: &str) -> Value<TableCell> {
    TableCell::new(vec![Paragraph::new(text).into()]).into()
}
#[test]
fn replacement_spans_runs_and_preserves_formatting_and_xml_special_text() {
    let temp = Temp::new();
    let first = TextRun {
        format: RunFormat {
            bold: Some(true),
            ..RunFormat::default()
        }
        .into(),
        ..TextRun::new("Hello {{NA")
    };
    let second = TextRun {
        format: RunFormat {
            italic: Some(true),
            ..RunFormat::default()
        }
        .into(),
        ..TextRun::new("ME}}! {{NAME}}")
    };
    resolved(vec![
        Paragraph::from_children(vec![first.into(), second.into()]).into(),
    ])
    .save(temp.path())
    .unwrap();
    let mut template = DocumentTemplate::new(temp.path());
    template.edits.replacements = vec![("{{NAME}}".into(), "A<&>\nB\t".into())];
    let bytes = empty().to_docx_with_template(&template).unwrap();
    assert_eq!(texts(&bytes), ["Hello A<&>\nB\t! A<&>\nB\t"]);
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    let runs: Vec<_> = parsed
        .descendants()
        .filter(|n| n.has_tag_name((W, "r")))
        .collect();
    assert!(runs[0].descendants().any(|n| n.has_tag_name((W, "b"))));
    assert!(runs[1].descendants().any(|n| n.has_tag_name((W, "i"))));
}
#[test]
fn replacement_order_and_empty_replacements_match_python_dictionary_semantics() {
    let temp = Temp::new();
    resolved(vec![Paragraph::new("x erase x").into()])
        .save(temp.path())
        .unwrap();
    let mut template = DocumentTemplate::new(temp.path());
    template.edits.replacements = vec![
        ("x".into(), "y".into()),
        ("y".into(), "z".into()),
        ("erase".into(), "".into()),
    ];
    assert_eq!(
        texts(&empty().to_docx_with_template(&template).unwrap()),
        ["z  z"]
    );
    template.edits.replacements = vec![("".into(), "bad".into())];
    assert!(empty().to_docx_with_template(&template).is_err());
}
#[test]
fn replacement_covers_nested_tables_headers_and_footers_but_not_new_content() {
    let temp = Temp::new();
    let table = Table::new(vec![TableRow::new(vec![cell("{{X}}")]).into()]);
    let mut section = Section::new(vec![table.into()]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("Header {{X}}").into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::new("Footer {{X}}").into()]).into(),
    );
    block_on(Document::new(vec![section.into()]).resolve())
        .unwrap()
        .save(temp.path())
        .unwrap();
    let mut template = DocumentTemplate::new(temp.path());
    template.edits.replacements = vec![("{{X}}".into(), "changed".into())];
    let bytes = resolved(vec![Paragraph::new("New {{X}}").into()])
        .to_docx_with_template(&template)
        .unwrap();
    assert_eq!(texts(&bytes), ["New {{X}}"]);
    assert!(part(&bytes, "word/document.xml").contains("changed"));
    assert!(part(&bytes, "word/header1.xml").contains("Header changed"));
    assert!(part(&bytes, "word/footer1.xml").contains("Footer changed"));
}
#[test]
fn insertion_before_original_paragraphs_handles_tables_and_multiple_sections() {
    let temp = Temp::new();
    resolved(vec![
        Paragraph::new("first").into(),
        Paragraph::new("second").into(),
        Paragraph::new("third").into(),
    ])
    .save(temp.path())
    .unwrap();
    for (index, expected) in [
        (0, vec!["A", "B", "first", "second", "third"]),
        (1, vec!["first", "A", "B", "second", "third"]),
        (99, vec!["first", "second", "third", "A", "B"]),
    ] {
        let first = Section::new(vec![
            Paragraph::new("A").into(),
            Table::new(vec![TableRow::new(vec![cell("table")]).into()]).into(),
        ]);
        let second = Section::new(vec![Paragraph::new("B").into()]);
        let declaration =
            block_on(Document::new(vec![first.into(), second.into()]).resolve()).unwrap();
        let mut template = DocumentTemplate::new(temp.path());
        template.edits.paragraph_index = Some(index);
        let bytes = declaration.to_docx_with_template(&template).unwrap();
        assert_eq!(
            texts(&bytes)
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn template_headers_are_extended_and_existing_comments_and_images_survive() {
    let temp = Temp::new();
    let mut first = Section::new(vec![
        Paragraph::from_children(vec![
            TextRun {
                comment_text: Some("original comment".into()),
                ..TextRun::new("original")
            }
            .into(),
            ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec()).into(),
        ])
        .into(),
    ]);
    first.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("original header").into()]).into(),
    );
    block_on(Document::new(vec![first.into()]).resolve())
        .unwrap()
        .save(temp.path())
        .unwrap();
    let mut new = Section::new(vec![
        Paragraph::from_children(vec![
            TextRun {
                comment_text: Some("new comment".into()),
                ..TextRun::new("new")
            }
            .into(),
            ImageRun::new(include_bytes!("fixtures/images/sample.jpg").to_vec()).into(),
        ])
        .into(),
    ]);
    new.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("new header").into()]).into(),
    );
    let bytes = block_on(Document::new(vec![new.into()]).resolve())
        .unwrap()
        .to_docx_with_template(&DocumentTemplate::new(temp.path()))
        .unwrap();
    let header = part(&bytes, "word/header1.xml");
    assert!(header.contains("original header"));
    assert!(header.contains("new header"));
    let comments = part(&bytes, "word/comments.xml");
    let parsed = roxmltree::Document::parse(&comments).unwrap();
    let ids: Vec<_> = parsed
        .descendants()
        .filter(|n| n.has_tag_name((W, "comment")))
        .map(|n| n.attribute((W, "id")).unwrap())
        .collect();
    assert_eq!(ids, ["0", "1"]);
    let archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(
        archive
            .file_names()
            .filter(|p| p.starts_with("word/media/"))
            .count(),
        2
    );
}
#[test]
fn unrelated_parts_and_template_metadata_are_retained() {
    let temp = Temp::new();
    let mut source = Document::new(vec![]);
    source.title = Some("original title".into());
    source.subject = Some("retain subject".into());
    let bytes = block_on(source.resolve()).unwrap().to_docx().unwrap();
    let mut reader = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..reader.len() {
        let mut entry = reader.by_index(i).unwrap();
        writer
            .start_file(entry.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if entry.name() == "[Content_Types].xml" {
            let xml=String::from_utf8(bytes).unwrap().replace("</Types>","<Override PartName=\"/custom/opaque.bin\" ContentType=\"application/octet-stream\"/></Types>");
            writer.write_all(xml.as_bytes()).unwrap();
        } else {
            writer.write_all(&bytes).unwrap();
        }
    }
    writer
        .start_file(
            "custom/opaque.bin",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer.write_all(b"opaque bytes\0\xff").unwrap();
    std::fs::write(temp.path(), writer.finish().unwrap().into_inner()).unwrap();
    let mut new = Document::new(vec![]);
    new.title = Some("new title".into());
    let result = block_on(new.resolve())
        .unwrap()
        .to_docx_with_template(&DocumentTemplate::new(temp.path()))
        .unwrap();
    let core = part(&result, "docProps/core.xml");
    assert!(core.contains("new title"));
    assert!(core.contains("retain subject"));
    let mut reader = zip::ZipArchive::new(Cursor::new(&result)).unwrap();
    let mut opaque = Vec::new();
    reader
        .by_name("custom/opaque.bin")
        .unwrap()
        .read_to_end(&mut opaque)
        .unwrap();
    assert_eq!(opaque, b"opaque bytes\0\xff");
}
#[test]
fn template_save_can_replace_the_input_and_invalid_templates_leave_outputs_untouched() {
    let temp = Temp::new();
    resolved(vec![Paragraph::new("original").into()])
        .save(temp.path())
        .unwrap();
    resolved(vec![Paragraph::new("appended").into()])
        .save_with_template(temp.path(), &DocumentTemplate::new(temp.path()))
        .unwrap();
    assert_eq!(
        texts(&std::fs::read(temp.path()).unwrap()),
        ["original", "appended"]
    );
    let invalid = temp.0.join("invalid.docx");
    std::fs::write(&invalid, "not a zip").unwrap();
    assert!(
        empty()
            .save_with_template(temp.path(), &DocumentTemplate::new(invalid))
            .is_err()
    );
    assert_eq!(
        texts(&std::fs::read(temp.path()).unwrap()),
        ["original", "appended"]
    );
}

#[test]
fn minimal_opc_templates_receive_missing_optional_word_parts() {
    let temp = Temp::new();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, xml) in [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/document.xml",
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>minimal</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#,
        ),
    ] {
        writer
            .start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    std::fs::write(temp.path(), writer.finish().unwrap().into_inner()).unwrap();
    let bytes = resolved(vec![Paragraph::new("new").into()])
        .to_docx_with_template(&DocumentTemplate::new(temp.path()))
        .unwrap();
    assert_eq!(texts(&bytes), ["minimal", "new"]);
    for path in [
        "word/styles.xml",
        "word/settings.xml",
        "word/numbering.xml",
        "docProps/core.xml",
    ] {
        assert!(!part(&bytes, path).is_empty());
    }
}

#[test]
fn utf16_template_parts_are_read_and_written_as_valid_utf8() {
    for little_endian in [true, false] {
        let temp = Temp::new();
        let original = resolved(vec![Paragraph::new("Unicode 日本語 {{X}}").into()])
            .to_docx()
            .unwrap();
        let mut reader = zip::ZipArchive::new(Cursor::new(&original)).unwrap();
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for i in 0..reader.len() {
            let mut entry = reader.by_index(i).unwrap();
            let name = entry.name().to_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if name == "word/document.xml" {
                let xml = String::from_utf8(bytes)
                    .unwrap()
                    .replace("encoding=\"UTF-8\"", "encoding=\"UTF-16\"");
                bytes = if little_endian {
                    vec![0xff, 0xfe]
                } else {
                    vec![0xfe, 0xff]
                };
                for u in xml.encode_utf16() {
                    bytes.extend_from_slice(&if little_endian {
                        u.to_le_bytes()
                    } else {
                        u.to_be_bytes()
                    });
                }
            }
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        std::fs::write(temp.path(), writer.finish().unwrap().into_inner()).unwrap();
        let mut template = DocumentTemplate::new(temp.path());
        template.edits.replacements = vec![("{{X}}".into(), "changed".into())];
        assert_eq!(
            texts(&empty().to_docx_with_template(&template).unwrap()),
            ["Unicode 日本語 changed"]
        );
    }
}

#[test]
fn edits_preserve_page_breaks_and_reject_structured_boundary_crossings() {
    let temp = Temp::new();
    let mut package =
        zip::ZipArchive::new(Cursor::new(include_bytes!("../assets/default.docx"))).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..package.len() {
        let mut entry = package.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name() == "word/document.xml" {
            let xml = String::from_utf8(data).unwrap();
            data=xml.replace("<w:sectPr",r#"<w:p><w:r><w:t>replace</w:t><w:br w:type="page"/><w:t>before</w:t></w:r><w:hyperlink w:anchor="local"><w:r><w:t>inside</w:t></w:r></w:hyperlink></w:p><w:sectPr"#).into_bytes();
        }
        writer
            .start_file(entry.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&data).unwrap();
    }
    std::fs::write(temp.path(), writer.finish().unwrap().into_inner()).unwrap();
    let mut template = DocumentTemplate::new(temp.path());
    template.edits.replacements = vec![("replace".into(), "changed".into())];
    let output = empty().to_docx_with_template(&template).unwrap();
    let xml = part(&output, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert!(
        parsed
            .descendants()
            .any(|n| n.has_tag_name((W, "br")) && n.attribute((W, "type")) == Some("page"))
    );
    assert!(
        parsed.descendants().any(
            |n| n.has_tag_name((W, "hyperlink")) && n.attribute((W, "anchor")) == Some("local")
        )
    );
    template.edits.replacements = vec![("beforeinside".into(), "unsafe".into())];
    assert_eq!(
        empty().to_docx_with_template(&template).unwrap_err().rule(),
        Some(Rule::UnsupportedEdit)
    );
    template.edits.replacements = vec![("inside".into(), "safe".into())];
    assert!(
        part(
            &empty().to_docx_with_template(&template).unwrap(),
            "word/document.xml"
        )
        .contains("safe")
    );
}
