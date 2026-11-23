//! Application orchestration and explicit rendering options.
use crate::ooxml::{CompileSource, compile as compile_ooxml};
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
    compile_ooxml(
        document,
        CompileSource::Template {
            bytes: template,
            edits: None,
        },
        options,
    )
}
pub(crate) fn render(document: &ResolvedDocument) -> Result<Vec<u8>> {
    render_with_template(document, None)
}
pub(crate) fn render_with_template(
    document: &ResolvedDocument,
    template: Option<&DocumentTemplate>,
) -> Result<Vec<u8>> {
    render_with_options(document, template, &RenderOptions::default())
}
pub(crate) fn render_with_options(
    document: &ResolvedDocument,
    template: Option<&DocumentTemplate>,
    options: &RenderOptions,
) -> Result<Vec<u8>> {
    let plan = match template {
        Some(t) => {
            let bytes = crate::adapters::io::read(&t.path)?;
            compile_with_edits(document, &bytes, &t.edits, options)?
        }
        None => compile_ooxml(document, CompileSource::Fresh, options)?,
    };
    plan.to_bytes()
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
    /// Render using explicit profile, timestamp and package-read budgets.
    pub fn to_docx_with_options(&self, options: &RenderOptions) -> Result<Vec<u8>> {
        render_with_options(self, None, options)
    }
    pub fn to_docx_with_template_and_options(
        &self,
        template: &DocumentTemplate,
        options: &RenderOptions,
    ) -> Result<Vec<u8>> {
        render_with_options(self, Some(template), options)
    }
    pub fn save_with_options(&self, path: impl AsRef<Path>, options: &RenderOptions) -> Result<()> {
        let bytes = self.to_docx_with_options(options)?;
        crate::adapters::io::write(path.as_ref(), &bytes)
    }
    pub fn save_with_template_and_options(
        &self,
        path: impl AsRef<Path>,
        template: &DocumentTemplate,
        options: &RenderOptions,
    ) -> Result<()> {
        let bytes = self.to_docx_with_template_and_options(template, options)?;
        crate::adapters::io::write(path.as_ref(), &bytes)
    }
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
    compile_ooxml(
        document,
        CompileSource::Template {
            bytes: template,
            edits: Some(edits),
        },
        options,
    )
}
