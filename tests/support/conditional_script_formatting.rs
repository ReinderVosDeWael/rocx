use rv_docx_rs::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![]);
    document.styles = Value::future(async {
        vec![
            TableStyleDefinition {
                name: "Scripts".into(),
                first_row: Some(TableSectionFormat {
                    language: Some(RunLanguage {
                        latin: Some("en-US".into()),
                        east_asia: Some("ja-JP".into()),
                        bidi: Some("ar-SA".into()),
                    }),
                    bold_complex_script: Some(false),
                    italic_complex_script: Some(true),
                    size_complex_script: Some(13.5),
                    rtl: Some(false),
                    complex_script: Some(true),
                    font_east_asia: Some("East".into()),
                    font_complex_script: Some("Complex".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }
            .into(),
        ]
    });
    document
}
