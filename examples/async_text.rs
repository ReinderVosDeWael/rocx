use cmi_docx::{Document, Paragraph, Result, Section, Value};

async fn fetch_text() -> String {
    "Text produced asynchronously.".into()
}

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/async_text.docx".into());
    futures::executor::block_on(async {
        Document::new(vec![
            Section::new(vec![Paragraph::new(Value::future(fetch_text())).into()]).into(),
        ])
        .resolve()
        .await?
        .save(output)
    })
}
