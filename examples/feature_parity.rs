mod support;
fn main() -> rv_docx_rs::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/feature_parity.docx".into());
    futures::executor::block_on(support::document().resolve())?.save(output)
}
