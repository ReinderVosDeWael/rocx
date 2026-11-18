use futures::executor::block_on;
use rv_docx_rs::*;
#[test]
fn compilation_is_deterministic_and_inspectable_with_explicit_inputs() {
    let snapshot = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new("pure compilation").into()]).into(),
        ])
        .resolve(),
    )
    .unwrap();
    let options = application::RenderOptions {
        timestamp: Some("2026-01-01T00:00:00Z".into()),
        ..Default::default()
    };
    let first = application::compile(
        &snapshot,
        include_bytes!("../assets/default.docx"),
        &options,
    )
    .unwrap();
    let second = application::compile(
        &snapshot,
        include_bytes!("../assets/default.docx"),
        &options,
    )
    .unwrap();
    assert_eq!(first.to_bytes().unwrap(), second.to_bytes().unwrap());
    assert!(
        first
            .part(&opc::PartName::new("word/document.xml").unwrap())
            .unwrap()
            .windows(16)
            .any(|w| w == b"pure compilation")
    );
    assert!(first.parts().count() > 5);
    assert!(
        application::compile(
            &snapshot,
            include_bytes!("../assets/default.docx"),
            &application::RenderOptions {
                profile: OutputProfile::Strict,
                ..options
            }
        )
        .is_err()
    );
}
#[test]
fn formatting_can_depend_on_another_named_async_property() {
    let format = Property::new(
        "source.format",
        Value::future(async {
            RunFormat {
                bold: Some(true),
                size: Some(18.),
                color: Some(Color::Theme {
                    name: ThemeColor::Accent1,
                    tint: Some(0x80),
                    shade: None,
                }),
                underline: Some(Underline::Double),
                ..Default::default()
            }
        }),
    );
    let reference = format.reference();
    let text = Value::try_future(async move {
        Ok::<_, Error>(format!("Size: {}", reference.await?.size.unwrap()))
    });
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::new(text).into(),
            Paragraph::from_children(vec![TextRun::new("source").with_format(format).into()])
                .into(),
        ])
        .into(),
    ]);
    let snapshot = block_on(document.resolve()).unwrap();
    assert_eq!(snapshot.sections()[0].paragraphs()[0].text(), "Size: 18");
    let run = &snapshot.sections()[0].paragraphs()[1].runs()[0];
    assert_eq!(run.format().underline, Some(Underline::Double));
    assert!(matches!(
        run.format().color,
        Some(Color::Theme {
            name: ThemeColor::Accent1,
            ..
        })
    ));
}
