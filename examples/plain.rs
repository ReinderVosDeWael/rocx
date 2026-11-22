// docs:start quickstart
use rocx::{Document, Paragraph, Result, Section};

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/plain.docx".into());
    futures::executor::block_on(async {
        Document::new(vec![
            Section::new(vec![
                Paragraph::new("Hello, world!").into(),
                Paragraph::new("A second paragraph, with Unicode: café 🌍").into(),
            ])
            .into(),
        ])
        .with_timestamp(Some("2026-11-21T00:00:00Z".into()))
        .resolve()
        .await?
        .save(output)
    })
}
// docs:end quickstart
