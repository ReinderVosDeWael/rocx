//! Native reference packages for `javascript/tests/parity/editor.mjs`.
use rocx::{
    Alignment, Component, Hyperlink, HyperlinkTarget, ImageRun, InlineFraction, Paragraph,
    RunFormat, Table, TableCell, TableCellProperties, TableProperties, TableRow, TextRun,
    editor::{
        BodyBlock, HeaderEditMode, HeaderImageEdit, ImportedDocument, ParagraphAlignmentEdit,
        ParagraphAnchor, TableInsertion,
    },
};
use std::{
    env, fs,
    future::Future,
    path::PathBuf,
    task::{Context, Poll, Waker},
};

// All oracle declarations use immediate values; no external executor is needed.
fn immediate<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("immediate fixture yielded"),
    }
}

fn anchor(doc: &ImportedDocument, index: usize) -> ParagraphAnchor {
    match &doc.body()[index] {
        BodyBlock::Paragraph(p) => p.anchor.clone(),
        _ => panic!("expected paragraph"),
    }
}

fn pair(a: &str, b: &str) -> Table {
    Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new(a).into()]).into(),
            TableCell::new(vec![Paragraph::new(b).into()]).into(),
        ])
        .into(),
    ])
}

fn main() {
    let directory = PathBuf::from(env::args_os().nth(1).expect("fixture directory"));
    let doc = ImportedDocument::open(&fs::read(directory.join("base.docx")).unwrap()).unwrap();
    let headers = doc.default_headers().unwrap();
    let image = immediate(Component::resolve(ImageRun::new(
        fs::read(directory.join("image.png")).unwrap(),
    )))
    .unwrap()
    .unwrap()
    .image()
    .unwrap()
    .clone();

    let rich = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("bold")
                        .with_format(RunFormat {
                            bold: Some(true),
                            ..Default::default()
                        })
                        .into(),
                    InlineFraction::new("1", "2").into(),
                    Hyperlink::new(
                        HyperlinkTarget::external("https://example.invalid/inserted").unwrap(),
                        vec![TextRun::new("linked").into()],
                    )
                    .into(),
                ])
                .into(),
                pair("nested A", "nested B").into(),
            ])
            .with_properties(TableCellProperties::default().with_grid_span(2))
            .into(),
            TableCell::new(vec![Paragraph::new("tail").into()]).into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties::default().with_column_widths(vec![1200, 2400, 3600]));
    let alignments = [
        ParagraphAlignmentEdit {
            target: anchor(&doc, 0),
            alignment: Alignment::Center,
        },
        ParagraphAlignmentEdit {
            target: anchor(&doc, 1),
            alignment: Alignment::Right,
        },
    ];
    let insertions = [
        TableInsertion {
            target: anchor(&doc, 0),
            table: immediate(Component::resolve(rich)).unwrap().unwrap(),
        },
        TableInsertion {
            target: anchor(&doc, 3),
            table: immediate(Component::resolve(pair("next A", "next B")))
                .unwrap()
                .unwrap(),
        },
    ];
    let edit = |index: usize, mode| HeaderImageEdit {
        target: headers[index].anchor.clone(),
        mode,
        image: image.clone(),
    };
    let local = [
        edit(0, HeaderEditMode::SectionLocal),
        edit(1, HeaderEditMode::SectionLocal),
    ];
    let shared = edit(1, HeaderEditMode::Shared);
    let write = |name, bytes: Vec<u8>| fs::write(directory.join(name), bytes).unwrap();
    let ranged =
        ImportedDocument::open(&fs::read(directory.join("range-source.docx")).unwrap()).unwrap();
    let selection = rocx::editor::TextRangeEdit {
        target: anchor(&ranged, 1),
        start: 1,
        end: 6,
        text: " 🐈 ".into(),
    };
    write(
        "native-range.docx",
        ranged
            .preview_with_ranges(&[], &[], &[], &[], &[], &[], &[], &[selection])
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    let BodyBlock::Paragraph(first) = &doc.body()[0] else {
        panic!("paragraph")
    };
    let rocx::editor::ImportedInlineNode::Run(run) = &first.inline_structure[0] else {
        panic!("run")
    };
    write(
        "native-removal.docx",
        doc.preview_with_run_removals(&[], &[], &[], &[], std::slice::from_ref(&run.anchor))
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    {
        use rocx::editor::{ImportedInlineNode, InlineAction, InlineEdit};
        let target = |index: usize| {
            let BodyBlock::Paragraph(p) = &doc.body()[index] else {
                panic!("paragraph")
            };
            let ImportedInlineNode::Run(run) = &p.inline_structure[0] else {
                panic!("run")
            };
            run.anchor.clone()
        };
        let payload = immediate(Component::resolve(Paragraph::from_children(vec![
            Hyperlink::new(
                HyperlinkTarget::external("https://example.invalid/inline").unwrap(),
                vec![TextRun::new("editable link").into()],
            )
            .into(),
            rocx::Break::new(rocx::BreakType::Line).into(),
        ])))
        .unwrap()
        .unwrap();
        let batch = [
            InlineEdit {
                target: target(4),
                action: InlineAction::InsertAfter(payload.runs()[1].clone()),
            },
            InlineEdit {
                target: target(0),
                action: InlineAction::SetBold(false),
            },
            InlineEdit {
                target: target(1),
                action: InlineAction::Replace(payload.runs()[0].clone()),
            },
        ];
        let preview = doc
            .preview_with_inlines(&[], &[], &[], &[], &[], &[], &[], &batch)
            .unwrap();
        assert_eq!(preview.inline_changes(), &[(0, 0), (1, 0), (4, 0)]);
        assert!(
            doc.preview_with_inlines(&[], &[], &[], &[], &[], &[], &[target(1)], &batch)
                .is_err()
        );
        write("native-inline.docx", preview.to_docx().unwrap());
    }
    for (name, a, t, h) in [
        ("native-noop.docx", &[][..], &[][..], &[][..]),
        (
            "native-body.docx",
            &alignments[..],
            &insertions[..],
            &[][..],
        ),
        (
            "native-mixed.docx",
            &alignments[..],
            &insertions[..],
            &local[..],
        ),
        (
            "native-mixedShared.docx",
            &alignments[..],
            &insertions[..],
            std::slice::from_ref(&shared),
        ),
    ] {
        write(
            name,
            doc.preview_transaction(a, t, h).unwrap().to_docx().unwrap(),
        );
    }
    let body = doc
        .preview_transaction(&alignments, &insertions, &[])
        .unwrap()
        .to_docx()
        .unwrap();
    let reopened = ImportedDocument::open(&body).unwrap();
    let rocx::editor::BodyBlock::Table(table) = &reopened.body()[5] else {
        panic!("expected second inserted table")
    };
    let cell = rocx::editor::CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index: 0,
        cell_index: 0,
        alignment: rocx::VerticalAlignment::Center,
    };
    write(
        "native-cell.docx",
        reopened
            .preview_with_cells(&[], &[], &[], &[], std::slice::from_ref(&cell))
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    let BodyBlock::Paragraph(later) = &reopened.body()[2] else {
        panic!("expected removable body run")
    };
    let rocx::editor::ImportedInlineNode::Run(removed) = &later.inline_structure[0] else {
        panic!("expected removable body run")
    };
    let insertion = rocx::editor::ParagraphInsertion {
        target: anchor(&reopened, 0),
        paragraph: immediate(Component::resolve(Paragraph::new("inserted paragraph")))
            .unwrap()
            .unwrap(),
    };
    use rocx::editor::{TableOperation::*, TableStructureEdit};
    let mut structure = body.clone();
    for operation in [
        InsertRow(1),
        RemoveRow(1),
        InsertColumn(1),
        RemoveColumn(1),
        InsertCell { row: 0, cell: 1 },
        RemoveCell { row: 0, cell: 1 },
        ReplaceCell {
            row: 0,
            cell: 1,
            blocks: vec![
                rocx::ResolvedBlock::Paragraph(insertion.paragraph.clone()),
                rocx::ResolvedBlock::Table(insertions[1].table.clone()),
            ],
        },
    ] {
        let source = ImportedDocument::open(&structure).unwrap();
        let BodyBlock::Table(table) = &source.body()[5] else {
            unreachable!()
        };
        structure = source
            .preview_with_table_structure(
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[TableStructureEdit {
                    target: table.anchor.clone(),
                    operation,
                }],
            )
            .unwrap()
            .to_docx()
            .unwrap();
    }
    write("native-tableStructure.docx", structure);
    write(
        "native-mixedBody.docx",
        reopened
            .preview_mixed(
                &[],
                &[],
                &[],
                &[],
                &[insertion],
                &[cell],
                std::slice::from_ref(&removed.anchor),
            )
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    for (name, e) in [
        ("native-shared.docx", &shared),
        ("native-local.docx", &local[1]),
    ] {
        write(
            name,
            doc.preview_header_image(e).unwrap().to_docx().unwrap(),
        );
    }
    let stories = doc.stories().unwrap();
    let first_page = stories
        .iter()
        .find(|s| {
            s.section_index == 3
                && s.kind == rocx::editor::SectionStoryKind::Header
                && s.variant == rocx::HeaderFooterType::First
        })
        .unwrap();
    for (name, mode) in [
        ("native-firstShared.docx", HeaderEditMode::Shared),
        ("native-firstLocal.docx", HeaderEditMode::SectionLocal),
    ] {
        write(
            name,
            doc.preview_header_image(&HeaderImageEdit {
                target: first_page.anchor.clone().unwrap(),
                mode,
                image: image.clone(),
            })
            .unwrap()
            .to_docx()
            .unwrap(),
        );
    }
    let relocated =
        ImportedDocument::open(&fs::read(directory.join("relocated.docx")).unwrap()).unwrap();
    let target = relocated.default_headers().unwrap()[1].anchor.clone();
    write(
        "native-relocated-out.docx",
        relocated
            .preview_header_image(&HeaderImageEdit {
                target,
                mode: HeaderEditMode::SectionLocal,
                image,
            })
            .unwrap()
            .to_docx()
            .unwrap(),
    );
}
