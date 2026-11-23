//! Native Rust equivalent of the JavaScript report, for exact byte parity.
#[path = "showcase/javascript.rs"]
mod javascript;

fn main() -> rocx::Result<()> {
    let path = std::env::args().nth(1).expect("Pass the DOCX output path");
    let document = rocx::Document::new(vec![rocx::Section::new(javascript::blocks()).into()])
        .with_timestamp(Some("2026-11-21T00:00:00Z".into()));
    let bytes = futures::executor::block_on(document.resolve())?.to_docx()?;
    std::fs::write(path, bytes)?;
    Ok(())
}
