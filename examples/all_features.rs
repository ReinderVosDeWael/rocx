//! Matching Rust/Python visual inspection catalog. See examples/ALL_FEATURES.md.
//! Includes labeled DrawingML shape text styling and its omission boundaries.
//! The shape gallery also requests italic text and a Latin typeface explicitly.
//! Standalone inline and floating text boxes use a direct DrawingML text shape.
//! Includes chainable authoring builders and omitted/true/false formatting.
//! Demonstrates two concrete lists sharing an authored abstract numbering ID and separate formatting, paragraph style links, and an inherited numbering style.
//! Shows an inline image with crop, rotation and flipping, plus floating-picture wrap sides, distances, alignment, reference frames, placement flags and a wrapping polygon.
//! Shows inline rectangles, ellipses with outlines and DrawingML shape text.
//! Labels inline, block, row and cell controls, explicit IDs, plain text, a drop-down list and a tag-targeted template edit.
//! Shows tracked text insertions, deletions and paired moves with review metadata and tracking settings and template acceptance of text and moves.
//! Labels independent footnote and endnote numbering formats, defaults and placement.
//! Labels document-wide mirror margins and top gutter requests.
//! Includes contextual paragraph spacing and mirrored-indent controls.
//! Shows section line-number restart modes and paragraph suppression, plus header/footer edge distances and physical page borders, placement options and document grids with paragraph and run snapping.
#[path = "showcase/mod.rs"]
mod showcase;

fn main() -> rocx::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("examples/all_features-rust.docx".into());
    futures::executor::block_on(showcase::generate(std::path::Path::new(&output)))
}
