//! Direct Rust byte oracle for the independently authored layout fixtures.
use rocx::editor::{
    BodyBlock, ImportedDocument, LayoutBorder, LayoutEdit, LayoutField, LayoutTab, LayoutValue,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use LayoutField::*;

    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let (source, output, mode) = match args.as_slice() {
        [source, output] => (source, output, "mixed"),
        [source, output, mode] => (source, output, mode.to_str().ok_or("invalid mode")?),
        _ => {
            return Err(
                "usage: mcp_layout_oracle SOURCE OUTPUT [empty|set|clear|guarded-noop|outline]"
                    .into(),
            );
        }
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    if mode == "empty" {
        std::fs::write(output, document.preview_layout(&[])?.to_docx()?)?;
        return Ok(());
    }
    let [
        BodyBlock::Table(_),
        BodyBlock::Paragraph(paragraph),
        BodyBlock::Paragraph(break_paragraph),
    ] = document.body()
    else {
        return Err("expected table-first fixture with two body paragraphs".into());
    };
    if mode == "outline" {
        std::fs::write(
            output,
            document
                .preview_layout(&[LayoutEdit::Paragraph(
                    paragraph.anchor.clone(),
                    OutlineLevel,
                    Some(LayoutValue::Integer(4)),
                )])?
                .to_docx()?,
        )?;
        return Ok(());
    }
    let sections = document.sections()?;
    let paragraph_edit =
        |field, value| LayoutEdit::Paragraph(paragraph.anchor.clone(), field, value);
    let section_edit = |index: usize, field, value| {
        LayoutEdit::Section(sections[index].anchor.clone(), field, value)
    };
    let break_edit =
        |field, value| LayoutEdit::Paragraph(break_paragraph.anchor.clone(), field, value);
    let int = |v| Some(LayoutValue::Integer(v));
    let flag = |v| Some(LayoutValue::Boolean(v));
    let token = |v: &str| Some(LayoutValue::Token(v.into()));
    let border = |size, space, color: Option<&str>| {
        Some(LayoutValue::Border(LayoutBorder {
            size,
            space,
            color: color.map(str::to_owned),
        }))
    };
    let edits = match mode {
        "mixed" => vec![
            paragraph_edit(Alignment, token("center")),
            paragraph_edit(FirstLineIndent, int(-240)),
            paragraph_edit(KeepWithNext, flag(false)),
            paragraph_edit(WidowControl, None),
            paragraph_edit(
                TabStops,
                Some(LayoutValue::Tabs(vec![LayoutTab {
                    position: 720,
                    alignment: "right".into(),
                    leader: Some("dot".into()),
                }])),
            ),
            section_edit(0, TitlePage, None),
            section_edit(1, MarginTop, int(900)),
            section_edit(1, BorderBottom, border(Some(8), Some(4), Some("336699"))),
        ],
        "set" => {
            let mut edits: Vec<_> = [
                (StyleId, token("Title")),
                (KeepWithNext, flag(false)),
                (KeepTogether, flag(true)),
                (PageBreakBefore, flag(true)),
                (WidowControl, flag(false)),
                (SuppressLineNumbers, flag(true)),
                (ParagraphBidi, flag(true)),
                (SnapToGrid, flag(false)),
                (ContextualSpacing, flag(true)),
                (MirrorIndents, flag(true)),
                (Alignment, token("both")),
                (OutlineLevel, int(4)),
                (SpaceBefore, int(120)),
                (SpaceAfter, int(240)),
                (LineHeight, int(300)),
                (LineRule, token("exact")),
                (LeftIndent, int(360)),
                (RightIndent, int(-120)),
                (FirstLineIndent, int(180)),
                (
                    TabStops,
                    Some(LayoutValue::Tabs(vec![
                        LayoutTab {
                            position: 480,
                            alignment: "left".into(),
                            leader: None,
                        },
                        LayoutTab {
                            position: 960,
                            alignment: "decimal".into(),
                            leader: Some("hyphen".into()),
                        },
                    ])),
                ),
            ]
            .into_iter()
            .map(|(f, v)| paragraph_edit(f, v))
            .collect();
            edits.push(break_edit(Heading, int(3)));
            edits.extend([
                section_edit(0, SectionType, token("continuous")),
                section_edit(0, TitlePage, flag(false)),
            ]);
            edits.extend(
                [
                    (PageWidth, int(16000)),
                    (PageHeight, int(10000)),
                    (Orientation, token("landscape")),
                    (MarginTop, int(-600)),
                    (MarginBottom, int(-720)),
                    (MarginLeft, int(1000)),
                    (MarginRight, int(1100)),
                    (MarginHeader, int(400)),
                    (MarginFooter, int(500)),
                    (MarginGutter, int(120)),
                    (ColumnCount, int(3)),
                    (ColumnSpace, int(360)),
                    (ColumnSeparator, flag(true)),
                    (GridMode, token("linesAndChars")),
                    (GridPitch, int(320)),
                    (GridCharSpace, int(-40)),
                    (PageStart, int(7)),
                    (PageFormat, token("lowerRoman")),
                    (LineStart, int(6)),
                    (LineCountBy, int(2)),
                    (LineDistance, int(180)),
                    (LineRestart, token("newSection")),
                    (VerticalAlign, token("center")),
                    (Bidi, flag(true)),
                    (RtlGutter, flag(true)),
                    (BorderOffset, token("text")),
                    (BorderDisplay, token("firstPage")),
                    (BorderZOrder, token("back")),
                    (BorderTop, border(Some(12), None, Some("auto"))),
                    (BorderLeft, border(Some(8), Some(4), Some("336699"))),
                    (BorderBottom, border(None, None, None)),
                    (BorderRight, border(None, Some(6), None)),
                ]
                .into_iter()
                .map(|(f, v)| section_edit(1, f, v)),
            );
            edits
        }
        "clear" => vec![
            paragraph_edit(LineRule, None),
            paragraph_edit(RightIndent, None),
            paragraph_edit(TabStops, None),
            break_edit(Heading, None),
            section_edit(0, Margins, None),
            section_edit(1, LineNumbering, None),
            section_edit(1, BorderDisplay, None),
            section_edit(1, BorderLeft, None),
        ],
        "guarded-noop" => vec![
            paragraph_edit(KeepWithNext, flag(true)),
            paragraph_edit(Heading, None),
            section_edit(1, BorderTop, border(Some(8), None, Some("336699"))),
        ],
        _ => return Err("unknown layout oracle mode".into()),
    };
    let preview = document.preview_layout(&edits)?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
