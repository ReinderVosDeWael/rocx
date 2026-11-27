//! Measure one-character edits in a synthetic long document. Run in release mode.
use std::time::Instant;

use rocx_layout::{LayoutSession, PageSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: reflow FONT.ttf [--physical]";
    let font = args.next().ok_or(usage)?;
    let font = std::fs::read(font)?;
    let mut session = LayoutSession::new(font.clone(), 12 * 64)?;
    let mut text: Vec<_> = (0..1000)
        .map(|n| format!("Paragraph {n}: a measured line of ordinary text."))
        .collect();
    if let Some(arg) = args.next() {
        if arg != "--physical" || args.next().is_some() {
            return Err(usage.into());
        }
        let spec = PageSpec {
            width: 252 * 64,
            height: 500 * 64,
            left: 36 * 64,
            right: 36 * 64,
            top: 16 * 64,
            bottom: 16 * 64,
            space_before: 0,
            space_after: 3 * 64,
        };
        for (label, change) in [
            ("initial", None),
            ("edit at start", Some(0usize)),
            ("edit at end", Some(999)),
        ] {
            if let Some(i) = change {
                text[i].push('!');
            }
            let started = Instant::now();
            let warm = session.layout_pages(&text, spec)?;
            println!(
                "{label}: {:.2?}, {} physical pages, {:?}",
                started.elapsed(),
                warm.rows.len(),
                warm.layout.reuse
            );
            if change.is_some() {
                let cold = LayoutSession::with_cache_budget(font.clone(), 12 * 64, 0)?
                    .layout_pages(&text, spec)?;
                assert_eq!(warm.rows, cold.rows);
                assert_eq!(warm.layout.paragraphs, cold.layout.paragraphs);
            }
        }
        return Ok(());
    }
    let mut timed = |name: &str, paragraphs: &[String]| {
        let start = Instant::now();
        let result = session.layout(paragraphs, 180 * 64, 32)?;
        println!(
            "{name}: {:.2?}, {} pages, {:?}",
            start.elapsed(),
            result.pages.len(),
            result.reuse
        );
        Ok::<_, String>(result)
    };
    timed("initial", &text)?;
    for (name, index) in [("edit at start", 0), ("edit at end", 999)] {
        text[index].push('!');
        let warm = timed(name, &text)?;
        let cold = LayoutSession::with_cache_budget(font.clone(), 12 * 64, 0)?.layout(
            &text,
            180 * 64,
            32,
        )?;
        assert_eq!(warm.paragraphs, cold.paragraphs);
        assert_eq!(warm.pages, cold.pages);
    }
    Ok(())
}
