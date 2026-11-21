//! Matching Rust/Python visual inspection catalog. See examples/ALL_FEATURES.md.
#[path = "showcase/mod.rs"]
mod showcase;

fn main() -> rocx::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/all_features-rust.docx".into());
    futures::executor::block_on(showcase::generate(std::path::Path::new(&output)))
}
