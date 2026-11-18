use rv_docx_rs::{Document, Paragraph, Result, Section, TextRun, Value};

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/text_runs.docx".into());
    futures::executor::block_on(async {
        Document::new(vec![
            Section::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("Hello, ").into(),
                    TextRun::new(Value::future(async { "world".to_owned() })).into(),
                    TextRun::new(Value::lazy(|| panic!("hidden run")))
                        .when(false)
                        .into(),
                    TextRun::new("!").into(),
                ])
                .into(),
            ])
            .into(),
        ])
        .resolve()
        .await?
        .save(output)
    })
}
