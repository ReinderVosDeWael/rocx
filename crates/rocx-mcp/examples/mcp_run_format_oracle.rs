//! Direct native byte oracle for the independent MCP run-format fixture.
use rocx::{
    Color, RunFormat, RunLanguage, ThemeColor, Underline,
    editor::{
        BodyBlock, ImportedDocument, ImportedInlineNode, ImportedRowChild, ImportedRun,
        ImportedTableChild, InlineAction, InlineEdit, RunFormatField, RunToggle,
    },
};

fn link_run(inlines: &[ImportedInlineNode]) -> &ImportedRun {
    let link = inlines
        .iter()
        .find_map(|node| match node {
            ImportedInlineNode::Hyperlink(link) => Some(link),
            _ => None,
        })
        .expect("fixture hyperlink");
    let [ImportedInlineNode::Run(run)] = link.children.as_slice() else {
        panic!("expected one hyperlink run");
    };
    run
}

fn edit(run: &ImportedRun, action: InlineAction) -> InlineEdit {
    InlineEdit {
        target: run.anchor.clone(),
        action,
    }
}

fn link_edits(run: &ImportedRun) -> [InlineEdit; 2] {
    [
        edit(run, InlineAction::SetToggle(RunToggle::Hidden, None)),
        edit(
            run,
            InlineAction::PatchFormat {
                set: Box::new(RunFormat {
                    italic: Some(false),
                    color: Some(Color::Rgb([17, 51, 119])),
                    ..RunFormat::default()
                }),
                clear: vec![RunFormatField::Bold],
            },
        ),
    ]
}

fn body_target(doc: &ImportedDocument) -> (&ImportedRun, &[ImportedInlineNode]) {
    let BodyBlock::Paragraph(paragraph) = &doc.body()[1] else {
        panic!("expected fixture body paragraph");
    };
    let ImportedInlineNode::Run(run) = &paragraph.inline_structure[0] else {
        panic!("expected fixture body run");
    };
    (run, &paragraph.inline_structure)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [source, output, kind] = args.as_slice() else {
        return Err("usage: mcp_run_format_oracle SOURCE OUTPUT KIND".into());
    };
    let doc = ImportedDocument::open(&std::fs::read(source)?)?;
    let kind = kind.to_str().ok_or("expected UTF-8 owner kind")?;
    let bytes = match kind {
        "body" => {
            let (run, inlines) = body_target(&doc);
            let BodyBlock::Table(table) = &doc.body()[2] else {
                return Err("expected fixture table".into());
            };
            let [ImportedTableChild::Row(row)] = table.children.as_slice() else {
                return Err("expected one fixture row".into());
            };
            let [ImportedRowChild::Cell(cell)] = row.children.as_slice() else {
                return Err("expected one fixture cell".into());
            };
            let mut edits = vec![
                edit(run, InlineAction::SetBold(false)),
                edit(run, InlineAction::SetToggle(RunToggle::Hidden, Some(true))),
                edit(
                    run,
                    InlineAction::PatchFormat {
                        set: Box::new(RunFormat {
                            style: Some("Emphasis".into()),
                            font: Some("Finn".into()),
                            size: Some(13.5),
                            size_complex_script: Some(14.0),
                            italic_complex_script: Some(false),
                            bold_complex_script: Some(true),
                            font_east_asia: Some("Ming".into()),
                            font_complex_script: Some("Naskh".into()),
                            all_caps: Some(false),
                            small_caps: Some(true),
                            strike: Some(false),
                            snap_to_grid: Some(false),
                            underline: Some(Underline::Wave),
                            color: Some(Color::Theme {
                                name: ThemeColor::Accent2,
                                tint: Some(17),
                                shade: None,
                            }),
                            subscript: Some(true),
                            rtl: Some(false),
                            complex_script: Some(true),
                            language: Some(RunLanguage {
                                latin: Some("en-GB".into()),
                                east_asia: Some("ja-JP".into()),
                                bidi: Some("ar".into()),
                            }),
                            ..RunFormat::default()
                        }),
                        clear: vec![RunFormatField::Italic],
                    },
                ),
            ];
            edits.extend(link_edits(link_run(inlines)));
            edits.extend(link_edits(link_run(cell.inline_structure(0).unwrap())));
            doc.edit_run_format(None, &edits)?
        }
        "body-clear" => {
            use RunFormatField::*;
            doc.edit_run_format(
                None,
                &[edit(
                    body_target(&doc).0,
                    InlineAction::PatchFormat {
                        set: Box::default(),
                        clear: vec![
                            Style,
                            Font,
                            FontEastAsia,
                            FontComplexScript,
                            Bold,
                            BoldComplexScript,
                            Italic,
                            ItalicComplexScript,
                            AllCaps,
                            SmallCaps,
                            Strike,
                            SnapToGrid,
                            Hidden,
                            Color,
                            Size,
                            SizeComplexScript,
                            Underline,
                            VerticalAlignment,
                            Rtl,
                            ComplexScript,
                            LanguageLatin,
                            LanguageEastAsia,
                            LanguageBidi,
                        ],
                    },
                )],
            )?
        }
        "header" | "footer" => {
            let stories = doc.stories()?;
            let story = stories
                .iter()
                .find(|story| story.section_index == 0 && story.kind.as_str() == kind)
                .ok_or("missing fixture story")?;
            doc.edit_run_format(
                Some(&story.part_name),
                &link_edits(link_run(story.inline_structure(0).unwrap())),
            )?
        }
        "footnote" | "endnote" | "comment" => {
            let entries: Vec<_> = doc
                .entry_stories()?
                .iter()
                .filter(|entry| entry.kind.as_str() == kind && entry.note_type.is_none())
                .collect();
            let owner = &entries.first().ok_or("missing fixture entry")?.part_name;
            let edits: Vec<_> = entries
                .iter()
                .flat_map(|entry| link_edits(link_run(entry.inline_structure(0).unwrap())))
                .collect();
            doc.edit_run_format(Some(owner), &edits)?
        }
        _ => return Err("unknown fixture owner kind".into()),
    };
    std::fs::write(output, bytes)?;
    Ok(())
}
