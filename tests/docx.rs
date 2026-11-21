mod support;

use std::{
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use futures::executor::block_on;
use rocx::{Document, Error, Paragraph, ResolvedDocument, Section};
use support::{W, body, part, texts};

fn document(sections: Vec<Vec<&str>>) -> ResolvedDocument {
    block_on(
        Document::new(
            sections
                .into_iter()
                .map(|paragraphs| {
                    Section::new(
                        paragraphs
                            .into_iter()
                            .map(|text| Paragraph::new(text).into())
                            .collect(),
                    )
                    .into()
                })
                .collect(),
        )
        .resolve(),
    )
    .unwrap()
}

#[test]
fn package_has_the_required_parts_and_relationship() {
    let bytes = document(vec![vec!["Hello"]]).to_docx().unwrap();
    let archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert!(archive.len() >= 3);
    let types = part(&bytes, "[Content_Types].xml");
    let types = roxmltree::Document::parse(&types).unwrap();
    assert!(types.descendants().any(|n| n.attribute("PartName")==Some("/word/document.xml")
        && n.attribute("ContentType")==Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml")));
    let rels = part(&bytes, "_rels/.rels");
    let rels = roxmltree::Document::parse(&rels).unwrap();
    assert!(rels.descendants().any(|n| n.attribute("Target")==Some("word/document.xml")
        && n.attribute("Type")==Some("http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument")));
    let xml = part(&bytes, "word/document.xml");
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert!(doc.root_element().has_tag_name((W, "document")));
    assert!(
        !doc.descendants()
            .any(|n| n.has_tag_name((W, "rPr")) || n.has_tag_name((W, "pStyle")))
    );
}

#[test]
fn empty_documents_sections_and_paragraphs_are_valid() {
    for (sections, expected) in [
        (vec![], vec![]),
        (vec![vec![]], vec![]),
        (vec![vec![""]], vec![""]),
    ] {
        let bytes = document(sections).to_docx().unwrap();
        assert_eq!(texts(&bytes), expected);
        assert_eq!(body(&part(&bytes, "word/document.xml")).section_count, 1);
    }
}

#[test]
fn text_round_trips_with_whitespace_unicode_and_xml_escaping() {
    let input = [
        "  Leading  and trailing  ",
        "<tag> & \"quotes\" 'apostrophes' ]]> ",
        "café Ελληνικά 日本語 🌍",
        "\tfirst\t\tlast\t",
        "a\nb\rc\r\nd\n",
    ];
    let bytes = document(vec![input.to_vec()]).to_docx().unwrap();
    assert_eq!(
        texts(&bytes),
        input
            .iter()
            .map(|s| s.replace('\r', "\n"))
            .collect::<Vec<_>>()
    );
    let xml = part(&bytes, "word/document.xml");
    assert!(xml.contains("&lt;tag&gt; &amp;"));
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert!(
        doc.descendants()
            .filter(|n| n.has_tag_name((W, "t")))
            .all(
                |n| n.attribute(("http://www.w3.org/XML/1998/namespace", "space"))
                    == Some("preserve")
            )
    );
}

#[test]
fn sections_break_between_sections_without_a_trailing_break() {
    let bytes = document(vec![vec!["one"], vec![], vec!["two", "three"]])
        .to_docx()
        .unwrap();
    let body = body(&part(&bytes, "word/document.xml"));
    assert_eq!(body.section_count, 3);
    assert_eq!(
        body.paragraphs,
        vec![
            ("one".into(), false),
            ("".into(), true),
            ("".into(), true),
            ("two".into(), false),
            ("three".into(), false),
        ]
    );
}

#[test]
fn invalid_xml_characters_report_the_field_and_byte_offset() {
    let invalid = (0..32)
        .filter(|n| ![9, 10, 13].contains(n))
        .chain([0xfffe, 0xffff])
        .map(|n| char::from_u32(n).unwrap());
    for character in invalid {
        let error = block_on(
            Document::new(vec![
                Section::new(vec![Paragraph::new(format!("é{character}")).into()]).into(),
            ])
            .resolve(),
        )
        .unwrap_err();
        assert_eq!(error.path(), Some("sections[0].children[0].text"));
        match error {
            Error::At { source, .. } => assert!(matches!(
                *source,
                Error::InvalidXmlCharacter { byte_offset: 2, .. }
            )),
            _ => panic!("Expected a tree location"),
        }
    }
}

#[test]
fn rendering_is_repeatable_and_resolved_text_is_immutable() {
    let resolved = document(vec![vec!["one", "two"]]);
    assert_eq!(resolved.sections()[0].paragraphs()[1].text(), "two");
    assert_eq!(resolved.to_docx().unwrap(), resolved.to_docx().unwrap());
}

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cmi-docx-test-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn saving_writes_a_readable_package_and_reports_io_failures() {
    let temp = Temp::new();
    let resolved = document(vec![vec!["saved"]]);
    let output = temp.0.join("output.docx");
    resolved.save(&output).unwrap();
    assert_eq!(texts(&std::fs::read(output).unwrap()), ["saved"]);
    assert!(matches!(
        resolved.save(temp.0.join("missing/output.docx")),
        Err(Error::Io(_))
    ));
}

#[test]
fn resolution_failure_neither_creates_nor_overwrites_the_output() {
    let temp = Temp::new();
    let output = temp.0.join("output.docx");
    for exists in [false, true] {
        if exists {
            std::fs::write(&output, "existing content").unwrap();
        }
        let result = block_on(async {
            Document::new(vec![
                Section::new(vec![Paragraph::new("invalid\0").into()]).into(),
            ])
            .resolve()
            .await?
            .save(&output)
        });
        assert!(result.is_err());
        if exists {
            assert_eq!(
                std::fs::read_to_string(&output).unwrap(),
                "existing content"
            );
        } else {
            assert!(!output.exists());
        }
    }
}
