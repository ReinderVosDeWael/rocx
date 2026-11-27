//! Emit glyph-indexed SVG outlines for the checkout-only browser demo.
use std::io;

use rustybuzz::ttf_parser::Face;

#[path = "support/outline.rs"]
mod outline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: font_atlas FONT.ttf";
    let font = args.next().ok_or(usage)?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let bytes = std::fs::read(font)?;
    let face = Face::parse(&bytes, 0).map_err(|_| "invalid outline font")?;
    // Include every glyph ID, not just characters in cmap: shaping can
    // choose ligatures and other unencoded glyphs. Spaces have empty paths.
    let paths: Vec<_> = (0..face.number_of_glyphs())
        .map(|id| outline::svg_path(&face, id))
        .collect();
    // FNV-1a pairs the outline atlas with the bytes passed to Wasm. The
    // preparation instructions separately verify the pinned font's SHA-256.
    let fingerprint = bytes.iter().fold(0x811c9dc5_u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x01000193)
    });
    serde_json::to_writer(
        io::stdout().lock(),
        &serde_json::json!({
            "unitsPerEm": face.units_per_em(),
            "fingerprint": fingerprint,
            "paths": paths,
        }),
    )?;
    Ok(())
}
