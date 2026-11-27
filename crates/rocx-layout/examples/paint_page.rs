//! Debug painter: print one physical DOCX page as SVG using the engine's glyphs.
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io::{self, Write},
};

use rocx_layout::LayoutSession;
use rustybuzz::ttf_parser::Face;
#[path = "support/outline.rs"]
mod outline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: paint_page FONT.ttf INPUT.docx [zero-based page index]";
    let font = std::fs::read(args.next().ok_or(usage)?)?;
    let docx = std::fs::read(args.next().ok_or(usage)?)?;
    let page = args
        .next()
        .map(|s| s.into_string().map_err(|_| usage))
        .transpose()?
        .unwrap_or_else(|| "0".into())
        .parse::<usize>()?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let size = 12 * 64;
    let mut engine = LayoutSession::new(font.clone(), size)?;
    let source = engine.open_pages(&docx)?;
    let (w, h) = (source.spec().width, source.spec().height);
    let result = source.geometry();
    let rows = result.rows.get(page).ok_or("page outside document")?;
    let face = Face::parse(&font, 0).map_err(|_| "invalid outline font")?;
    let scale = f64::from(size) / f64::from(face.units_per_em());
    let mut outlines = BTreeMap::new();
    let mut body = String::new();
    for row in rows.iter() {
        for g in &result.layout.paragraphs[row.paragraph][row.line].glyphs {
            outlines
                .entry(g.id)
                .or_insert_with(|| outline::svg_path(&face, g.id as u16));
            // Glyph positions already include shaping offsets. Only font
            // outlines are y-up; screen baselines and glyph y are y-down.
            let x = i64::from(row.x) + i64::from(g.x);
            let y = i64::from(row.baseline) + i64::from(g.y);
            writeln!(
                body,
                "<use href=\"#g{}\" transform=\"translate({x} {y}) scale({scale} -{scale})\"/>",
                g.id
            )?;
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{}pt\" height=\"{}pt\">",
        f64::from(w) / 64.,
        f64::from(h) / 64.
    )?;
    writeln!(out, "<defs>")?;
    for (id, path) in outlines {
        writeln!(out, "<path id=\"g{id}\" d=\"{path}\"/>")?;
    }
    writeln!(
        out,
        "</defs><rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n{body}</svg>"
    )?;
    out.flush()?;
    Ok(())
}
