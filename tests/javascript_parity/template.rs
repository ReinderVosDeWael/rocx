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
    Ok(())
}
