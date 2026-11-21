use futures::executor::block_on;
use rocx::*;

#[test]
fn builders_set_every_property_without_changing_struct_literal_semantics() {
    let language = RunLanguage {
        latin: Some("en-US".into()),
        ..Default::default()
    };
    let built = RunFormat::default()
        .with_style("Strong")
        .with_bold(false)
        .with_italic(true)
        .with_underline(Underline::Double)
        .with_font(String::from("Arial"))
        .with_size(14.)
        .with_color(Color::Auto)
        .with_superscript(false)
        .with_subscript(true)
        .with_strike(false)
        .with_all_caps(true)
        .with_small_caps(false)
        .with_language(language.clone())
        .with_bold_complex_script(true)
        .with_italic_complex_script(false)
        .with_size_complex_script(16.)
        .with_rtl(true)
        .with_complex_script(false)
        .with_font_east_asia("Yu Mincho")
        .with_font_complex_script("Arial");
    assert_eq!(
        built,
        RunFormat {
            style: Some("Strong".into()),
            bold: Some(false),
            italic: Some(true),
            underline: Some(Underline::Double),
            font: Some("Arial".into()),
            size: Some(14.),
            color: Some(Color::Auto),
            superscript: Some(false),
            subscript: Some(true),
            strike: Some(false),
            all_caps: Some(true),
            small_caps: Some(false),
            language: Some(language),
            bold_complex_script: Some(true),
            italic_complex_script: Some(false),
            size_complex_script: Some(16.),
            rtl: Some(true),
            complex_script: Some(false),
            font_east_asia: Some("Yu Mincho".into()),
            font_complex_script: Some("Arial".into()),
        }
    );
}

#[test]
fn bold_builder_preserves_three_states_other_fields_and_last_assignment() {
    assert_eq!(RunFormat::default().bold, None);
    let format = RunFormat {
        italic: Some(true),
        ..Default::default()
    };
    let enabled = format.with_bold(true);
    assert_eq!(enabled.bold, Some(true));
    assert_eq!(enabled.italic, Some(true));
    let disabled = enabled.with_bold(false);
    assert_eq!(disabled.bold, Some(false));
    assert_eq!(disabled.italic, Some(true));
    assert_eq!(disabled.font, None);
}

#[test]
fn builder_and_literal_formats_produce_identical_docx_bytes() {
    let bytes = |formats: Vec<RunFormat>| {
        let runs = formats
            .into_iter()
            .map(|format| {
                TextRun::new("Participant (REDCap ID): ")
                    .with_format(format)
                    .into()
            })
            .collect();
        block_on(
            Document::new(vec![
                Section::new(vec![Paragraph::from_children(runs).into()]).into(),
            ])
            .with_timestamp(Some("2026-11-20T00:00:00Z".into()))
            .resolve(),
        )
        .unwrap()
        .to_docx()
        .unwrap()
    };
    assert_eq!(
        bytes(vec![
            RunFormat::default(),
            RunFormat::default().with_bold(true),
            RunFormat::default().with_bold(false).with_italic(true)
        ]),
        bytes(vec![
            RunFormat::default(),
            RunFormat {
                bold: Some(true),
                ..Default::default()
            },
            RunFormat {
                bold: Some(false),
                italic: Some(true),
                ..Default::default()
            }
        ])
    );
}
