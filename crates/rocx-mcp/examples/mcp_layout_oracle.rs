//! Direct Rust byte oracle for the independently authored layout fixture.
use rocx::editor::{
    BodyBlock, ImportedDocument, LayoutBorder, LayoutEdit, LayoutField, LayoutTab, LayoutValue,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let (source, output, empty) = match args.as_slice() {
        [source, output] => (source, output, false),
        [source, output, mode] if mode == "empty" => (source, output, true),
        _ => return Err("usage: mcp_layout_oracle SOURCE OUTPUT [empty]".into()),
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    if empty {
        std::fs::write(output, document.preview_layout(&[])?.to_docx()?)?;
        return Ok(());
    }
    let [
        BodyBlock::Table(_),
        BodyBlock::Paragraph(paragraph),
        BodyBlock::Paragraph(_),
    ] = document.body()
    else {
        return Err("expected table-first fixture with two body paragraphs".into());
    };
    let sections = document.sections()?;
    let paragraph_edit =
        |field, value| LayoutEdit::Paragraph(paragraph.anchor.clone(), field, value);
    let section_edit = |index: usize, field, value| {
        LayoutEdit::Section(sections[index].anchor.clone(), field, value)
    };
    let preview = document.preview_layout(&[
        paragraph_edit(
            LayoutField::Alignment,
            Some(LayoutValue::Token("center".into())),
        ),
        paragraph_edit(
            LayoutField::FirstLineIndent,
            Some(LayoutValue::Integer(-240)),
        ),
        paragraph_edit(LayoutField::KeepWithNext, Some(LayoutValue::Boolean(false))),
        paragraph_edit(LayoutField::WidowControl, None),
        paragraph_edit(
            LayoutField::TabStops,
            Some(LayoutValue::Tabs(vec![LayoutTab {
                position: 720,
                alignment: "right".into(),
                leader: Some("dot".into()),
            }])),
        ),
        section_edit(0, LayoutField::TitlePage, None),
        section_edit(1, LayoutField::MarginTop, Some(LayoutValue::Integer(900))),
        section_edit(
            1,
            LayoutField::BorderBottom,
            Some(LayoutValue::Border(LayoutBorder {
                size: Some(8),
                space: Some(4),
                color: Some("336699".into()),
            })),
        ),
    ])?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
