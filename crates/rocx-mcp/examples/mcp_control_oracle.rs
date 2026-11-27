use rocx::editor::{
    BodyBlock, ControlContext, ControlEdit, EditBatch, ImportedControl, ImportedDocument,
    ImportedInlineNode, RunTextEdit, WrapperEdit,
};
use rocx::{
    Alignment, AltText, BookmarkName, BreakType, Component, ContentControlListItem,
    ContentControlLock, ContentControlProperties, ContentControlText, Hyperlink,
    HyperlinkProperties, HyperlinkTarget, ImageData, ImageRun, ImageTransformation, Paragraph,
    ParagraphFormat, ResolvedBlock, ResolvedInline, RowHeight, RunFormat, Table, TableAlignment,
    TableCell, TableCellProperties, TableLayout, TableProperties, TableRow, TableRowProperties,
    TextRun, VerticalAlignment,
};
use serde_json::{Value, json};

fn findings(controls: &[ImportedControl]) -> Vec<Value> {
    controls
        .iter()
        .map(|c| {
            json!({
                "context": c.context.as_str(),
                "id": c.id, "tag": c.tag, "alias": c.alias, "lock": c.lock,
                "bound": c.bound, "kind": c.kind, "selected": c.selected, "multi_line": c.multi_line,
                "choices": c.choices.iter().map(|choice|
                    json!({"display_text": choice.display_text, "value": choice.value})
                ).collect::<Vec<_>>(),
                "owner_part": c.owner_part, "entry_id": c.entry_id, "entry_index": c.entry_index,
            })
        })
        .collect()
}

fn rich_link(target: HyperlinkTarget) -> rocx::Result<ResolvedInline> {
    Ok(futures_executor::block_on(
        Hyperlink::new(
            target,
            vec![
                TextRun::new("Read the guide")
                    .with_format(RunFormat::default().with_italic(true))
                    .into(),
            ],
        )
        .with_properties(HyperlinkProperties {
            tooltip: Some("Control guide".into()),
            history: Some(false),
        })
        .resolve(),
    )?
    .unwrap())
}

fn rich_paragraph(link: ResolvedInline) -> rocx::Result<ResolvedBlock> {
    Ok(ResolvedBlock::Paragraph(
        futures_executor::block_on(
            Paragraph::from_children(vec![link.into()])
                .with_format(ParagraphFormat::default().with_alignment(Alignment::Center))
                .resolve(),
        )?
        .unwrap(),
    ))
}

fn rich_replacement(doc: &ImportedDocument, inline: usize, block: usize) -> rocx::Result<Vec<u8>> {
    let picture = ImageRun::new(ImageData::Bytes(
        include_bytes!("../../../tests/fixtures/images/sample.png").to_vec(),
    ))
    .with_transformation(Some(ImageTransformation::default().with_width(24.)))
    .with_alt_text(Some(AltText::default().with_description("Control logo")));
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("Cell text")
                        .with_format(RunFormat::default().with_bold(true))
                        .into(),
                ])
                .into(),
            ])
            .with_properties(
                TableCellProperties::default()
                    .with_background_color([0x11, 0x22, 0x33])
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .into(),
        ])
        .with_properties(
            TableRowProperties::default()
                .with_repeat_header(true)
                .with_height(RowHeight::Exact(14.)),
        )
        .into(),
    ])
    .with_properties(
        TableProperties::default()
            .with_column_widths(vec![1440])
            .with_layout(TableLayout::Fixed)
            .with_alignment(TableAlignment::Right),
    );
    doc.preview_edits(EditBatch {
        wrappers: &[
            WrapperEdit::ReplaceInlines(
                doc.controls()[inline].anchor.clone(),
                vec![
                    ResolvedInline::Tab {
                        format: RunFormat::default().with_bold(false),
                        comment: None,
                    },
                    futures_executor::block_on(picture.resolve())?.unwrap(),
                    ResolvedInline::Break {
                        kind: BreakType::Line,
                        format: RunFormat::default().with_italic(true),
                        comment: None,
                    },
                ],
            ),
            WrapperEdit::ReplaceBlocks(
                doc.controls()[block].anchor.clone(),
                vec![
                    rich_paragraph(rich_link(HyperlinkTarget::external(
                        "https://example.org/control",
                    )?)?)?,
                    ResolvedBlock::Table(futures_executor::block_on(table.resolve())?.unwrap()),
                ],
            ),
        ],
        ..Default::default()
    })?
    .to_docx()
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    let target = |index: usize| doc.controls()[index].anchor.clone();
    let site = |context, text| {
        doc.control_sites()
            .iter()
            .find(|s| s.context == context && s.text == text)
            .unwrap()
            .clone()
    };
    let choices = || {
        ["Alpha", "Gamma"]
            .into_iter()
            .map(|s| ContentControlListItem {
                display_text: s.into(),
                value: s.into(),
            })
            .collect()
    };
    let rich_run = || -> rocx::Result<_> {
        Ok(futures_executor::block_on(
            TextRun::new("new rich text")
                .with_format(RunFormat::default().with_bold(true))
                .resolve(),
        )?
        .unwrap())
    };
    let preview = match action.as_str() {
        "inspect" => {
            std::fs::write(
                output,
                json!({
                    "controls": findings(doc.controls()),
                    "body_controls": doc.body().iter().enumerate().filter_map(|(index, block)| {
                        let BodyBlock::ContentControl(control) = block else { return None; };
                        Some(json!({
                            "type": "content-control", "index": index, "tag": control.tag,
                            "alias": control.alias, "lock": control.lock, "has_opaque_content": true,
                        }))
                    }).collect::<Vec<_>>(),
                    "story_controls": findings(&doc.story_controls()?),
                    "control_sites": doc.control_sites().iter().map(|site| json!({
                        "context": site.context.as_str(), "text": site.text,
                    })).collect::<Vec<_>>(),
                })
                .to_string(),
            )
            .unwrap();
            return Ok(());
        }
        "promote-and-define" => doc.preview_edits(EditBatch {
            wrappers: &[
                WrapperEdit::Unwrap(target(0)),
                WrapperEdit::Unwrap(target(1)),
                WrapperEdit::Unwrap(target(2)),
                WrapperEdit::Define(
                    target(3),
                    ContentControlProperties {
                        tag: Some("rich".into()),
                        lock: Some(ContentControlLock::Unlocked),
                        text: ContentControlText::Rich,
                        ..Default::default()
                    },
                ),
                WrapperEdit::Define(
                    target(4),
                    ContentControlProperties {
                        id: Some(43),
                        tag: Some("dropdown".into()),
                        text: ContentControlText::DropDown {
                            items: [("First label", "one"), ("Second label", "two")]
                                .into_iter()
                                .map(|(display_text, value)| ContentControlListItem {
                                    display_text: display_text.into(),
                                    value: value.into(),
                                })
                                .collect(),
                            selected: "one".into(),
                        },
                        ..Default::default()
                    },
                ),
            ],
            ..Default::default()
        })?,
        "select" => doc.preview_control_choice(&target(1), "two", None)?,
        "locks" => {
            let wrappers: Vec<_> = [
                (ContentControlLock::ControlLocked, Some(true)),
                (ContentControlLock::ContentLocked, None),
                (ContentControlLock::ControlAndContentLocked, Some(false)),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (lock, multi_line))| {
                WrapperEdit::Define(
                    target(i + 10),
                    ContentControlProperties {
                        id: None,
                        tag: Some(format!("lock-{i}")),
                        alias: Some(format!("Alias {i} & Café")),
                        lock: Some(lock),
                        text: ContentControlText::Plain { multi_line },
                    },
                )
            })
            .collect();
            doc.preview_edits(EditBatch {
                wrappers: &wrappers,
                ..Default::default()
            })?
        }
        "locked-define" => {
            let control = &doc.controls()[10].anchor;
            let error = doc
                .preview_replace_control(control, &ContentControlProperties::default())
                .expect_err("locked definition must be rejected");
            std::fs::write(output, error.to_string()).unwrap();
            return Ok(());
        }
        "rich-replacement" => {
            std::fs::write(output, rich_replacement(&doc, 1, 4)?)?;
            return Ok(());
        }
        "inline-link" | "block-bookmark-link" => {
            let wrapper = if action == "inline-link" {
                WrapperEdit::ReplaceInlines(
                    target(1),
                    vec![rich_link(HyperlinkTarget::external(
                        "https://example.org/control",
                    )?)?],
                )
            } else {
                WrapperEdit::ReplaceBlocks(
                    target(4),
                    vec![rich_paragraph(rich_link(HyperlinkTarget::bookmark(
                        BookmarkName::new("Destination")?,
                    ))?)?],
                )
            };
            let error = doc
                .preview_edits(EditBatch {
                    wrappers: &[wrapper],
                    ..Default::default()
                })
                .expect_err("link must be rejected in this control scope");
            std::fs::write(output, error.to_string())?;
            return Ok(());
        }
        "mixed" => {
            let BodyBlock::Paragraph(outside) = &doc.body()[10] else {
                panic!("expected ordinary paragraph")
            };
            let ImportedInlineNode::Run(outside) = &outside.inline_structure[0] else {
                panic!("expected ordinary run")
            };
            doc.preview_edits(EditBatch {
                controls: &[ControlEdit {
                    target: target(0),
                    tag: Some("patched".into()),
                    alias: Some("New alias".into()),
                    text: Some("updated plain".into()),
                }],
                wrappers: &[
                    WrapperEdit::ReplaceInlines(target(1), vec![rich_run()?]),
                    WrapperEdit::Choice {
                        target: target(2),
                        selected: "Gamma".into(),
                        items: Some(choices()),
                    },
                    WrapperEdit::Unwrap(target(3)),
                    WrapperEdit::ReplaceBlocks(
                        target(4),
                        vec![ResolvedBlock::Paragraph(
                            futures_executor::block_on(Paragraph::new("new block").resolve())?
                                .unwrap(),
                        )],
                    ),
                    WrapperEdit::Define(
                        target(5),
                        ContentControlProperties {
                            id: Some(42),
                            text: ContentControlText::Plain {
                                multi_line: Some(false),
                            },
                            ..Default::default()
                        },
                    ),
                ],
                run_text: &[RunTextEdit {
                    target: outside.anchor.clone(),
                    text: "edited outside".into(),
                }],
                ..Default::default()
            })?
        }
        "block" => doc.preview_edits(EditBatch {
            wrappers: &[WrapperEdit::Wrap {
                first: site(ControlContext::Block, "block first"),
                last: Some(site(ControlContext::Block, "block last")),
                tag: "wrapped".into(),
            }],
            ..Default::default()
        })?,
        "inline" => {
            doc.preview_wrap_control(&site(ControlContext::Inline, "block first"), "wrapped")?
        }
        "row" => doc.preview_wrap_control(&site(ControlContext::Row, "table text"), "wrapped")?,
        "cell" => doc.preview_wrap_control(&site(ControlContext::Cell, "table text"), "wrapped")?,
        _ => panic!("unknown control oracle action"),
    };
    std::fs::write(output, preview.to_docx()?).unwrap();
    Ok(())
}
