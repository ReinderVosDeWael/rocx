//! Write an edited DOCX to stdout. Uses source scalar coordinates and the
//! bound physical snapshot; equal endpoints insert at the visual caret.
use std::io::{self, Write};

use rocx_layout::{Affinity, LayoutSession, TextPoint};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: edit_page FONT.ttf INPUT.docx PARAGRAPH START END TEXT > output.docx";
    let font = std::fs::read(args.next().ok_or(usage)?)?;
    let source = std::fs::read(args.next().ok_or(usage)?)?;
    let paragraph = args.next().ok_or(usage)?.parse()?;
    let start = TextPoint {
        paragraph,
        offset: args.next().ok_or(usage)?.parse()?,
    };
    let end = TextPoint {
        paragraph,
        offset: args.next().ok_or(usage)?.parse()?,
    };
    let text = args.next().ok_or(usage)?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let pages = LayoutSession::new(font, 12 * 64)?.open_pages(&source)?;
    let bytes = if start == end {
        let caret = pages
            .geometry()
            .caret_box(start, Affinity::Downstream)
            .ok_or("invalid scalar caret boundary")?;
        if pages.geometry().hit_test(caret.page, caret.x, caret.top)
            != Some((paragraph, start.offset))
        {
            return Err(
                "rounded visual caret does not uniquely target the requested offset".into(),
            );
        }
        pages.preview_insert(caret.page, caret.x, caret.top, &text)?
    } else {
        pages.preview_replace(start, end, &text)?
    };
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
