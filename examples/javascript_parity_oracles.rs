//! Deterministic native reference packages for the installed JavaScript tests.
use futures::executor::block_on;
use rocx::Result;
use std::path::{Path, PathBuf};

#[path = "../tests/javascript_parity/drawings.rs"]
mod drawings;
#[path = "../tests/javascript_parity/image_path.rs"]
mod image_path;
#[path = "../tests/support/linked_styles.rs"]
mod linked_styles;
#[path = "../tests/support/numbering_replacements.rs"]
mod numbering_replacements;
#[path = "../tests/javascript_parity/structured.rs"]
mod structured;
#[path = "../tests/javascript_parity/tables.rs"]
mod tables;
#[path = "../tests/javascript_parity/template.rs"]
mod template;
#[path = "../tests/javascript_parity/text_layout.rs"]
mod text_layout;

fn main() -> Result<()> {
    let out = PathBuf::from(std::env::args_os().nth(1).expect("output directory"));
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let png = fixtures.join("images/sample.png");
    let editor = fixtures.join("imported-editor.docx");
    std::fs::create_dir_all(&out)?;
    drawings::write(&out, &png)?;
    image_path::write(&out, &png)?;
    structured::write(&out)?;
    block_on(tables::write(&out))?;
    template::write(&out, &editor)?;
    text_layout::write(&out)?;
    for (name, declaration) in [
        ("linked-styles", linked_styles::document()),
        ("numbering-replacements", numbering_replacements::document()),
    ] {
        let declaration = declaration.with_timestamp(Some("2022-02-03T04:05:06Z".into()));
        let bytes = block_on(declaration.resolve())?.to_docx()?;
        std::fs::write(out.join(format!("{name}.docx")), bytes)?;
    }
    Ok(())
}
