//! Print page-local placements and line geometry for a native golden comparison.
use rocx_layout::LayoutSession;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let font = args.next().ok_or("usage: pages FONT.ttf [physical.docx]")?;
    let mut session = LayoutSession::new(std::fs::read(font)?, 12 * 64)?;
    if let Some(input) = args.next() {
        let source = session.open_pages(&std::fs::read(input)?)?;
        let snapshot = source.geometry();
        let pages: Vec<Vec<_>> = snapshot
            .rows
            .iter()
            .map(|rows| {
                rows.iter()
                    .map(|r| (r, &snapshot.layout.paragraphs[r.paragraph][r.line]))
                    .collect()
            })
            .collect();
        println!("{}", serde_json::to_string(&pages)?);
        return Ok(());
    }
    let snapshot = session.layout(&["hello".into(), "hello".into()], 20_000, 1)?;
    let pages: Vec<Vec<_>> = snapshot
        .pages
        .iter()
        .map(|page| {
            page.lines
                .iter()
                .map(|p| (p, &snapshot.paragraphs[p.paragraph][p.line]))
                .collect()
        })
        .collect();
    println!("{}", serde_json::to_string(&pages)?);
    Ok(())
}
