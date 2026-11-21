//! Deferred aggregates and immutable child reuse, with an explicit render policy.
use rocx::*;

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("resolution-options.docx".into());
    let snapshot = futures::executor::block_on(async {
        let children = Property::new(
            "children",
            Value::try_future(async {
                let paragraph = Paragraph::new("Shared resolved content λ")
                    .resolve()
                    .await?
                    .unwrap();
                Ok::<_, Error>(vec![ResolvedBlock::Paragraph(paragraph)])
            }),
        );
        let content = |reference| {
            Value::from(reference).map(|blocks: Vec<ResolvedBlock>| {
                blocks
                    .into_iter()
                    .map(Into::into)
                    .collect::<BlockChildren>()
            })
        };
        let mut section = Section::default().with_children(content(children.reference()));
        section.properties = Property::new(
            "section.properties",
            Value::future(async {
                Some(SectionProperties {
                    columns: Some(Columns {
                        count: Some(2),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            }),
        )
        .into();
        section.headers.insert(
            HeaderFooterType::Default,
            Header {
                children: content(children.reference()),
                ..Default::default()
            }
            .into(),
        );
        let mut document = Document::new(vec![section.into()]);
        document.metadata = Property::new(
            "metadata",
            Value::future(async {
                Metadata {
                    title: Some("Resolution and rendering".into()),
                    ..Default::default()
                }
            }),
        )
        .into();
        document.resolve().await
    })?;
    snapshot.save_with_options(
        output,
        &application::RenderOptions {
            timestamp: Some("2026-11-18T00:00:00Z".into()),
            ..Default::default()
        },
    )
}
