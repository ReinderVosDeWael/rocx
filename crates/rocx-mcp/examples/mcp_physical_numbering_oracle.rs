//! Native physical numbering transactions against original entry, story, and cell coordinates.
use rocx::editor::{
    BodyBlock, EditBatch, ImportedDocument, NumberingEdit as N, ResourceEdit, TableOperation,
    TableStructureEdit,
};
use rocx::{NumberingLevel, NumberingReference};

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    let assign = match action.as_str() {
        "assign" => true,
        "clear" => false,
        _ => panic!("expected assign or clear"),
    };
    let table = |i: usize| {
        let BodyBlock::Table(table) = &doc.body()[i] else {
            panic!("expected fixture table")
        };
        &table.anchor
    };
    let mut targets =
        vec![doc.cell_paragraph_anchor(table(if assign { 2 } else { 1 }), 0, 0, 2)?];
    for (part, id) in [
        ("word/stories/selected.xml", None),
        ("word/footer1.xml", None),
        ("word/notes/footnotes.xml", Some("007")),
        ("word/notes/endnotes.xml", Some("007")),
        ("word/comments.xml", Some("0")),
    ] {
        targets.push(doc.story_paragraph_anchor(part, 0, id)?);
    }
    let reference = assign.then_some(NumberingReference { id: 90, level: 0 });
    let mut resources: Vec<_> = targets
        .into_iter()
        .map(|target| {
            ResourceEdit::Numbering(N::StoryParagraph {
                target,
                value: reference.clone(),
            })
        })
        .collect();
    let mut structural = vec![];
    let definitions = if assign {
        structural.push(TableStructureEdit {
            target: table(1).clone(),
            operation: TableOperation::RemoveTable,
        });
        [
            N::AddInstance {
                num_id: 90,
                abstract_id: 80,
            },
            N::AddAbstract {
                abstract_id: 80,
                levels: vec![NumberingLevel {
                    level: 0,
                    start: 1,
                    format: "decimal".into(),
                    text: "%1.".into(),
                    ..Default::default()
                }],
            },
        ]
    } else {
        [
            N::RemoveAbstract { abstract_id: 80 },
            N::RemoveInstance { num_id: 90 },
        ]
    };
    resources.extend(definitions.into_iter().map(ResourceEdit::Numbering));
    let preview = doc.preview_edits(EditBatch {
        resources: &resources,
        structural: &structural,
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?).unwrap();
    Ok(())
}
