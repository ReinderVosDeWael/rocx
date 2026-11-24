//! The Rust visual catalog also runs without Python or official schemas.
#[path = "support/catalog_readback.rs"]
mod catalog_readback;
#[path = "../examples/showcase/mod.rs"]
mod showcase;

use std::io::{Cursor, Read};

#[test]
fn visual_catalog_sorts_combined_conditional_properties_and_all_cell_borders() {
    let directory = std::env::temp_dir().join(format!("rocx-catalog-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("catalog.docx");
    futures::executor::block_on(showcase::generate(&path)).unwrap();
    catalog_readback::verify(&path);
    let bytes = std::fs::read(&path).unwrap();
    let mut package = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let borders = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "tcBorders")))
        .unwrap();
    assert_eq!(
        borders
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["top", "start", "bottom", "end"]
    );
    let mut xml = String::new();
    package
        .by_name("word/styles.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let style = tree
        .descendants()
        .find(|n| n.attribute((w, "styleId")) == Some("Regionfirst_row"))
        .unwrap();
    let conditional = style
        .children()
        .find(|n| n.has_tag_name((w, "tblStylePr")))
        .unwrap();
    assert_eq!(
        conditional
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["pPr", "rPr", "tcPr"]
    );
    std::fs::remove_file(path.with_extension("template.docx")).unwrap();
    std::fs::remove_file(path.with_extension("edited.docx")).unwrap();
    for suffix in [
        "entry-text.docx",
        "entry-edits.docx",
        "story-blocks.docx",
        "footer-even.docx",
        "run-text.docx",
        "header-source.docx",
        "header-shared.docx",
        "header-local.docx",
        "header-first-shared.docx",
        "header-first-local.docx",
    ] {
        std::fs::remove_file(path.with_extension(suffix)).unwrap();
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
