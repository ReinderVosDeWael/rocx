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
    let combined = doc
        .preview_edits(rocx::editor::EditBatch {
            ranges: &[rocx::editor::TextRangeEdit {
                target: anchor(&doc, 1),
                start: 0,
                end: 1,
                text: "X".into(),
            }],
            blocks: &[rocx::editor::BlockEdit::KeepWithNext(anchor(&doc, 4), true)],
            resources: &[rocx::editor::ResourceEdit::CoreTitle(Some(
                "Combined editor".into(),
            ))],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(combined.block_changes(), &[4]);
    write("native-combined.docx", combined.to_docx().unwrap());
    use rocx::editor::BlockEdit;
    let paragraph = immediate(Component::resolve(Paragraph::new("inserted paragraph")))
        .unwrap()
        .unwrap();
    let blocks = doc
        .preview_with_blocks(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[
                BlockEdit::KeepWithNext(anchor(&doc, 0), false),
                BlockEdit::Replace(anchor(&doc, 1), Box::new(paragraph.clone())),
                BlockEdit::ReplaceSection(
                    anchor(&doc, 4),
                    vec![
                        rocx::ResolvedBlock::Paragraph(paragraph.clone()),
                        rocx::ResolvedBlock::Paragraph(paragraph),
                        rocx::ResolvedBlock::Table(
                            immediate(Component::resolve(pair("next A", "next B")))
                                .unwrap()
                                .unwrap(),
                        ),
                    ],
                ),
            ],
        )
        .unwrap();
    assert_eq!(blocks.block_changes(), &[0, 1, 4]);
    let intermediate = ImportedDocument::open(&blocks.to_docx().unwrap()).unwrap();
    write(
        "native-blocks.docx",
        intermediate
            .preview_with_blocks(
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[BlockEdit::Remove(anchor(&intermediate, 5))],
            )
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    {
        use rocx::{ParagraphStyleDefinition, editor::StyleEdit};
        let catalog =
            ImportedDocument::open(&fs::read(directory.join("styles-source.docx")).unwrap())
                .unwrap();
        let style = |name: &str| ParagraphStyleDefinition::default().with_name(name);
        let changes = [
            StyleEdit::Add(style("Memo Added").with_base_style("Memo Active").into()),
            StyleEdit::Replace {
                target: catalog.style_anchor("MemoReplace").unwrap(),
                definition: style("Memo Replace").with_bold(true).into(),
            },
            StyleEdit::Remove(catalog.style_anchor("MemoDrop").unwrap()),
            StyleEdit::Patch {
                target: catalog.style_anchor("MemoActive").unwrap(),
                bold: None,
                italic: Some(true),
            },
        ];
        write(
            "native-styleEdits.docx",
            catalog
                .preview_with_styles(&[], &[], &[], &[], &[], &[], &[], &changes)
                .unwrap()
                .to_docx()
                .unwrap(),
        );
    }
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
    use rocx::{
        TextRevisionAction,
        editor::{
            ImportedInlineNode::{Revision, Run},
            TrackedText,
        },
    };
    let paragraph = |doc: &ImportedDocument, block, inline| -> rocx::editor::InlineAnchor {
        let BodyBlock::Paragraph(p) = &doc.body()[block] else {
            panic!("paragraph")
        };
        match &p.inline_structure[inline] {
            Run(r) => r.anchor.clone(),
            Revision(r) => r.anchor.clone(),
            _ => panic!("expected run or revision"),
        }
    };
    let date = "2024-02-29T12:34:56Z";
    {
        use rocx::{
            Field, FieldForm, FieldInstruction, FieldProperties,
            editor::{FieldPatch, ImportedInlineNode, InlineAction, InlineEdit},
        };
        let open = |bytes: Vec<u8>| ImportedDocument::open(&bytes).unwrap();
        let mut d = open(fs::read(directory.join("paired-source.docx")).unwrap());
        let field = immediate(Component::resolve(Paragraph::from_children(vec![
            Field::new(
                FieldInstruction::Page,
                vec![TextRun::new("new cache").into()],
            )
            .with_properties(FieldProperties::default().with_form(FieldForm::Complex))
            .into(),
        ])))
        .unwrap()
        .unwrap();
        d = open(
            d.replace_field(&anchor(&d, 0), 0, Some(&field.runs()[0]))
                .unwrap(),
        );
        d = open(
            d.patch_field(
                &anchor(&d, 0),
                0,
                FieldPatch::Instruction(" NUMPAGES ".into()),
            )
            .unwrap(),
        );
        let link = |d: &ImportedDocument, i: usize| {
            let BodyBlock::Paragraph(p) = &d.body()[2] else {
                panic!()
            };
            let ImportedInlineNode::Hyperlink(l) = &p.inline_structure[i] else {
                panic!()
            };
            l.anchor.clone()
        };
        let edit = InlineEdit {
            target: link(&d, 0),
            action: InlineAction::HyperlinkTooltip(Some("hover".into())),
        };
        d = open(
            d.preview_edits(rocx::editor::EditBatch {
                inline_edits: &[edit],
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap(),
        );
        d = open(
            d.set_paired_range(0, None, Some((&link(&d, 0), &paragraph(&d, 3, 0))))
                .unwrap(),
        );
        write(
            "native-fieldsRanges.docx",
            d.set_paired_range(
                7,
                Some("Crossing"),
                Some((&paragraph(&d, 1, 0), &link(&d, 1))),
            )
            .unwrap(),
        );
        use rocx::editor::{FieldAction, FieldEdit, PairedRangeEdit};
        let d = open(fs::read(directory.join("field-batch-source.docx")).unwrap());
        let children = immediate(Component::resolve(Paragraph::from_children(vec![
            Hyperlink::new(
                HyperlinkTarget::external("https://example.invalid/cache").unwrap(),
                vec![TextRun::new("remote").into()],
            )
            .into(),
            Hyperlink::new(
                HyperlinkTarget::bookmark(rocx::BookmarkName::new("BatchMark").unwrap()),
                vec![TextRun::new("jump").into()],
            )
            .into(),
        ])))
        .unwrap()
        .unwrap();
        let fields = [
            FieldEdit {
                target: anchor(&d, 4),
                inline_index: 1,
                action: FieldAction::Cache(children.runs().to_vec()),
            },
            FieldEdit{target:anchor(&d,4),inline_index:1,action:FieldAction::Patch(FieldPatch::Dirty(Some(false)))},
            FieldEdit {
                target: anchor(&d, 0),
                inline_index: 0,
                action: FieldAction::Patch(FieldPatch::Instruction(" NUMPAGES ".into())),
            },
            FieldEdit {
                target: anchor(&d, 0),
                inline_index: 0,
                action: FieldAction::Patch(FieldPatch::Dirty(Some(true))),
            },
        ];
        let ranges = [
            PairedRangeEdit {
                id: 7,
                bookmark: Some("BatchMark".into()),
                endpoints: Some((paragraph(&d, 1, 1), link(&d, 0))),
            },
            PairedRangeEdit {
                id: 0,
                bookmark: None,
                endpoints: Some((link(&d, 0), paragraph(&d, 3, 0))),
            },
        ];
        let edits = [InlineEdit {
            target: link(&d, 0),
            action: InlineAction::HyperlinkTooltip(Some("Batch tooltip".into())),
        },InlineEdit{target:link(&d,0),action:InlineAction::HyperlinkHistory(Some(false))}];
        write(
            "native-fieldBatch.docx",
            d.preview_edits(rocx::editor::EditBatch {
                fields: &fields,
                paired_ranges: &ranges,
                inline_edits: &edits,
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap(),
        );
    }
    let tracked = doc
        .preview_tracked_text(
            &run.anchor,
            TrackedText::Replace(" Revised ".into()),
            "A & B",
            Some(date),
        )
        .unwrap()
        .to_docx()
        .unwrap();
    let review = ImportedDocument::open(&tracked).unwrap();
    write(
        "native-reviewed.docx",
        review
            .preview_revision(&paragraph(&review, 0, 1), TextRevisionAction::Accept, None)
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    write("native-tracked.docx", tracked);
    let moved = doc
        .preview_tracked_move(&run.anchor, &paragraph(&doc, 1, 0), "Move1", "A & B", date)
        .unwrap()
        .to_docx()
        .unwrap();
    let review = ImportedDocument::open(&moved).unwrap();
    write(
        "native-moveAccepted.docx",
        review
            .preview_revision(
                &paragraph(&review, 0, 1),
                TextRevisionAction::Accept,
                Some(&paragraph(&review, 1, 2)),
            )
            .unwrap()
            .to_docx()
            .unwrap(),
    );
    write("native-moved.docx", moved);
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
