mod support;
fn main() -> cmi_docx::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/feature_parity.docx".into());
    futures::executor::block_on(support::document().resolve())?.save(output)
}
