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
    {
        use rocx::editor::ControlEdit;
        let data = fs::read(directory.join("control-source.docx")).unwrap();
        let source = ImportedDocument::open(&data).unwrap();
        let c = source.controls();
        assert_eq!(
            c.iter().map(|c| c.context.as_str()).collect::<Vec<_>>(),
            ["block", "inline", "row", "cell", "block", "block"]
        );
        assert!(
            c.iter()
                .all(|c| c.tag.as_deref() == Some("repeat") && c.id.is_none())
                && c[4].bound
        );
        let edit =
            |i: usize, tag: Option<&str>, alias: Option<&str>, text: Option<&str>| ControlEdit {
                target: c[i].anchor.clone(),
                tag: tag.map(str::to_owned),
                alias: alias.map(str::to_owned),
                text: text.map(str::to_owned),
            };
        let edits = [
            edit(0, None, None, Some(" block ")),
            edit(1, None, Some("caption"), None),
            edit(2, None, None, Some("new row")),
            edit(3, Some("cell"), None, Some("new cell")),
        ];
        let preview = |doc: &ImportedDocument, edits: &[ControlEdit]| {
            doc.preview_with_controls(&[], &[], &[], &[], &[], &[], &[], edits)
        };
        for (bad, reason) in [
            (edit(1, None, None, Some("no")), "locked"),
            (edit(0, Some("no"), None, None), "locked"),
            (edit(4, None, Some("no"), None), "XML-bound"),
            (edit(5, None, None, Some("no")), "structure"),
            (edits[0].clone(), "duplicate"),
        ] {
            let batch = if bad.tag.is_some() {
                vec![bad]
            } else {
                vec![edits[0].clone(), bad]
            };
            assert!(
                preview(&source, &batch)
                    .unwrap_err()
                    .to_string()
                    .contains(reason)
            );
        }
        assert!(
            preview(&ImportedDocument::open(&data).unwrap(), &edits)
                .unwrap_err()
                .to_string()
                .contains("snapshot")
        );
        write(
            "native-controlEdits.docx",
            preview(&source, &edits).unwrap().to_docx().unwrap(),
        );
    }
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
