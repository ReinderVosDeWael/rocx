//! Application orchestration and explicit rendering options.
use crate::opc::{PackagePlan, ReadLimits};
use crate::{OutputProfile, ResolvedDocument, Result};
#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    pub profile: OutputProfile,
    pub timestamp: Option<String>,
    pub read_limits: ReadLimits,
}
/// Compile existing bytes without filesystem access or implicit clock reads.
pub fn compile(
    document: &ResolvedDocument,
    template: &[u8],
    options: &RenderOptions,
) -> Result<PackagePlan> {
    crate::ooxml::compile(document, template, None, options)
}
pub(crate) fn render(document: &ResolvedDocument) -> Result<Vec<u8>> {
    render_with_template(document, None)
}
pub(crate) fn render_with_template(
    document: &ResolvedDocument,
    template: Option<&DocumentTemplate>,
) -> Result<Vec<u8>> {
    let bytes = match template {
        Some(t) => crate::adapters::io::read(&t.path)?,
        None => include_bytes!("../assets/default.docx").to_vec(),
    };
    crate::ooxml::compile(
        document,
        &bytes,
        template.map(|t| &t.edits),
        &RenderOptions::default(),
    )?
    .to_bytes()
}

use std::path::Path;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentTemplate {
    pub path: std::path::PathBuf,
    pub edits: crate::template::TemplateEdits,
}

impl DocumentTemplate {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Self::default()
        }
    }
}

impl ResolvedDocument {
    pub fn to_docx_with_template(&self, template: &DocumentTemplate) -> Result<Vec<u8>> {
        crate::application::render_with_template(self, Some(template))
    }
    pub fn save_with_template(
        &self,
        path: impl AsRef<Path>,
        template: &DocumentTemplate,
    ) -> Result<()> {
        let bytes = self.to_docx_with_template(template)?;
        crate::adapters::io::write(path.as_ref(), &bytes)?;
        Ok(())
    }
    pub fn to_docx(&self) -> Result<Vec<u8>> {
        crate::application::render(self)
    }

    /// Compile first, then replace the destination through a sibling temporary file.
    /// File I/O is synchronous and isolated in the filesystem adapter.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.to_docx()?;
        crate::adapters::io::write(path.as_ref(), &bytes)?;
        Ok(())
    }
}

/// Compile a pure template-edit request over caller-provided package bytes.
pub fn compile_with_edits(
    document: &ResolvedDocument,
    template: &[u8],
    edits: &crate::template::TemplateEdits,
    options: &RenderOptions,
) -> Result<PackagePlan> {
    crate::ooxml::compile(document, template, Some(edits), options)
}
