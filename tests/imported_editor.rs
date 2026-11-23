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
    package_bytes(entries)
}

fn package_bytes(entries: BTreeMap<String, Vec<u8>>) -> Vec<u8> {
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
    let BodyBlock::Table(table) = &doc.body()[1] else {
        panic!("expected table")
    };
    let [rocx::editor::ImportedTableChild::Row(row)] = table.children.as_slice() else {
        panic!("expected row")
    };
    let [rocx::editor::ImportedRowChild::Cell(cell)] = row.children.as_slice() else {
        panic!("expected cell")
    };
    let [rocx::editor::ImportedCellBlock::Paragraph(items)] = cell.blocks.as_slice() else {
        panic!("expected paragraph")
    };
    assert_eq!(
        items,
        &[rocx::editor::ImportedInline::Text("in table".into())]
    );
    assert!(matches!(&doc.body()[2], BodyBlock::Opaque(_)));
    let second = paragraph(&doc, 3);
    assert_eq!(second.text, "Second");
    assert_eq!(second.alignment, None);
    assert_eq!(second.alignment_xml, None);
    assert!(!second.has_opaque_content);
}

#[test]
fn table_inspection_preserves_unknown_children_in_order() {
    use rocx::editor::{ImportedCellBlock, ImportedRowChild, ImportedTableChild};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:tbl><w:tblPr/><w:customXml/><w:tr><w:trPr/><w:sdt/><w:tc><w:tcPr/><w:p><w:r><w:t>x</w:t><w:drawing/></w:r></w:p><w:tbl/><w:sdt/></w:tc></w:tr></w:tbl></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let BodyBlock::Table(table) = &doc.body()[0] else {
        panic!()
    };
    assert!(
        matches!(&table.children[0], ImportedTableChild::Opaque(name) if name == "w:customXml")
    );
    let ImportedTableChild::Row(row) = &table.children[1] else {
        panic!()
    };
    assert!(matches!(&row.children[0], ImportedRowChild::Opaque(name) if name == "w:sdt"));
    let ImportedRowChild::Cell(cell) = &row.children[1] else {
        panic!()
    };
    assert!(
        matches!(&cell.blocks[0], ImportedCellBlock::Paragraph(items) if matches!(&items[1], rocx::editor::ImportedInline::Opaque(name) if name == "w:drawing"))
    );
    assert!(matches!(&cell.blocks[1], ImportedCellBlock::Opaque(name) if name == "w:tbl"));
    assert!(matches!(&cell.blocks[2], ImportedCellBlock::Opaque(name) if name == "w:sdt"));
}

#[test]
fn inspects_ordered_inline_items_without_changing_the_text_projection() {
    use rocx::BreakType;
    use rocx::editor::ImportedInline::{Break, Opaque, Tab, Text};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t xml:space="preserve">a </w:t><w:tab/><w:br/><w:br w:type="page"/><w:cr/><w:br w:type="column"/><w:drawing/></w:r><w:hyperlink w:anchor="bookmark"><w:r><w:t>b</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let first = paragraph(&doc, 0);
    assert_eq!(first.text, "a \t\n\nb");
    assert!(first.has_opaque_content);
    assert_eq!(
        first.inline_items,
        [
            Text("a ".into()),
            Tab,
            Break(BreakType::TextWrapping),
            Break(BreakType::Page),
            Break(BreakType::Line),
            Break(BreakType::Column),
            Opaque("w:drawing".into()),
            Text("b".into()),
        ]
    );
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
fn inspects_explicit_and_implicit_sections_and_refuses_ambiguity() {
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:sectPr><w:type w:val="continuous"/></w:sectPr></w:pPr></w:p><w:p><w:pPr><w:sectPr><w:type w:val="unknown"/></w:sectPr></w:pPr></w:p><w:tbl/></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let sections = doc.sections().unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|s| (s.body_start, s.body_end, s.section_type_xml.as_deref()))
            .collect::<Vec<_>>(),
        [
            (0, 1, Some("continuous")),
            (1, 2, Some("unknown")),
            (2, 3, None),
        ]
    );
    assert_eq!(
        sections[0].section_type,
        Some(rocx::SectionType::Continuous)
    );
    assert_eq!(sections[1].section_type, None);
    let duplicate = xml.replace(
        r#"<w:type w:val="continuous"/>"#,
        r#"<w:type w:val="continuous"/><w:type w:val="nextPage"/>"#,
    );
    let doc = ImportedDocument::open(&fixture_with_body(&duplicate)).unwrap();
    assert!(doc.sections().is_err());
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

// These packages are synthetic; they do not claim Word-authored provenance.
fn header_fixture(content: &str, hyperlink: bool) -> Vec<u8> {
    let mut entries = parts(&fixture());
    let body = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>
<w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="headerTest"/></w:sectPr></w:pPr></w:p>
<w:p><w:pPr><w:sectPr/></w:pPr></w:p>
<w:p/><w:sectPr/></w:body></w:document>"#;
    entries.insert("word/document.xml".into(), body.as_bytes().to_vec());
    let rel_name = "word/_rels/document.xml.rels";
    let rels = String::from_utf8(entries[rel_name].clone()).unwrap();
    let rels = rels.replace("</Relationships>", r#"<Relationship Id="headerTest" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="headerTest.xml"/></Relationships>"#);
    entries.insert(rel_name.into(), rels.into_bytes());
    let types_name = "[Content_Types].xml";
    let types = String::from_utf8(entries[types_name].clone()).unwrap();
    let types = types.replace("</Types>", r#"<Override PartName="/word/headerTest.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#);
    entries.insert(types_name.into(), types.into_bytes());
    let header = format!(
        r#"<w:hdr xmlns:w="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{content}</w:hdr>"#
    );
    entries.insert("word/headerTest.xml".into(), header.into_bytes());
    if hyperlink {
        entries.insert("word/_rels/headerTest.xml.rels".into(), br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="link1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/header" TargetMode="External"/></Relationships>"#.to_vec());
    }
    package_bytes(entries)
}

fn header_image() -> rocx::ResolvedImage {
    use rocx::{Component, ImageRun};
    block_on(Component::resolve(ImageRun::new(
        include_bytes!("fixtures/images/sample.png").to_vec(),
    )))
    .unwrap()
    .unwrap()
    .image()
    .unwrap()
    .clone()
}

fn assert_header_image_relationship(output: &BTreeMap<String, Vec<u8>>, part: &str) {
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
    const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let part = part.trim_start_matches('/');
    let (dir, filename) = part.rsplit_once('/').unwrap();
    let rel_name = format!("{dir}/_rels/{filename}.rels");
    let header = roxmltree::Document::parse(std::str::from_utf8(&output[part]).unwrap()).unwrap();
    let id = header
        .descendants()
        .find(|n| n.has_tag_name((A, "blip")))
        .unwrap()
        .attribute((R, "embed"))
        .unwrap();
    let rels =
        roxmltree::Document::parse(std::str::from_utf8(&output[&rel_name]).unwrap()).unwrap();
    let rel = rels
        .descendants()
        .find(|n| n.has_tag_name((REL, "Relationship")) && n.attribute("Id") == Some(id))
        .unwrap();
    assert!(rel.attribute("Type").unwrap().ends_with("/image"));
    let target = rel.attribute("Target").unwrap();
    let image_part = rocx::opc::PartName::resolve(part, target).unwrap();
    assert_eq!(
        output[image_part.zip_name()],
        include_bytes!("fixtures/images/sample.png")
    );
}

#[test]
fn shared_header_edit_reports_all_consumers_and_owns_image_relationship() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture("<w:p><w:r><w:t>original</w:t></w:r></w:p>", false);
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(
        headers.iter().map(|h| h.section_index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        headers.iter().map(|h| h.inherited).collect::<Vec<_>>(),
        [false, true, true]
    );
    assert!(headers.iter().all(|h| h.part_name == headers[0].part_name));
    assert!(matches!(&headers[1].blocks[..],
        [rocx::editor::ImportedHeaderBlock::Paragraph(items)]
        if matches!(&items[..], [rocx::editor::ImportedInline::Text(text)] if text == "original")));
    let edit = HeaderImageEdit {
        target: headers[1].anchor.clone(),
        mode: HeaderEditMode::Shared,
        image: header_image(),
    };
    let preview = doc.preview_header_image(&edit).unwrap();
    let change = preview.change();
    assert_eq!(change.source_part, headers[0].part_name);
    assert_eq!(change.output_part, change.source_part);
    assert_eq!(change.affected_sections, [0, 1, 2]);
    assert_eq!(change.restored_section, None);
    assert_eq!(change.mode, HeaderEditMode::Shared);
    let output = parts(&preview.to_docx().unwrap());
    assert_header_image_relationship(&output, &change.output_part);
    let original = parts(&input);
    for (name, bytes) in &original {
        if name != "word/headerTest.xml" && name != "[Content_Types].xml" {
            assert_eq!(output.get(name), Some(bytes), "{name}");
        }
    }
}

#[test]
fn header_story_preserves_order_and_opaque_children() {
    use rocx::editor::{ImportedHeaderBlock, ImportedInline};
    let input = header_fixture(
        "<w:p><w:r><w:t>A</w:t><w:tab/><w:t>B</w:t></w:r></w:p><w:tbl/><w:p><w:r><w:drawing/></w:r></w:p>",
        false,
    );
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    assert!(matches!(&headers[0].blocks[..], [
        ImportedHeaderBlock::Paragraph(first),
        ImportedHeaderBlock::Opaque(table),
        ImportedHeaderBlock::Paragraph(last),
    ] if matches!(&first[..], [ImportedInline::Text(a), ImportedInline::Tab, ImportedInline::Text(b)] if a == "A" && b == "B")
        && table == "w:tbl"
        && matches!(&last[..], [ImportedInline::Opaque(name)] if name == "w:drawing")));
}

#[test]
fn local_header_edit_restores_later_inheritors_and_preserves_link() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture(
        r#"<w:p><w:hyperlink r:id="link1"><w:r><w:t>linked</w:t></w:r></w:hyperlink></w:p>"#,
        true,
    );
    for section in [0, 1] {
        let doc = ImportedDocument::open(&input).unwrap();
        let headers = doc.default_headers().unwrap();
        let edit = HeaderImageEdit {
            target: headers[section].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        };
        let preview = doc.preview_header_image(&edit).unwrap();
        let change = preview.change();
        assert_eq!(change.source_part, headers[section].part_name);
        assert_ne!(change.output_part, change.source_part);
        assert_eq!(change.affected_sections, [section]);
        assert_eq!(change.restored_section, Some(section + 1));
        assert_eq!(change.mode, HeaderEditMode::SectionLocal);
        let bytes = preview.to_docx().unwrap();
        let output = parts(&bytes);
        assert_header_image_relationship(&output, &change.output_part);
        assert_eq!(
            output["word/headerTest.xml"],
            parts(&input)["word/headerTest.xml"]
        );
        assert_eq!(
            output["word/_rels/headerTest.xml.rels"],
            parts(&input)["word/_rels/headerTest.xml.rels"]
        );
        let reopened = ImportedDocument::open(&bytes).unwrap();
        let after = reopened.default_headers().unwrap();
        for (index, header) in after.iter().enumerate() {
            assert_eq!(
                header.part_name,
                if index == section {
                    &change.output_part
                } else {
                    &change.source_part
                }
                .as_str(),
                "section {index}"
            );
        }
        let part = change.output_part.trim_start_matches('/');
        let (dir, filename) = part.rsplit_once('/').unwrap();
        let rels = format!("{dir}/_rels/{filename}.rels");
        let header = std::str::from_utf8(&output[part]).unwrap();
        let header = roxmltree::Document::parse(header).unwrap();
        let id = header
            .descendants()
            .find(|n| n.has_tag_name((W, "hyperlink")))
            .unwrap()
            .attribute((
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                "id",
            ))
            .unwrap();
        let rels =
            roxmltree::Document::parse(std::str::from_utf8(&output[&rels]).unwrap()).unwrap();
        let link = rels
            .descendants()
            .find(|n| n.attribute("Id") == Some(id))
            .unwrap();
        assert_eq!(
            link.attribute("Target"),
            Some("https://example.invalid/header")
        );
        assert_eq!(link.attribute("TargetMode"), Some("External"));
    }
}

#[test]
fn local_header_rejects_identity_bearing_and_unknown_content_and_stale_anchors() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    for content in [
        r#"<w:p><w:r><w:drawing/></w:r></w:p>"#,
        r#"<w:p><w:bookmarkStart w:id="1" w:name="identity"/></w:p>"#,
        r#"<w:p><w:customXml w:element="unknown"/></w:p>"#,
    ] {
        let input = header_fixture(content, false);
        let doc = ImportedDocument::open(&input).unwrap();
        let headers = doc.default_headers().unwrap();
        let edit = HeaderImageEdit {
            target: headers[1].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        };
        assert!(doc.preview_header_image(&edit).is_err(), "{content}");
    }
    let input = header_fixture("<w:p/>", false);
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let edit = HeaderImageEdit {
        target: other.default_headers().unwrap()[0].anchor.clone(),
        mode: HeaderEditMode::Shared,
        image: header_image(),
    };
    assert!(doc.preview_header_image(&edit).is_err());
}

#[test]
fn local_header_forks_beside_relocated_source_with_prefixed_relationships() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let original = header_fixture(
        r#"<w:p><w:hyperlink r:id="link1"><w:r><w:t>linked</w:t></w:r></w:hyperlink></w:p>"#,
        true,
    );
    let mut entries = parts(&original);
    let header = entries.remove("word/headerTest.xml").unwrap();
    entries.insert("content/headers/source.xml".into(), header);
    let links = entries.remove("word/_rels/headerTest.xml.rels").unwrap();
    entries.insert("content/headers/_rels/source.xml.rels".into(), links);
    let replace = |xml: &mut Vec<u8>, substitutions: &[(&str, &str)]| {
        let mut text = String::from_utf8(std::mem::take(xml)).unwrap();
        for (from, to) in substitutions {
            text = text.replace(from, to);
        }
        *xml = text.into_bytes();
    };
    for rels in [
        "word/_rels/document.xml.rels",
        "content/headers/_rels/source.xml.rels",
    ] {
        replace(
            entries.get_mut(rels).unwrap(),
            &[
                ("<Relationships xmlns=", "<p:Relationships xmlns:p="),
                ("</Relationships>", "</p:Relationships>"),
                ("<Relationship ", "<p:Relationship "),
                (
                    r#"Target="headerTest.xml""#,
                    r#"Target="../content/headers/source.xml""#,
                ),
            ],
        );
    }
    replace(
        entries.get_mut("[Content_Types].xml").unwrap(),
        &[
            ("<Types xmlns=", "<p:Types xmlns:p="),
            ("</Types>", "</p:Types>"),
            ("<Default ", "<p:Default "),
            ("<Override ", "<p:Override "),
            ("/word/headerTest.xml", "/content/headers/source.xml"),
        ],
    );
    let input = package_bytes(entries);
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_header_image(&HeaderImageEdit {
            target: doc.default_headers().unwrap()[1].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        })
        .unwrap();
    assert_eq!(preview.change().source_part, "content/headers/source.xml");
    assert!(preview.change().output_part.starts_with("content/headers/"));
    let output = parts(&preview.to_docx().unwrap());
    assert_header_image_relationship(&output, &preview.change().output_part);
    assert_eq!(
        output["content/headers/_rels/source.xml.rels"],
        parts(&input)["content/headers/_rels/source.xml.rels"]
    );
}

#[test]
fn shared_edit_reserves_drawing_ids_from_bomless_utf16_parts() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture(
        r#"<w:p xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><w:r><w:drawing><wp:inline><wp:docPr id="91" name="old"/></wp:inline></w:drawing></w:r></w:p>"#,
        false,
    );
    let mut entries = parts(&input);
    let header = entries.get_mut("word/headerTest.xml").unwrap();
    *header = std::str::from_utf8(header)
        .unwrap()
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    let preview = doc
        .preview_header_image(&HeaderImageEdit {
            target: doc.default_headers().unwrap()[0].anchor.clone(),
            mode: HeaderEditMode::Shared,
            image: header_image(),
        })
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let header =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/headerTest.xml"]).unwrap())
            .unwrap();
    let ids: Vec<_> = header
        .descendants()
        .filter(|n| n.tag_name().name() == "docPr")
        .map(|n| n.attribute("id").unwrap())
        .collect();
    assert_eq!(ids, ["91", "92"]);
}

#[test]
fn shared_edit_counts_aliases_on_first_page_headers() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let mut entries = parts(&header_fixture("<w:p/>", false));
    let mut rewrite = |path: &str, old: &str, new: &str| {
        let xml = entries.get_mut(path).unwrap();
        let source = std::str::from_utf8(xml).unwrap();
        assert!(source.contains(old));
        *xml = source.replace(old, new).into_bytes();
    };
    rewrite(
        "word/document.xml",
        "<w:sectPr/></w:pPr>",
        r#"<w:sectPr><w:headerReference w:type="default" r:id="otherHeader"/></w:sectPr></w:pPr>"#,
    );
    rewrite(
        "word/document.xml",
        "<w:sectPr/></w:body>",
        r#"<w:sectPr><w:headerReference w:type="first" r:id="aliasForHeaderTest"/><w:titlePg/></w:sectPr></w:body>"#,
    );
    rewrite(
        "word/_rels/document.xml.rels",
        "</Relationships>",
        r#"<Relationship Id="otherHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="other.xml"/><Relationship Id="aliasForHeaderTest" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="headerTest.xml"/></Relationships>"#,
    );
    rewrite(
        "[Content_Types].xml",
        "</Types>",
        r#"<Override PartName="/word/other.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#,
    );
    entries.insert(
        "word/other.xml".into(),
        entries["word/headerTest.xml"].clone(),
    );
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    let headers = doc.default_headers().unwrap();
    assert_eq!(headers[1].part_name, headers[2].part_name);
    assert_ne!(headers[0].part_name, headers[1].part_name);
    let preview = doc
        .preview_header_image(&HeaderImageEdit {
            target: headers[0].anchor.clone(),
            mode: HeaderEditMode::Shared,
            image: header_image(),
        })
        .unwrap();
    assert_eq!(preview.change().affected_sections, [0, 2]);
}
