//! Time the complete source-preserving edit loop, including DOCX processing.
use std::time::Instant;

use rocx_layout::{Affinity, LayoutSession, TextPoint};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: docx_reflow FONT.ttf INPUT.docx";
    let font = std::fs::read(args.next().ok_or(usage)?)?;
    let docx = std::fs::read(args.next().ok_or(usage)?)?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let mut session = LayoutSession::new(font.clone(), 12 * 64)?;
    let t = Instant::now();
    let original = session.open_pages(&docx)?;
    let snapshot = original.geometry();
    println!(
        "initial open + physical layout: {:.2?}; {} paragraphs, {} pages",
        t.elapsed(),
        snapshot.layout.paragraphs.len(),
        snapshot.rows.len()
    );
    let paragraph = snapshot
        .layout
        .paragraphs
        .len()
        .checked_sub(1)
        .ok_or("empty document")?;
    let end = snapshot.layout.paragraphs[paragraph]
        .last()
        .ok_or("empty paragraph geometry")?
        .end;
    let caret = snapshot
        .caret_box(
            TextPoint {
                paragraph,
                offset: end,
            },
            Affinity::Downstream,
        )
        .ok_or("no final caret")?;
    let t = Instant::now();
    let edited = original.preview_insert(caret.page, caret.x, caret.top, "!")?;
    println!("preview insertion + encode DOCX: {:.2?}", t.elapsed());
    let t = Instant::now();
    let next = session.open_pages(&edited)?;
    println!(
        "reopen + warm physical layout: {:.2?}; {:?}",
        t.elapsed(),
        next.geometry().layout.reuse
    );
    let t = Instant::now();
    let cold = LayoutSession::with_cache_budget(font, 12 * 64, 0)?.open_pages(&edited)?;
    println!("reopen + cold physical layout: {:.2?}", t.elapsed());
    assert_eq!(next.geometry().rows, cold.geometry().rows);
    assert_eq!(
        next.geometry().layout.paragraphs,
        cold.geometry().layout.paragraphs
    );
    Ok(())
}
