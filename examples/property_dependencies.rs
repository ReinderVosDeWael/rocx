//! Run with `cargo run --example property_dependencies -- report.docx`.
use rocx::{Document, Error, Paragraph, Section, Value};

fn main() -> Result<(), Error> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "report.docx".into());
    futures::executor::block_on(async {
        let result = Value::future(async { "42".to_owned() }).deferred();
        let dependency = result.clone();
        let summary = Paragraph::new(Value::try_future(async move {
            Ok::<_, Error>(format!("The result is {}", dependency.await?))
        }));
        let source = Paragraph::new(result);
        Document::new(vec![
            Section::new(vec![summary.into(), source.into()]).into(),
        ])
        .resolve()
        .await?
        .save(destination)
    })
}
