//! Rich external links without fetching their targets.
use rv_docx_rs::*;

fn main() -> Result<()> {
    let output = std::env::args().nth(1).unwrap_or("hyperlinks.docx".into());
    let document = futures::executor::block_on(async {
        let target = Property::new(
            "link.target",
            Value::try_future(async {
                HyperlinkTarget::external("https://example.invalid/λ?q=a%20b&lang=日本#résumé")
                    .map_err(Error::from)
            }),
        );
        let link = Hyperlink::new(
            target.reference(),
            vec![
                TextRun::new("Read ")
                    .with_format(RunFormat {
                        bold: Some(true),
                        ..Default::default()
                    })
                    .into(),
                TextRun::new("more λ")
                    .with_format(RunFormat {
                        underline: Some(Underline::Single),
                        ..Default::default()
                    })
                    .into(),
            ],
        )
        .with_properties(HyperlinkProperties {
            tooltip: Some("External resource".into()),
            history: Some(true),
        });
        Document::new(vec![
            Section::new(vec![Paragraph::from_children(vec![link.into()]).into()]).into(),
        ])
        .resolve()
        .await
    })?;
    document.save(output)
}
