use futures::executor::block_on;
use rocx::*;
use std::path::Path;

pub fn write(out: &Path) -> Result<()> {
    let heading = BookmarkName::new("Summary")?;
    let block = BlockContentControl::new(vec![Paragraph::new("block").into()]).with_properties(
        ContentControlProperties::default()
            .with_id(10)
            .with_tag("block"),
    );
    let link = Hyperlink::new(
        HyperlinkTarget::bookmark(heading.clone()),
        vec![TextRun::new("link").into()],
    )
    .with_properties(HyperlinkProperties::default().with_history(false));
    let field = Field::new(FieldInstruction::Page, vec![TextRun::new("3").into()]).with_properties(
        FieldProperties {
            form: FieldForm::Complex,
            dirty: Some(false),
            locked: Some(true),
        },
    );
    let dropdown = InlineContentControl::new(vec![TextRun::new("Beta").into()]).with_properties(
        ContentControlProperties {
            id: Some(11),
            text: ContentControlText::DropDown {
                items: vec![
                    ContentControlListItem {
                        display_text: "Alpha".into(),
                        value: "a".into(),
                    },
                    ContentControlListItem {
                        display_text: "Beta".into(),
                        value: "b".into(),
                    },
                ],
                selected: "b".into(),
            },
            ..Default::default()
        },
    );
    let tracked = TextRun::new("tracked").with_revision(Some(TextRevision {
        kind: RevisionKind::Insertion,
        author: "Ada".into(),
        date: None,
    }));
    let paragraph = Paragraph::from_children(vec![
        RangeStart::bookmark(heading.clone()).into(),
        TextRun::new("marker").into(),
        RangeEnd::bookmark(&heading).into(),
        link.into(),
        field.into(),
        dropdown.into(),
        tracked.into(),
    ]);
    let declaration = Document::new(vec![
        Section::new(vec![block.into(), paragraph.into()]).into(),
    ]);
    let snapshot = block_on(
        declaration
            .with_timestamp(Some("2024-02-29T01:02:03Z".into()))
            .resolve(),
    )?;
    std::fs::write(out.join("structured.docx"), snapshot.to_docx()?)?;
    Ok(())
}
