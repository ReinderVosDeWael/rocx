use rocx::{Condition, Document, Paragraph, Result, Section, Value};

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/conditional.docx".into());
    futures::executor::block_on(async {
        Document::new(vec![
            Section::new(vec![
                Paragraph::new("Always visible.").into(),
                Paragraph::new(Value::lazy(|| "Only constructed when visible.".into()))
                    .when(Condition::lazy(|| true))
                    .into(),
                Paragraph::new(Value::lazy(|| panic!("A hidden producer must never run")))
                    .when(false)
                    .into(),
            ])
            .into(),
        ])
        .resolve()
        .await?
        .save(output)
    })
}
