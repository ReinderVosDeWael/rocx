//! Synthetic package: a rocx-authored DOCX with hand-written WordprocessingML.
use futures::executor::block_on;
use rocx::{
    Alignment, Document, Paragraph, Section,
    editor::{BodyBlock, ImportedDocument, ParagraphAlignmentEdit},
};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const BODY: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:pPr><w:keepNext/><w:jc w:val="left"/><w:textDirection w:val="lrTb"/></w:pPr><w:bookmarkStart w:id="7" w:name="spot"/><w:r><w:rPr><w:b/></w:rPr><w:t>Hello </w:t></w:r><w:r><w:t>world</w:t></w:r><w:bookmarkEnd w:id="7"/></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>in table</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:sdt><w:sdtPr/><w:sdtContent><w:p><w:r><w:t>hidden</w:t></w:r></w:p></w:sdtContent></w:sdt>
<w:p><w:pPr><w:keepNext/><w:textDirection w:val="lrTb"/></w:pPr><w:r><w:t>Second</w:t></w:r></w:p>
<w:sectPr/></w:body></w:document>"#;

fn parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (entry.name().to_owned(), data)
        })
        .collect()
}

fn fixture() -> Vec<u8> {
    fixture_with_body(BODY)
}

fn fixture_with_body(xml: &str) -> Vec<u8> {
    let original = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new("seed").into()]).into(),
        ])
        .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap();
    let mut entries = parts(&original);
    entries.insert("word/document.xml".into(), xml.as_bytes().to_vec());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn paragraph(doc: &ImportedDocument, index: usize) -> &rocx::editor::ImportedParagraph {
    match &doc.body()[index] {
        BodyBlock::Paragraph(p) => p,
        _ => panic!("expected paragraph at {index}"),
    }
}

// Compare XML structure rather than serialization; the edited part may be reserialized.
fn tree(node: roxmltree::Node<'_, '_>, skip_jc: bool) -> String {
    if node.is_text() {
        let text = node.text().unwrap_or("");
        return if text.trim().is_empty() && !node.parent().is_some_and(|n| n.has_tag_name((W, "t")))
        {
            String::new()
        } else {
            text.to_owned()
        };
    }
    if !node.is_element() || (skip_jc && node.has_tag_name((W, "jc"))) {
        return String::new();
    }
    let mut attrs: Vec<_> = node
        .attributes()
        .map(|a| (a.namespace().unwrap_or(""), a.name(), a.value()))
        .collect();
    attrs.sort_unstable();
    format!(
        "<{:?}{attrs:?}>{}</>",
        node.tag_name(),
        node.children()
            .map(|n| tree(n, skip_jc))
            .collect::<String>()
    )
}

#[test]
fn reads_direct_body_blocks_and_paragraph_properties() {
    let doc = ImportedDocument::open(&fixture()).unwrap();
    assert!(doc.body().len() >= 4);
    let first = paragraph(&doc, 0);
    assert_eq!(first.text, "Hello world");
    assert_eq!(first.alignment, Some(Alignment::Left));
    assert_eq!(first.alignment_xml.as_deref(), Some("left"));
    assert!(first.has_opaque_content);
    assert!(matches!(&doc.body()[1], BodyBlock::Table));
    assert!(matches!(&doc.body()[2], BodyBlock::Opaque(_)));
    let second = paragraph(&doc, 3);
    assert_eq!(second.text, "Second");
    assert_eq!(second.alignment, None);
    assert_eq!(second.alignment_xml, None);
    assert!(!second.has_opaque_content);
}

#[test]
fn patch_preserves_other_markup_and_parts_and_preview_is_non_mutating() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let edit = ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    };
    let preview = doc.preview(&[edit]).unwrap();
    assert_eq!(preview.changes().len(), 1);
    assert_eq!(preview.changes()[0].body_index, 0);
    assert_eq!(preview.changes()[0].before.as_deref(), Some("left"));
    assert_eq!(preview.changes()[0].after, Alignment::Right);
    assert_eq!(paragraph(&doc, 0).alignment, Some(Alignment::Left));
    let output = preview.to_docx().unwrap();
    let original_parts = parts(&input);
    let output_parts = parts(&output);
    assert_eq!(original_parts.len(), output_parts.len());
    for (name, payload) in &original_parts {
        if name != "word/document.xml" {
            assert_eq!(output_parts.get(name), Some(payload), "part {name}");
        }
    }
    let new_xml = std::str::from_utf8(&output_parts["word/document.xml"]).unwrap();
    let old = roxmltree::Document::parse(BODY).unwrap();
    let new = roxmltree::Document::parse(new_xml).unwrap();
    let old_body = old
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let new_body = new
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    assert_eq!(tree(old_body, true), tree(new_body, true));
    let jc = new
        .descendants()
        .find(|n| n.has_tag_name((W, "jc")))
        .unwrap();
    assert_eq!(jc.attribute((W, "val")), Some("right"));
}

#[test]
fn inserts_alignment_in_schema_order() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview(&[ParagraphAlignmentEdit {
            target: paragraph(&doc, 3).anchor.clone(),
            alignment: Alignment::Center,
        }])
        .unwrap();
    assert_eq!(preview.changes()[0].body_index, 3);
    assert_eq!(preview.changes()[0].before, None);
    let output = parts(&preview.to_docx().unwrap());
    let xml = std::str::from_utf8(&output["word/document.xml"]).unwrap();
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let p = parsed
        .descendants()
        .rfind(|n| n.has_tag_name((W, "p")))
        .unwrap();
    let ppr = p.children().find(|n| n.has_tag_name((W, "pPr"))).unwrap();
    let names: Vec<_> = ppr
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect();
    assert_eq!(names, ["keepNext", "jc", "textDirection"]);
    assert_eq!(
        ppr.children()
            .find(|n| n.has_tag_name((W, "jc")))
            .unwrap()
            .attribute((W, "val")),
        Some("center")
    );
}

#[test]
fn rejects_foreign_and_duplicate_anchors_without_mutation() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let foreign = ParagraphAlignmentEdit {
        target: paragraph(&other, 0).anchor.clone(),
        alignment: Alignment::Center,
    };
    assert!(doc.preview(&[foreign]).is_err());
    let duplicate = || ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    };
    assert!(doc.preview(&[duplicate(), duplicate()]).is_err());
    assert_eq!(paragraph(&doc, 0).alignment, Some(Alignment::Left));
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

#[test]
fn no_op_preserves_all_payloads() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let empty = doc.preview(&[]).unwrap();
    assert!(empty.changes().is_empty());
    assert_eq!(parts(&empty.to_docx().unwrap()), parts(&input));
    let same = doc
        .preview(&[ParagraphAlignmentEdit {
            target: paragraph(&doc, 0).anchor.clone(),
            alignment: Alignment::Left,
        }])
        .unwrap();
    assert!(same.changes().is_empty());
    assert_eq!(parts(&same.to_docx().unwrap()), parts(&input));
}

#[test]
fn rejects_ambiguous_alignment_without_staging_other_changes() {
    let xml = BODY.replacen(
        "<w:keepNext/><w:textDirection",
        r#"<w:keepNext/><mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Fallback><w:jc w:val="left"/></mc:Fallback></mc:AlternateContent><w:textDirection"#,
        1,
    );
    let input = fixture_with_body(&xml);
    let doc = ImportedDocument::open(&input).unwrap();
    let result = doc.preview(&[
        ParagraphAlignmentEdit {
            target: paragraph(&doc, 0).anchor.clone(),
            alignment: Alignment::Right,
        },
        ParagraphAlignmentEdit {
            target: paragraph(&doc, 3).anchor.clone(),
            alignment: Alignment::Center,
        },
    ]);
    assert!(result.unwrap_err().to_string().contains("ambiguous"));
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

fn insertion_table() -> rocx::ResolvedTable {
    use rocx::{Table, TableCell, TableRow};
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("inserted A").into()]).into(),
            TableCell::new(vec![Paragraph::new("inserted B").into()]).into(),
        ])
        .into(),
    ]);
    resolve_insertion_table(table)
}

fn resolve_insertion_table(table: rocx::Table) -> rocx::ResolvedTable {
    use rocx::ResolvedBlock;
    let resolved =
        block_on(Document::new(vec![Section::new(vec![table.into()]).into()]).resolve()).unwrap();
    match &resolved.sections()[0].blocks()[0] {
        ResolvedBlock::Table(table) => table.clone(),
        _ => panic!("expected table"),
    }
}

#[test]
fn inserts_formatted_content_nested_table_and_fraction() {
    use rocx::editor::TableInsertion;
    use rocx::{InlineFraction, RunFormat, Table, TableCell, TableRow, TextRun};
    let nested = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("nested").into()]).into(),
        ])
        .into(),
    ]);
    let content = Paragraph::from_children(vec![
        TextRun::new("bold")
            .with_format(RunFormat {
                bold: Some(true),
                ..Default::default()
            })
            .into(),
        InlineFraction::new("1", "2").into(),
    ]);
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![content.into(), nested.into()]).into(),
        ])
        .into(),
    ]);
    let doc = ImportedDocument::open(&fixture()).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 3).anchor.clone(),
                table: resolve_insertion_table(table),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let table = body
        .children()
        .rfind(|n| n.has_tag_name((W, "tbl")))
        .unwrap();
    assert_eq!(
        table
            .descendants()
            .filter(|n| n.has_tag_name((W, "tbl")))
            .count(),
        2
    );
    assert!(table.descendants().any(|n| n.has_tag_name((W, "b"))));
    assert!(table.descendants().any(|n| n.tag_name().namespace()
        == Some("http://schemas.openxmlformats.org/officeDocument/2006/math")
        && n.tag_name().name() == "f"));
    assert!(
        table
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("nested"))
    );
}

#[test]
fn inserted_external_hyperlink_uses_document_relationship() {
    use rocx::editor::TableInsertion;
    use rocx::{Hyperlink, HyperlinkTarget, Table, TableCell, TableRow, TextRun};
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
    let url = "https://example.invalid/inserted";
    let link = Hyperlink::new(
        HyperlinkTarget::external(url).unwrap(),
        vec![TextRun::new("linked").into()],
    );
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::from_children(vec![link.into()]).into()]).into(),
        ])
        .into(),
    ]);
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 3).anchor.clone(),
                table: resolve_insertion_table(table),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let document =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let id = document
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")))
        .unwrap()
        .attribute((R, "id"))
        .unwrap();
    let relationships = roxmltree::Document::parse(
        std::str::from_utf8(&output["word/_rels/document.xml.rels"]).unwrap(),
    )
    .unwrap();
    let relationship = relationships
        .descendants()
        .find(|n| n.has_tag_name((REL, "Relationship")) && n.attribute("Id") == Some(id))
        .unwrap();
    assert_eq!(relationship.attribute("Target"), Some(url));
    assert_eq!(relationship.attribute("TargetMode"), Some("External"));
    assert!(
        relationship
            .attribute("Type")
            .unwrap()
            .ends_with("/hyperlink")
    );
    let original = parts(&input);
    for (name, payload) in &original {
        if name != "word/document.xml" && name != "word/_rels/document.xml.rels" {
            assert_eq!(output.get(name), Some(payload), "part {name}");
        }
    }
}

#[test]
fn inserts_exactly_after_anchor_in_mixed_batch_and_preserves_untouched_parts() {
    use rocx::editor::TableInsertion;
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_batch(
            &[ParagraphAlignmentEdit {
                target: paragraph(&doc, 3).anchor.clone(),
                alignment: Alignment::Right,
            }],
            &[TableInsertion {
                target: paragraph(&doc, 0).anchor.clone(),
                table: insertion_table(),
            }],
        )
        .unwrap();
    assert_eq!(preview.changes().len(), 1);
    assert_eq!(preview.insertions().len(), 1);
    assert_eq!(preview.insertions()[0].after_body_index, 0);
    assert_eq!(preview.insertions()[0].rows, 1);
    assert_eq!(preview.insertions()[0].columns, 2);
    assert_eq!(paragraph(&doc, 3).alignment, None);
    let original = parts(&input);
    let output = parts(&preview.to_docx().unwrap());
    for (name, payload) in &original {
        if name != "word/document.xml" {
            assert_eq!(output.get(name), Some(payload), "part {name}");
        }
    }
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let children: Vec<_> = body.children().filter(|n| n.is_element()).collect();
    assert_eq!(
        children
            .iter()
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["p", "tbl", "tbl", "sdt", "p", "sectPr"]
    );
    assert!(
        children[1]
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("inserted A"))
    );
    assert!(
        children[2]
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("in table"))
    );
    assert!(
        children[4]
            .descendants()
            .any(|n| n.has_tag_name((W, "jc")) && n.attribute((W, "val")) == Some("right"))
    );
}

#[test]
fn rejects_stale_duplicate_and_crossing_range_insertions_atomically() {
    use rocx::editor::TableInsertion;
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:bookmarkStart w:id="1" w:name="cross"/><w:r><w:t>first</w:t></w:r></w:p><w:p><w:bookmarkEnd w:id="1"/><w:r><w:t>second</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;
    let input = fixture_with_body(xml);
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let insert = |target| TableInsertion {
        target,
        table: insertion_table(),
    };
    let edit = || ParagraphAlignmentEdit {
        target: paragraph(&doc, 1).anchor.clone(),
        alignment: Alignment::Center,
    };
    assert!(
        doc.preview_batch(&[edit()], &[insert(paragraph(&other, 1).anchor.clone())])
            .is_err()
    );
    assert!(
        doc.preview_batch(
            &[edit()],
            &[
                insert(paragraph(&doc, 1).anchor.clone()),
                insert(paragraph(&doc, 1).anchor.clone())
            ]
        )
        .is_err()
    );
    assert!(
        doc.preview_batch(&[edit()], &[insert(paragraph(&doc, 0).anchor.clone())])
            .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

#[test]
fn table_after_section_terminator_uses_next_section_width() {
    use rocx::editor::TableInsertion;
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:sectPr><w:pgSz w:w="10000" w:h="15840"/><w:pgMar w:left="1000" w:right="1000" w:top="1440" w:bottom="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:pPr><w:r><w:t>section one</w:t></w:r></w:p><w:p><w:r><w:t>section two</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="14000" w:h="15840"/><w:pgMar w:left="2000" w:right="2000" w:top="1440" w:bottom="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#;
    let doc = ImportedDocument::open(&fixture_with_body(xml)).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 0).anchor.clone(),
                table: insertion_table(),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let children: Vec<_> = body.children().filter(|n| n.is_element()).collect();
    assert_eq!(children[1].tag_name().name(), "tbl");
    let widths: Vec<_> = children[1]
        .descendants()
        .filter(|n| n.has_tag_name((W, "gridCol")))
        .map(|n| n.attribute((W, "w")).unwrap())
        .collect();
    assert_eq!(widths, ["5000", "5000"]);
}
