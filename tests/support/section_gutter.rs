use rv_docx_rs::*;

pub fn document() -> Document {
    let mut first = Section::new(vec![Paragraph::new("on").into()]);
    first.properties = Value::future(async {
        Some(SectionProperties {
            rtl_gutter: Some(true),
            bidi: Some(false),
            ..Default::default()
        })
    });
    let mut last = Section::new(vec![Paragraph::new("off").into()]);
    last.properties = Some(SectionProperties {
        rtl_gutter: Some(false),
        ..Default::default()
    })
    .into();
    Document::new(vec![
        first.into(),
        Section::new(vec![Paragraph::new("inherited").into()]).into(),
        last.into(),
    ])
}
