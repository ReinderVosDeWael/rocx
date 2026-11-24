use futures::executor::block_on;
use rocx::{
    Document, Paragraph, ResolvedDocument, Section, application::RenderOptions,
    template::TemplateEdits,
};
use std::path::Path;
const SNAPSHOT: &str = "2026-11-21T00:00:00Z";
const OUTPUT: &str = "2026-12-01T12:30:00.123Z";
fn resolve(sections: &[&str]) -> ResolvedDocument {
    let doc = Document::new(
        sections
            .iter()
            .map(|text| Section::new(vec![Paragraph::new(*text).into()]).into())
            .collect(),
    )
    .with_timestamp(Some(SNAPSHOT.into()));
    block_on(doc.resolve()).unwrap()
}
pub fn write(out: &Path, editor: &Path) -> rocx::Result<()> {
    let doc = resolve(&["TOKEN"]);
    let options = RenderOptions {
        timestamp: Some(OUTPUT.into()),
        ..Default::default()
    };
    std::fs::write(out.join("fresh.docx"), doc.to_docx()?)?;
    std::fs::write(
        out.join("override.docx"),
        doc.to_docx_with_options(&options)?,
    )?;
    let doc = resolve(&["Added section one", "Added section two"]);
    let source = std::fs::read(editor)?;
    let edits = TemplateEdits {
        paragraph_index: Some(1),
        replacements: vec![("Insert table after me".into(), "Patched".into())],
        ..Default::default()
    };
    let encoded =
        rocx::application::compile_with_edits(&doc, &source, &edits, &options)?.to_bytes()?;
    std::fs::write(out.join("edited.docx"), encoded)?;
    write_render_options(out, &source)?;
    Ok(())
}

fn write_render_options(out: &Path, source: &[u8]) -> rocx::Result<()> {
    for (case, snapshot, override_date) in [
        (
            "fractional",
            "2026-01-01T00:00:00.1Z",
            "2026-02-28T01:02:03.123456789Z",
        ),
        (
            "offset",
            "2026-02-28T01:02:03.123456789+00:00",
            "2026-03-01T01:02:03.00001-04:30",
        ),
        // Deliberately not schema-valid dates: the native API does not parse
        // calendar dates or normalize the text placed in core properties.
        ("unparsed", "2026-02-30T01:02:03.1Z", "not a <date> & λ"),
    ] {
        let doc = block_on(
            Document::new(vec![
                Section::new(vec![Paragraph::new("Budgets").into()]).into(),
            ])
            .with_timestamp(Some(snapshot.into()))
            .resolve(),
        )?;
        let options = RenderOptions {
            timestamp: Some(override_date.into()),
            read_limits: rocx::opc::ReadLimits {
                max_parts: u32::MAX as usize,
                max_part_bytes: 9_007_199_254_740_993,
                max_expanded_bytes: u64::MAX,
            },
            ..Default::default()
        };
        std::fs::write(out.join(format!("{case}-snapshot.docx")), doc.to_docx()?)?;
        std::fs::write(
            out.join(format!("{case}-override.docx")),
            doc.to_docx_with_options(&options)?,
        )?;
        std::fs::write(
            out.join(format!("{case}-template.docx")),
            rocx::application::compile(&doc, source, &options)?.to_bytes()?,
        )?;
    }
    Ok(())
}
