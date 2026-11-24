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

fn physical_resources(directory: &std::path::Path) -> Vec<u8> {
    use rocx::editor::{
        EditBatch, LayoutBorder, LayoutEdit, LayoutField, LayoutValue, NumberingEdit as N,
        ResourceEdit::Numbering, StyleEdit, TableOperation, TableSite, TableStructureEdit,
    };
    let owner = "word/footer1.xml";
    let source = fs::read(directory.join("physical-resources-source.docx")).unwrap();
    let original = ImportedDocument::open(&source).unwrap();
    let target = original.story_paragraph_anchor(owner, 0, None).unwrap();
    let staged = original
        .preview_edits(EditBatch {
            styles: &[
                StyleEdit::Add(
                    rocx::TableStyleDefinition::default()
                        .with_name("Owner Table Style")
                        .into(),
                ),
                StyleEdit::Add(
                    rocx::NumberingStyleDefinition {
                        name: "Owner Numbering Style".into(),
                        numbering_id: 773,
                    }
                    .into(),
                ),
            ],
            resources: &[
                Numbering(N::AddAbstract {
                    abstract_id: 772,
                    levels: vec![rocx::NumberingLevel {
                        level: 0,
                        start: 1,
                        format: "decimal".into(),
                        text: "%1.".into(),
                        ..Default::default()
                    }],
                }),
                Numbering(N::AddInstance {
                    num_id: 773,
                    abstract_id: 772,
                }),
                Numbering(N::StoryParagraph {
                    target,
                    value: Some(rocx::NumberingReference { id: 773, level: 0 }),
                }),
            ],
            ..Default::default()
        })
        .unwrap()
        .to_docx()
        .unwrap();
    let numbered = ImportedDocument::open(&staged).unwrap();
    let donor = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("Donor text must not move").into()]).into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties::default().with_style("Owner Table Style"));
    let donor = immediate(donor.resolve()).unwrap().unwrap();
    let table = &numbered.part_tables(Some(owner)).unwrap()[0];
    let tables_edited = numbered
        .edit_part_tables(
            owner,
            &[TableStructureEdit {
                target: table.anchor.clone(),
                operation: TableOperation::Property {
                    site: TableSite::Table,
                    field: "style".into(),
                    source: Some(Box::new(donor)),
                },
            }],
        )
        .unwrap();
    let pre_layout = ImportedDocument::open(&tables_edited).unwrap();
    let section = &pre_layout.sections().unwrap()[0].anchor;
    pre_layout
        .preview_layout(&[
            LayoutEdit::Section(
                section.clone(),
                LayoutField::MarginLeft,
                Some(LayoutValue::Integer(1440)),
            ),
            LayoutEdit::Section(
                section.clone(),
                LayoutField::BorderTop,
                Some(LayoutValue::Border(LayoutBorder {
                    size: Some(12),
                    color: Some("123456".into()),
                    space: None,
                })),
            ),
        ])
        .unwrap()
        .to_docx()
        .unwrap()
}

fn structural_resources(directory: &std::path::Path) -> Vec<u8> {
    use rocx::editor::{
        CustomValue, EditBatch, NumberingEdit as N,
        ResourceEdit::{CustomProperty, Numbering},
        StyleEdit, TableOperation, TableSite, TableStructureEdit,
    };

    let original = fs::read(directory.join("table-layout.docx")).unwrap();
    let doc = ImportedDocument::open(&original).unwrap();
    let table = |index: usize| {
        let BodyBlock::Table(table) = &doc.body()[index] else {
            panic!("expected table");
        };
        table.anchor.clone()
    };
    let paragraph = anchor(&doc, 2);
    let replacement = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![
                Paragraph::from_children(vec![
                    Hyperlink::new(
                        HyperlinkTarget::external("https://example.invalid/staged-table").unwrap(),
                        vec![TextRun::new("staged link").into()],
                    )
                    .into(),
                ])
                .into(),
            ])
            .into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties::default().with_style("Staged Table"));
    let replacement = immediate(Component::resolve(replacement)).unwrap().unwrap();
    let structural = [
        TableStructureEdit {
            target: table(0),
            operation: TableOperation::ReplaceNode {
                site: TableSite::Table,
                source: Box::new(replacement),
            },
        },
        TableStructureEdit {
            target: table(1),
            operation: TableOperation::RemoveTable,
        },
    ];
    let styles = [StyleEdit::Add(
        rocx::TableStyleDefinition::default()
            .with_name("Staged Table")
            .into(),
    )];
    let reference = rocx::NumberingReference { id: 91, level: 0 };
    let resources = [
        Numbering(N::AddAbstract {
            abstract_id: 90,
            levels: vec![
                rocx::NumberingLevel::default()
                    .with_start(1)
                    .with_format("decimal")
                    .with_text("%1."),
            ],
        }),
        Numbering(N::AddInstance {
            num_id: 91,
            abstract_id: 90,
        }),
        Numbering(N::Paragraph {
            target: paragraph.clone(),
            value: Some(reference.clone()),
        }),
        CustomProperty("Staged".into(), Some(CustomValue::Boolean(true))),
    ];
    let preview = |resources| {
        doc.preview_edits(EditBatch {
            structural: &structural,
            styles: &styles,
            resources,
            ..Default::default()
        })
    };
    // Existing ListBullet still uses numId 1. Reject the whole batch after
    // staging the new body, style, hyperlink relationship and custom part.
    let mut rejected = resources.to_vec();
    rejected.push(Numbering(N::RemoveInstance { num_id: 1 }));
    let Err(error) = preview(&rejected) else {
        panic!("removed live numbering")
    };
    assert!(error.to_string().contains("references"), "{error}");
    let data = preview(&resources).unwrap().to_docx().unwrap();
    assert_eq!(preview(&resources).unwrap().to_docx().unwrap(), data);
    assert_eq!(doc.paragraph_numbering(&paragraph).unwrap(), None);
    let reopened = ImportedDocument::open(&data).unwrap();
    assert_eq!(reopened.body().len(), 2);
    assert!(reopened.paragraph_numbering(&paragraph).is_err());
    assert_eq!(
        reopened.paragraph_numbering(&anchor(&reopened, 1)).unwrap(),
        Some(reference)
    );
    data
}

fn table_nodes(directory: &std::path::Path, pair: &rocx::ResolvedTable) -> Vec<u8> {
    use rocx::editor::{EditBatch, TableOperation::*, TableSite, TableStructureEdit};
    let doc =
        ImportedDocument::open(&fs::read(directory.join("table-layout.docx")).unwrap()).unwrap();
    let edit = |doc: &ImportedDocument, index, operation| {
        let BodyBlock::Table(t) = &doc.body()[index] else {
            panic!("table")
        };
        TableStructureEdit {
            target: t.anchor.clone(),
            operation,
        }
    };
    let single = immediate(Component::resolve(
        Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::new("cell").into()])
                    .with_properties(
                        TableCellProperties::default()
                            .with_margins(rocx::CellMargins::default().with_top(4.5)),
                    )
                    .into(),
            ])
            .with_properties(rocx::TableRowProperties::default().with_repeat_header(false))
            .into(),
        ])
        .with_properties(TableProperties::default().with_alignment(rocx::TableAlignment::Center)),
    ))
    .unwrap()
    .unwrap();
    let mut data = doc
        .preview_edits(EditBatch {
            structural: &[
                edit(
                    &doc,
                    0,
                    ReplaceNode {
                        site: TableSite::Table,
                        source: Box::new(pair.clone()),
                    },
                ),
                edit(&doc, 1, RemoveTable),
            ],
            alignments: &[ParagraphAlignmentEdit {
                target: anchor(&doc, 2),
                alignment: Alignment::Right,
            }],
            ..Default::default()
        })
        .unwrap()
        .to_docx()
        .unwrap();
    for ops in [
        vec![ReplaceNode {
            site: TableSite::Row(0),
            source: Box::new(pair.clone()),
        }],
        vec![ReplaceNode {
            site: TableSite::Cell(0, 1),
            source: Box::new(single.clone()),
        }],
        [
            (TableSite::Table, "alignment"),
            (TableSite::Row(0), "repeatHeader"),
            (TableSite::Cell(0, 0), "margin.top"),
        ]
        .into_iter()
        .map(|(site, field)| Property {
            site,
            field: field.into(),
            source: Some(Box::new(single.clone())),
        })
        .collect(),
    ] {
        let doc = ImportedDocument::open(&data).unwrap();
        let edits = ops
            .into_iter()
            .map(|op| edit(&doc, 0, op))
            .collect::<Vec<_>>();
        data = doc
            .preview_edits(EditBatch {
                structural: &edits,
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap();
    }
    data
}

fn main() {
    let directory = PathBuf::from(env::args_os().nth(1).expect("fixture directory"));
    fs::write(
        directory.join("native-structuralResources.docx"),
        structural_resources(&directory),
    )
    .unwrap();
    {
        use rocx::editor::{
            ImportedInlineNode::{Hyperlink, Run},
            ImportedRowChild, ImportedTableChild, InlineAction, InlineEdit,
        };
        let mut bytes = fs::read(directory.join("format-owners-source.docx")).unwrap();
        for label in ["cell", "header", "footer"] {
            let source = ImportedDocument::open(&bytes).unwrap();
            let stories = source.stories().unwrap();
            let (owner, structure) = if label == "cell" {
                let BodyBlock::Table(t) = &source.body()[0] else {
                    panic!("table")
                };
                let ImportedTableChild::Row(r) = &t.children[0] else {
                    panic!("row")
                };
                let ImportedRowChild::Cell(c) = &r.children[0] else {
                    panic!("cell")
                };
                (None, c.inline_structure(0).unwrap())
            } else {
                let s = stories
                    .iter()
                    .find(|s| {
                        s.kind.as_str() == label
                            && s.section_index == 1
                            && s.variant == rocx::HeaderFooterType::Default
                    })
                    .unwrap();
                assert!(s.inherited);
                (Some(s.part_name.as_str()), s.inline_structure(0).unwrap())
            };
            let Hyperlink(link) = &structure[0] else {
                panic!("link")
            };
            let Run(r) = &link.children[0] else {
                panic!("run")
            };
            bytes = source
                .edit_run_format(
                    owner,
                    &[InlineEdit {
                        target: r.anchor.clone(),
                        action: InlineAction::PatchFormat {
                            set: Box::new(RunFormat::default().with_size(13.5)),
                            clear: vec![],
                        },
                    }],
                )
                .unwrap();
        }
        fs::write(directory.join("native-formatOwners.docx"), bytes).unwrap();
    }
    fs::write(
        directory.join("native-physicalResources.docx"),
        physical_resources(&directory),
    )
    .unwrap();
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
    let write = |name: &str, bytes: Vec<u8>| fs::write(directory.join(name), bytes).unwrap();
    {
        use rocx::{
            Color, LineHeight, NumberingAlignment, NumberingLevel, NumberingReference,
            NumberingSuffix, ParagraphFormat, TabAlignment, TabLeader, TabStop, ThemeColor,
            Underline,
            editor::{
                DefaultField, DefaultValue, EditBatch, ImportedInlineNode, InlineAction,
                InlineEdit, MetadataField, NumberingEdit as N, ResourceEdit as R,
                TableStructureEdit,
            },
        };
        let open = |name| ImportedDocument::open(&fs::read(directory.join(name)).unwrap()).unwrap();
        let doc = open("numbering-linked.docx");
        let catalog = doc.numbering_catalog().unwrap();
        let owner = catalog
            .abstracts
            .iter()
            .find(|a| a.abstract_id == 710)
            .unwrap();
        assert_eq!(owner.style_link.as_deref(), Some("FixtureList"));
        let override_level = &catalog
            .instances
            .iter()
            .find(|i| i.num_id == 712)
            .unwrap()
            .level_overrides[0];
        assert_eq!(&owner.levels[0], override_level);
        assert_eq!(
            override_level.label_format.as_ref().unwrap().size,
            Some(12.5)
        );
        assert_eq!(
            override_level
                .paragraph_format
                .as_ref()
                .unwrap()
                .line_height,
            Some(LineHeight::Exact(10.))
        );
        let level = NumberingLevel::default()
            .with_start(5)
            .with_format("ordinal")
            .with_text("%1)")
            .with_restart(0)
            .with_is_legal(false)
            .with_suffix(NumberingSuffix::Space)
            .with_alignment(NumberingAlignment::Start)
            .with_paragraph_style("Heading 2")
            .with_label_format(
                RunFormat::default()
                    .with_bold(false)
                    .with_size(12.5)
                    .with_underline(Underline::Double)
                    .with_color(Color::Theme {
                        name: ThemeColor::Accent2,
                        tint: Some(16),
                        shade: None,
                    }),
            )
            .with_paragraph_format(
                ParagraphFormat::default()
                    .with_alignment(Alignment::Center)
                    .with_first_line_indent(-9.)
                    .with_line_height(LineHeight::Exact(11.))
                    .with_tab_stops(vec![
                        TabStop::default()
                            .with_position(12.)
                            .with_alignment(TabAlignment::Right)
                            .with_leader(TabLeader::Dot),
                    ]),
            );
        let replace = R::Numbering(N::ReplaceAbstract {
            abstract_id: 710,
            levels: vec![level],
        });
        for name in ["numbering-dangling.docx", "numbering-opaque.docx"] {
            assert!(
                open(name)
                    .preview_edits(EditBatch {
                        resources: std::slice::from_ref(&replace),
                        ..Default::default()
                    })
                    .is_err()
            );
        }
        let resources = [
            replace,
            R::Numbering(N::Paragraph {
                target: anchor(&doc, 0),
                value: Some(NumberingReference { id: 713, level: 0 }),
            }),
            R::Default(
                DefaultField::TrackRevisions,
                Some(DefaultValue::Bool(false)),
            ),
            R::Metadata(
                MetadataField::Comments,
                Some("Numbering fixture review".into()),
            ),
        ];
        write(
            "native-linkedNumbering.docx",
            doc.preview_edits(EditBatch {
                resources: &resources,
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap(),
        );
        let mixed = open("linked-mixed-source.docx");
        let [
            BodyBlock::Paragraph(p),
            BodyBlock::Table(first),
            BodyBlock::Table(last),
        ] = mixed.body()
        else {
            panic!("linked mixed fixture body")
        };
        let ImportedInlineNode::Run(run) = &p.inline_structure[0] else {
            panic!("linked mixed fixture run")
        };
        let mixed_resources = [
            resources[0].clone(),
            R::Numbering(N::Paragraph {
                target: p.anchor.clone(),
                value: Some(NumberingReference { id: 713, level: 0 }),
            }),
            resources[2].clone(),
            resources[3].clone(),
            R::Numbering(N::StoryParagraph {
                target: mixed.cell_paragraph_anchor(&last.anchor, 0, 0, 0).unwrap(),
                value: Some(NumberingReference { id: 713, level: 0 }),
            }),
        ];
        write(
            "native-linkedMixed.docx",
            mixed
                .preview_edits(EditBatch {
                    resources: &mixed_resources,
                    inline_edits: &[InlineEdit {
                        target: run.anchor.clone(),
                        action: InlineAction::PatchFormat {
                            set: Box::new(RunFormat::default().with_bold(true)),
                            clear: vec![],
                        },
                    }],
                    structural: &[TableStructureEdit {
                        target: first.anchor.clone(),
                        operation: TableOperation::RemoveTable,
                    }],
                    ..Default::default()
                })
                .unwrap()
                .to_docx()
                .unwrap(),
        );
    }
    use rocx::editor::{TableOperation, TableSite};
    let mut physical = fs::read(directory.join("physical-table-source.docx")).unwrap();
    let linked_cell = immediate(Component::resolve(Paragraph::from_children(vec![
        Hyperlink::new(
            HyperlinkTarget::external("https://example.invalid/physical-cell").unwrap(),
            vec![TextRun::new("Footer link").into()],
        )
        .into(),
    ])))
    .unwrap()
    .unwrap();
    for (owner, depth, operation) in [
        (None, 1, TableOperation::InsertRow(1)),
        (
            Some("word/footer1.xml"),
            1,
            TableOperation::Layout(Some(rocx::TableLayout::Fixed)),
        ),
        (
            Some("word/footer1.xml"),
            1,
            TableOperation::ReplaceCell {
                row: 0,
                cell: 0,
                blocks: vec![rocx::ResolvedBlock::Paragraph(linked_cell)],
            },
        ),
        (
            Some("word/comments.xml"),
            0,
            TableOperation::Property {
                site: TableSite::Column(0),
                field: "width".into(),
                source: None,
            },
        ),
    ] {
        let doc = ImportedDocument::open(&physical).unwrap();
        let t = doc
            .part_tables(owner)
            .unwrap()
            .into_iter()
            .find(|t| t.depth == depth)
            .unwrap();
        physical = doc
            .edit_part_tables(
                &t.part_name,
                &[rocx::editor::TableStructureEdit {
                    target: t.anchor,
                    operation,
                }],
            )
            .unwrap();
    }
    write("native-physicalTables.docx", physical);
    write(
        "native-tableNodes.docx",
        table_nodes(&directory, &insertions[1].table),
    );
    {
        use rocx::editor::{
            LayoutBorder, LayoutEdit as E, LayoutField as F, LayoutTab, LayoutValue as V,
        };
        let d = ImportedDocument::open(&fs::read(directory.join("layout-source.docx")).unwrap())
            .unwrap();
        let sections = d.sections().unwrap();
        let p = |f, v| E::Paragraph(anchor(&d, 1), f, v);
        let s = |i: usize, f, v| E::Section(sections[i].anchor.clone(), f, Some(v));
        let edits = [
            p(F::SpaceAfter, Some(V::Integer(180))),
            p(F::Heading, Some(V::Integer(2))),
            p(F::WidowControl, Some(V::Boolean(false))),
            p(F::Alignment, None),
            p(F::FirstLineIndent, Some(V::Integer(-240))),
            p(
                F::TabStops,
                Some(V::Tabs(vec![LayoutTab {
                    position: 720,
                    alignment: "right".into(),
                    leader: Some("dot".into()),
                }])),
            ),
            s(0, F::PageWidth, V::Integer(16000)),
            s(0, F::MarginTop, V::Integer(-220)),
            s(0, F::ColumnCount, V::Integer(2)),
            s(
                0,
                F::BorderTop,
                V::Border(LayoutBorder {
                    color: Some("223344".into()),
                    size: Some(8),
                    space: None,
                }),
            ),
            s(1, F::GridPitch, V::Integer(360)),
            s(1, F::TitlePage, V::Boolean(false)),
        ];
        write(
            "native-layout.docx",
            d.preview_layout(&edits).unwrap().to_docx().unwrap(),
        );
        let split = d
            .preview_blocks(&[rocx::editor::BlockEdit::SplitSection(
                sections[0].anchor.clone(),
                1,
            )])
            .unwrap();
        assert_eq!(split.block_changes(), &[1]);
        write("native-sectionSplit.docx", split.to_docx().unwrap());
        let split = ImportedDocument::open(&split.to_docx().unwrap()).unwrap();
        let merge = split
            .preview_blocks(&[rocx::editor::BlockEdit::MergeSection(
                split.sections().unwrap()[0].anchor.clone(),
            )])
            .unwrap();
        write("native-sectionMerge.docx", merge.to_docx().unwrap());
        let p = futures::executor::block_on(rocx::Component::resolve(rocx::Paragraph::new(
            "Shared story replacement",
        )))
        .unwrap()
        .unwrap();
        write(
            "native-storyShared.docx",
            d.edit_story_blocks(
                &d.default_headers().unwrap()[1].anchor,
                HeaderEditMode::Shared,
                0,
                1,
                &[rocx::ResolvedBlock::Paragraph(p)],
            )
            .unwrap(),
        );
    }
    let combined = doc
        .preview_edits(rocx::editor::EditBatch {
            styles: &[rocx::editor::StyleEdit::Add(
                rocx::NumberingStyleDefinition {
                    name: "Batch List".into(),
                    numbering_id: 702,
                }
                .into(),
            )],
            ranges: &[rocx::editor::TextRangeEdit {
                target: anchor(&doc, 1),
                start: 0,
                end: 1,
                text: "X".into(),
            }],
            blocks: &[
                BlockEdit::KeepWithNext(anchor(&doc, 4), true),
                BlockEdit::ParagraphFlag(
                    anchor(&doc, 4),
                    rocx::editor::ParagraphFlag::PageBreakBefore,
                    Some(false),
                ),
                BlockEdit::SectionFlag(
                    anchor(&doc, 4),
                    rocx::editor::SectionFlag::TitlePage,
                    Some(true),
                ),
                BlockEdit::SectionType(anchor(&doc, 0), Some(rocx::SectionType::Continuous)),
            ],
            resources: &[
                rocx::editor::ResourceEdit::Default(
                    rocx::editor::DefaultField::TabStops,
                    Some(rocx::editor::DefaultValue::Tabs(vec![
                        rocx::editor::DefaultTab {
                            position_twips: 720,
                            alignment: "right".into(),
                            leader: Some("dot".into()),
                        },
                    ])),
                ),
                rocx::editor::ResourceEdit::CoreTitle(Some("Combined editor".into())),
                rocx::editor::ResourceEdit::Metadata(
                    rocx::editor::MetadataField::Subject,
                    Some("Batch resources".into()),
                ),
                rocx::editor::ResourceEdit::Default(
                    rocx::editor::DefaultField::Bold,
                    Some(rocx::editor::DefaultValue::Bool(false)),
                ),
                rocx::editor::ResourceEdit::Numbering(rocx::editor::NumberingEdit::AddInstance {
                    num_id: 702,
                    abstract_id: 701,
                }),
                rocx::editor::ResourceEdit::Numbering(rocx::editor::NumberingEdit::AddAbstract {
                    abstract_id: 701,
                    levels: vec![rocx::NumberingLevel {
                        level: 0,
                        start: 1,
                        format: "decimal".into(),
                        text: "%1.".into(),
                        ..Default::default()
                    }],
                }),
            ],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(combined.block_changes(), &[0, 4, 4, 4]);
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
        use rocx::editor::{EditBatch, NumberingEdit as N, ResourceEdit::Numbering};
        let numbers = catalog
            .preview_edits(EditBatch {
                resources: &[
                    Numbering(N::AddAbstract {
                        abstract_id: 90,
                        levels: vec![rocx::NumberingLevel {
                            level: 0,
                            start: 1,
                            format: "decimal".into(),
                            text: "%1.".into(),
                            ..Default::default()
                        }],
                    }),
                    Numbering(N::AddInstance {
                        num_id: 91,
                        abstract_id: 90,
                    }),
                    Numbering(N::Paragraph {
                        target: anchor(&catalog, 0),
                        value: Some(rocx::NumberingReference { id: 91, level: 0 }),
                    }),
                ],
                ..Default::default()
            })
            .unwrap();
        write("native-numberingEdits.docx", numbers.to_docx().unwrap());
        let owned = catalog
            .preview_edits(EditBatch {
                styles: &[StyleEdit::Add(
                    rocx::NumberingStyleDefinition {
                        name: "Owned List".into(),
                        numbering_id: 91,
                    }
                    .into(),
                )],
                resources: &[
                    Numbering(N::AddInstance {
                        num_id: 91,
                        abstract_id: 90,
                    }),
                    Numbering(N::AddAbstract {
                        abstract_id: 90,
                        levels: vec![rocx::NumberingLevel {
                            level: 0,
                            start: 1,
                            format: "decimal".into(),
                            text: "%1.".into(),
                            ..Default::default()
                        }],
                    }),
                    Numbering(N::InstanceStart {
                        num_id: 91,
                        level: 0,
                        start: Some(6),
                    }),
                ],
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap();
        write("native-styleOwnedList.docx", owned);
        let style = |name: &str| ParagraphStyleDefinition::default().with_name(name);
        let changes = [
            StyleEdit::Add(style("Memo Added").with_base_style("Memo Active").into()),
            StyleEdit::Replace {
                target: catalog.style_anchor("MemoReplace").unwrap(),
                definition: style("Memo Replace").with_bold(true).into(),
            },
            StyleEdit::Remove(catalog.style_anchor("MemoDrop").unwrap()),
            StyleEdit::Field {
                target: catalog.style_anchor("Heading1").unwrap(),
                region: None,
                field: "next".into(),
                value: Some("MemoAdded".into()),
            },
            StyleEdit::Region {
                target: catalog.style_anchor("TableGrid").unwrap(),
                region: "nwCell".into(),
                format: Some(Box::new(rocx::TableSectionFormat {
                    bold: Some(true),
                    background: Some([10, 20, 30]),
                    ..Default::default()
                })),
            },
            StyleEdit::Field {
                target: catalog.style_anchor("LightShading").unwrap(),
                region: Some("firstRow".into()),
                field: "shd.fill".into(),
                value: Some("BADACE".into()),
            },
            StyleEdit::Field {
                target: catalog.style_anchor("LightShading").unwrap(),
                region: Some("firstRow".into()),
                field: "spacing.after".into(),
                value: Some("180".into()),
            },
            StyleEdit::Add(
                rocx::NumberingStyleDefinition {
                    name: "Memo List".into(),
                    numbering_id: 1,
                }
                .into(),
            ),
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
        let preview = |doc: &ImportedDocument, edits: &[StyleEdit]| {
            doc.preview_edits(rocx::editor::EditBatch {
                styles: edits,
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap()
        };
        write(
            "native-styleRetarget.docx",
            preview(
                &catalog,
                &[StyleEdit::Retarget {
                    target: catalog.style_anchor("MemoActive").unwrap(),
                    replacement: catalog.style_anchor("MemoReplace").unwrap(),
                }],
            ),
        );
        let missing =
            ImportedDocument::open(&fs::read(directory.join("styles-missing.docx")).unwrap())
                .unwrap();
        write(
            "native-styleCreate.docx",
            preview(&missing, &[StyleEdit::Add(style("First Style").into())]),
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
        let bytes = fs::read(directory.join("control-choice-source.docx")).unwrap();
        let original = ImportedDocument::open(&bytes).unwrap();
        let choice = &original.controls()[0];
        assert_eq!(
            (choice.kind, choice.selected.as_deref()),
            ("dropDownList", Some("1"))
        );
        assert_eq!(choice.choices.len(), 2);
        assert!(
            original
                .preview_control_choice(&choice.anchor, "absent", None)
                .is_err()
        );
        assert!(source.preview_unwrap_control(&c[0].anchor).is_err());
        assert!(source.preview_unwrap_control(&c[4].anchor).is_err());
        let selected = ImportedDocument::open(
            &original
                .preview_control_choice(&choice.anchor, "2", None)
                .unwrap()
                .to_docx()
                .unwrap(),
        )
        .unwrap();
        let second = &selected.controls()[0];
        assert_eq!(second.choices, choice.choices);
        let items = [rocx::ContentControlListItem {
            display_text: " Revised ".into(),
            value: "R".into(),
        }];
        let mut data = selected
            .preview_control_choice(&second.anchor, "R", Some(&items))
            .unwrap()
            .to_docx()
            .unwrap();
        for ctx in ["block", "inline", "row", "cell"] {
            let doc = ImportedDocument::open(&data).unwrap();
            let site = doc
                .control_sites()
                .iter()
                .find(|s| s.context.as_str() == ctx && s.text == ctx)
                .unwrap();
            assert!(original.preview_wrap_control(site, ctx).is_err());
            data = doc
                .preview_wrap_control(site, ctx)
                .unwrap()
                .to_docx()
                .unwrap();
        }
        let doc = ImportedDocument::open(&data).unwrap();
        let props = rocx::ContentControlProperties::default()
            .with_tag("inline")
            .with_id(42)
            .with_text(rocx::ContentControlText::Plain {
                multi_line: Some(false),
            });
        let by = |tag| {
            &doc.controls()
                .iter()
                .find(|c| c.tag.as_deref() == Some(tag))
                .unwrap()
                .anchor
        };
        assert!(doc.preview_replace_control(by("row"), &props).is_err());
        data = doc
            .preview_replace_control(by("inline"), &props)
            .unwrap()
            .to_docx()
            .unwrap();
        write("native-controlWrapped.docx", data.clone());
        for ctx in ["block", "inline", "row", "cell"] {
            let doc = ImportedDocument::open(&data).unwrap();
            let c = doc
                .controls()
                .iter()
                .find(|c| c.context.as_str() == ctx && c.tag.as_deref() == Some(ctx))
                .unwrap();
            data = doc
                .preview_unwrap_control(&c.anchor)
                .unwrap()
                .to_docx()
                .unwrap();
        }
        write("native-controlUnwrapped.docx", data);
    }
    {
        use rocx::editor::{
            EditBatch, FieldAction, FieldEdit, FieldPatch, ImportedInlineNode, InlineAction,
            InlineEdit, PairedRangeEdit, ParagraphInsertion, WrapperEdit as W,
        };
        let source =
            ImportedDocument::open(&fs::read(directory.join("rich-control-source.docx")).unwrap())
                .unwrap();
        let stories = source.story_controls().unwrap();
        assert_eq!(stories.len(), 5);
        assert_eq!(stories.iter().filter(|c| c.entry_id.is_some()).count(), 3);
        assert!(source.preview_unwrap_control(&stories[0].anchor).is_err());
        let site = |ctx, label| {
            source
                .control_sites()
                .iter()
                .find(|s| s.context.as_str() == ctx && s.text == label)
                .unwrap()
                .clone()
        };
        let wrap = |ctx, first, last, tag: &str| W::Wrap {
            first: site(ctx, first),
            last: Some(site(ctx, last)),
            tag: tag.into(),
        };
        let c = source.controls();
        let p = |s: &str| {
            immediate(Component::resolve(Paragraph::new(s)))
                .unwrap()
                .unwrap()
        };
        let wrappers = [
            wrap("block", "range start", "range end", "group"),
            wrap("inline", "one", "two", "runs"),
            W::ReplaceBlocks(
                c[0].anchor.clone(),
                vec![
                    rocx::ResolvedBlock::Paragraph(p("A")),
                    rocx::ResolvedBlock::Paragraph(p("B")),
                ],
            ),
            W::ReplaceInlines(
                c[1].anchor.clone(),
                vec![
                    p("picture:").runs()[0].clone(),
                    rocx::ResolvedInline::Image {
                        image: image.clone(),
                        format: Default::default(),
                        comment: None,
                    },
                ],
            ),
        ];
        let headers = source.default_headers().unwrap();
        let h = HeaderImageEdit {
            target: headers[0].anchor.clone(),
            mode: HeaderEditMode::Shared,
            image: image.clone(),
        };
        let run = |source: &ImportedDocument, p: usize, i: usize| {
            let BodyBlock::Paragraph(p) = &source.body()[p] else {
                panic!("expected paragraph")
            };
            let ImportedInlineNode::Run(r) = &p.inline_structure[i] else {
                panic!("expected run")
            };
            r.anchor.clone()
        };
        let field_index = |source: &ImportedDocument| {
            source.body().iter().position(|p| matches!(p,BodyBlock::Paragraph(p) if p.text.starts_with("mixed-field left"))).unwrap()
        };
        let idx = field_index(&source);
        let fields = [FieldEdit {
            target: anchor(&source, idx),
            inline_index: 1,
            action: FieldAction::Patch(FieldPatch::Dirty(Some(true))),
        }];
        let range = |id, name: &str, first, last| PairedRangeEdit {
            id,
            bookmark: Some(name.into()),
            endpoints: Some((first, last)),
        };
        let paired_ranges = [range(
            91,
            "MixedControl",
            run(&source, idx, 0),
            run(&source, idx, 2),
        )];
        assert!(
            source
                .preview_edits(EditBatch {
                    wrappers: &wrappers,
                    paired_ranges: &[range(92, "Crossed", run(&source, 1, 0), run(&source, 3, 0))],
                    ..Default::default()
                })
                .is_err()
        );
        let inline_edits = [
            InlineEdit {
                target: run(&source, 0, 0),
                action: InlineAction::InsertAfter(rocx::ResolvedInline::Image {
                    image: image.clone(),
                    format: Default::default(),
                    comment: None,
                }),
            },
            InlineEdit {
                target: run(&source, idx, 0),
                action: InlineAction::PatchFormat {
                    set: Box::new(rocx::RunFormat {
                        size: Some(13.5),
                        ..Default::default()
                    }),
                    clear: vec![],
                },
            },
        ];
        let batch = EditBatch {
            wrappers: &wrappers,
            fields: &fields,
            paired_ranges: &paired_ranges,
            inline_edits: &inline_edits,
            header_images: &[h],
            paragraphs: &[ParagraphInsertion {
                target: anchor(&source, 0),
                paragraph: p("adjacent"),
            }],
            ..Default::default()
        };
        let data = source.preview_edits(batch).unwrap().to_docx().unwrap();
        let reopened = ImportedDocument::open(&data).unwrap();
        assert!(
            reopened
                .preview_tracked_bold(
                    &run(&reopened, field_index(&reopened), 1),
                    false,
                    "Reviewer",
                    None
                )
                .is_err()
        );
        write("native-richControls.docx", data);
        let unwrap: Vec<_> = reopened
            .controls()
            .iter()
            .filter(|c| matches!(c.tag.as_deref(), Some("group" | "runs")))
            .map(|c| W::Unwrap(c.anchor.clone()))
            .collect();
        write(
            "native-richUnwrapped.docx",
            reopened
                .preview_edits(EditBatch {
                    wrappers: &unwrap,
                    ..Default::default()
                })
                .unwrap()
                .to_docx()
                .unwrap(),
        );
    }
    let ranged =
        ImportedDocument::open(&fs::read(directory.join("range-source.docx")).unwrap()).unwrap();
    {
        use rocx::editor::{EditBatch, InlineAction, InlineEdit, RunToggle};
        let edits = [
            (0, 0, RunToggle::Bold, Some(false)),
            (1, 0, RunToggle::Hidden, Some(true)),
            (1, 1, RunToggle::Strike, Some(true)),
            (1, 2, RunToggle::Italic, None),
        ]
        .map(|(body, slot, prop, value)| {
            let BodyBlock::Paragraph(p) = &ranged.body()[body] else {
                panic!("paragraph")
            };
            let Run(run) = &p.inline_structure[slot] else {
                panic!("run")
            };
            InlineEdit {
                target: run.anchor.clone(),
                action: InlineAction::SetToggle(prop, value),
            }
        });
        let mut edits = edits.to_vec();
        edits.push(InlineEdit {
            target: edits[2].target.clone(),
            action: InlineAction::PatchFormat {
                set: Box::new(RunFormat {
                    style: Some("Strong".into()),
                    font: Some("Cambria".into()),
                    font_east_asia: Some("Yu Mincho".into()),
                    font_complex_script: Some("Amiri".into()),
                    size: Some(12.5),
                    size_complex_script: Some(14.),
                    underline: Some(rocx::Underline::Double),
                    color: Some(rocx::Color::Theme {
                        name: rocx::ThemeColor::Accent2,
                        tint: Some(150),
                        shade: None,
                    }),
                    superscript: Some(true),
                    all_caps: Some(false),
                    small_caps: Some(true),
                    snap_to_grid: Some(false),
                    bold_complex_script: Some(true),
                    italic_complex_script: Some(false),
                    rtl: Some(true),
                    complex_script: Some(true),
                    language: Some(rocx::RunLanguage {
                        latin: Some("fr-CA".into()),
                        east_asia: Some("ja-JP".into()),
                        bidi: None,
                    }),
                    ..Default::default()
                }),
                clear: vec![rocx::editor::RunFormatField::LanguageBidi],
            },
        });
        let bytes = ranged
            .preview_edits(EditBatch {
                inline_edits: &edits,
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap();
        let reopened = ImportedDocument::open(&bytes).unwrap();
        let BodyBlock::Paragraph(p) = &reopened.body()[1] else {
            unreachable!()
        };
        let Run(run) = &p.inline_structure[1] else {
            unreachable!()
        };
        assert_eq!(run.format.size, Some(12.5));
        assert_eq!(run.format.underline, Some(rocx::Underline::Double));
        assert!(run.has_opaque_formatting);
        write("native-runFormatting.docx", bytes);
    }
    let selection = rocx::editor::TextRangeEdit {
        target: anchor(&ranged, 1),
        start: 1,
        end: 6,
        text: " 🐈 ".into(),
    };
    let point = |index, position, text: &str| rocx::editor::TextInsertion {
        target: anchor(&ranged, index),
        position,
        text: text.into(),
    };
    write(
        "native-points.docx",
        ranged
            .preview_edits(rocx::editor::EditBatch {
                text_insertions: &[
                    point(1, 3, "➡"),
                    point(1, 7, "!"),
                    point(ranged.body().len() - 1, 0, " Empty paragraph "),
                ],
                ..Default::default()
            })
            .unwrap()
            .to_docx()
            .unwrap(),
    );
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
        let open = |data: Vec<u8>| ImportedDocument::open(&data).unwrap();
        let original = open(fs::read(directory.join("control-choice-source.docx")).unwrap());
        let target = paragraph(&original, 0, 0);
        let marked = open(
            original
                .set_paired_range(90, Some("Guard"), Some((&target, &target)))
                .unwrap(),
        );
        assert!(marked.control_sites().iter().all(|s| s.text != "block"));
        assert!(
            marked
                .preview_tracked_bold(&paragraph(&marked, 0, 1), false, "Reviewer", None)
                .is_err()
        );
        let plain = open(marked.set_paired_range(90, Some("Guard"), None).unwrap());
        let changed = open(
            plain
                .preview_tracked_bold(&paragraph(&plain, 0, 0), false, "Reviewer", None)
                .unwrap()
                .to_docx()
                .unwrap(),
        );
        let BodyBlock::Paragraph(p) = &changed.body()[0] else {
            unreachable!()
        };
        let Run(r) = &p.inline_structure[0] else {
            unreachable!()
        };
        let id = r
            .property_revision
            .as_ref()
            .unwrap()
            .id
            .as_deref()
            .unwrap()
            .parse()
            .unwrap();
        let other = paragraph(&changed, 1, 0);
        assert!(
            changed
                .set_paired_range(id, Some("Other"), Some((&other, &other)))
                .is_err()
        );
        assert!(changed.control_sites().iter().all(|s| s.text != "block"));
        let reviewed = open(
            changed
                .preview_revision(&r.anchor, TextRevisionAction::Accept, None)
                .unwrap()
                .to_docx()
                .unwrap(),
        );
        let site = reviewed
            .control_sites()
            .iter()
            .find(|s| s.context.as_str() == "block" && s.text == "block")
            .unwrap();
        write(
            "native-guardedWorkflow.docx",
            reviewed
                .preview_wrap_control(site, "reviewed")
                .unwrap()
                .to_docx()
                .unwrap(),
        );
    }
    use rocx::editor::TrackedParagraph;
    let mut source =
        ImportedDocument::open(&fs::read(directory.join("base.docx")).unwrap()).unwrap();
    for (label, edit) in [
        (
            "Insert",
            TrackedParagraph::InsertBefore(" proposed ".into()),
        ),
        ("Delete", TrackedParagraph::Delete),
    ] {
        let bytes = source
            .preview_tracked_paragraph(&anchor(&source, 1), edit, "A & B", Some(date))
            .unwrap()
            .to_docx()
            .unwrap();
        let changed = ImportedDocument::open(&bytes).unwrap();
        write(&format!("native-para{label}.docx"), bytes);
        for (action, suffix) in [
            (TextRevisionAction::Accept, "Accepted"),
            (TextRevisionAction::Reject, "Rejected"),
        ] {
            let bytes = changed
                .preview_paragraph_revision(&anchor(&changed, 1), action)
                .unwrap()
                .to_docx()
                .unwrap();
            if action == TextRevisionAction::Accept {
                source = ImportedDocument::open(&bytes).unwrap();
            }
            write(&format!("native-para{label}{suffix}.docx"), bytes);
        }
    }
    let bold = doc
        .preview_tracked_bold(&run.anchor, false, "A & B", Some(date))
        .unwrap()
        .to_docx()
        .unwrap();
    let review = ImportedDocument::open(&bold).unwrap();
    write("native-boldChange.docx", bold);
    for (action, name) in [
        (TextRevisionAction::Accept, "native-boldAccepted.docx"),
        (TextRevisionAction::Reject, "native-boldRejected.docx"),
    ] {
        write(
            name,
            review
                .preview_revision(&paragraph(&review, 0, 0), action, None)
                .unwrap()
                .to_docx()
                .unwrap(),
        );
    }
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
            FieldEdit {
                target: anchor(&d, 4),
                inline_index: 1,
                action: FieldAction::Patch(FieldPatch::Dirty(Some(false))),
            },
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
        let edits = [
            InlineEdit {
                target: link(&d, 0),
                action: InlineAction::HyperlinkTooltip(Some("Batch tooltip".into())),
            },
            InlineEdit {
                target: link(&d, 0),
                action: InlineAction::HyperlinkHistory(Some(false)),
            },
        ];
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
                action: InlineAction::SetToggle(rocx::editor::RunToggle::Bold, None),
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
        Layout(None),
        Layout(Some(rocx::TableLayout::Autofit)),
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
