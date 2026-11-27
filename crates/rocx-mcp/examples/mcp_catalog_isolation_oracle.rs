//! Direct Rust oracle for catalog-isolation fixtures.
use std::{env, fs};

use rocx::{
    TableLayout,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, RunTextEdit, TableOperation,
        TableStructureEdit,
    },
};

fn main() {
    let [source, output, case]: [String; 3] =
        env::args().skip(1).collect::<Vec<_>>().try_into().unwrap();
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let bytes = match case.as_str() {
        "body" => {
            assert!(doc.stories().is_err());
            let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
                panic!("expected opening paragraph");
            };
            let ImportedInlineNode::Run(run) = &paragraph.inline_structure[0] else {
                panic!("expected opening run");
            };
            doc.preview_edits(EditBatch {
                run_text: &[RunTextEdit {
                    target: run.anchor.clone(),
                    text: "Edited body 😀".into(),
                }],
                ..EditBatch::default()
            })
            .unwrap()
            .to_docx()
            .unwrap()
        }
        "main" => {
            assert!(doc.stories().is_err());
            let tables = doc.part_tables(None).unwrap();
            let [outer, nested] = tables.as_slice() else {
                panic!("expected exactly one outer table and one nested table");
            };
            let fixed = |table: &rocx::editor::LocatedTable| TableStructureEdit {
                target: table.anchor.clone(),
                operation: TableOperation::Layout(Some(TableLayout::Fixed)),
            };
            let err = doc
                .edit_part_tables(&outer.part_name, &[fixed(outer), fixed(nested)])
                .unwrap_err();
            assert!(
                err.to_string()
                    .contains("overlapping physical table targets")
            );
            doc.edit_part_tables(&nested.part_name, &[fixed(nested)])
                .unwrap()
        }
        "story" => {
            assert!(doc.entry_stories().is_err());
            let header = &doc.stories().unwrap()[0];
            let tables = doc.part_tables(Some(&header.part_name)).unwrap();
            doc.edit_part_tables(
                &header.part_name,
                &[TableStructureEdit {
                    target: tables[0].anchor.clone(),
                    operation: TableOperation::Layout(Some(TableLayout::Fixed)),
                }],
            )
            .unwrap()
        }
        _ => panic!("unknown fixture case"),
    };
    fs::write(output, bytes).unwrap();
}
