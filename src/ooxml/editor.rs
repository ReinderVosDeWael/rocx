//! Render declaration payloads into an existing package without running the
//! whole-document compiler or changing document defaults.

use super::{Compiler, StoryContent, WordParts, style_catalog::StyleCatalog};
use crate::{
    Diagnostic, ResolvedBlock, ResolvedDocument, ResolvedInline, ResolvedSection, ResolvedTable,
    Result, Rule, StoryKind,
    opc::{Package, PartName},
    xml::Node,
};
use std::collections::BTreeSet;

pub(crate) fn encode_editor_tables(
    package: Package,
    main: &str,
    tables: Vec<(ResolvedTable, i64)>,
) -> Result<(Package, Vec<Node>)> {
    let (blocks, widths): (Vec<_>, Vec<_>) = tables
        .into_iter()
        .map(|(table, width)| (ResolvedBlock::Table(table), width))
        .unzip();
    let doc = ResolvedDocument {
        sections: vec![ResolvedSection {
            blocks,
            properties: None,
            headers: Default::default(),
            footers: Default::default(),
        }],
        metadata: Default::default(),
        styles: Vec::new(),
        numbering: Vec::new(),
        notes: Vec::new(),
        defaults: None,
        comment_author: None,
        timestamp: String::new(),
    };
    let mut style_refs = BTreeSet::new();
    check_payload(doc.sections()[0].blocks(), &mut style_refs)?;
    let style_part = if style_refs.is_empty() {
        None
    } else {
        package.related(main, "styles")?
    };
    let styles = match style_part {
        Some((_, path)) => package.word(&path)?,
        None => Node::new("w:styles"),
    };
    if styles.name != "w:styles" {
        return Err(unsupported("unexpected styles part root").into());
    }
    let styles = StyleCatalog::new(&styles)?;
    for (kind, name) in style_refs {
        styles.get_typed(&name, kind)?;
    }
    doc.validate()?;

    // Table payloads in this slice may use existing styles and external links.
    // Parts and ID catalogs not involved in those operations are not scanned.
    let mut renderer = Compiler {
        package,
        main: main.into(),
        drawing: Default::default(),
        revisions: None,
        content_control_ids: None,
        media: Default::default(),
        stories: Default::default(),
        header_stories: Default::default(),
        ranges: Default::default(),
        notes: Default::default(),
        styles,
        document: &doc,
    };
    let story =
        renderer
            .stories
            .register(StoryKind::Body, PartName::new(main)?, StoryContent::Blocks)?;
    let mut output = Vec::with_capacity(widths.len());
    for (block, width) in doc.sections()[0].blocks().iter().zip(widths) {
        let context = renderer.context(story, width);
        output.extend(renderer.blocks(std::slice::from_ref(block), &context)?);
    }
    Ok((renderer.package, output))
}

fn unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(Rule::UnsupportedEdit, message).at("editor.table")
}

type StyleRefs = BTreeSet<(&'static str, String)>;

fn check_payload(blocks: &[ResolvedBlock], styles: &mut StyleRefs) -> Result<()> {
    fn check_inline(run: &ResolvedInline, styles: &mut StyleRefs) -> Result<()> {
        if let Some(style) = run.format().and_then(|format| format.style.as_ref()) {
            styles.insert(("character", style.clone()));
        }
        match run {
            ResolvedInline::Text {
                comment: None,
                revision: None,
                ..
            }
            | ResolvedInline::Tab { comment: None, .. }
            | ResolvedInline::Break { comment: None, .. }
            | ResolvedInline::Fraction { .. } => Ok(()),
            ResolvedInline::Hyperlink(link) if link.target().bookmark_name().is_none() => {
                for child in link.children() {
                    check_inline(child, styles)?;
                }
                Ok(())
            }
            _ => Err(unsupported("unsupported inline content in an inserted table").into()),
        }
    }
    for block in blocks {
        match block {
            ResolvedBlock::Table(table) => {
                if let Some(style) = table.style() {
                    styles.insert(("table", style.into()));
                }
                for row in table.rows() {
                    if row.properties().content_control.is_some() {
                        return Err(
                            unsupported("row content controls are not yet supported").into()
                        );
                    }
                    for cell in row.cells() {
                        if cell.content_control().is_some() {
                            return Err(
                                unsupported("cell content controls are not yet supported").into()
                            );
                        }
                        check_payload(cell.blocks(), styles)?;
                    }
                }
            }
            ResolvedBlock::Paragraph(paragraph) => {
                if paragraph.numbering().is_some() || paragraph.comment().is_some() {
                    return Err(unsupported("numbering and comments are not yet supported").into());
                }
                let format = paragraph.format();
                if let Some(heading) = format.heading.filter(|value| *value > 0) {
                    styles.insert(("paragraph", format!("Heading{heading}")));
                } else if let Some(style) = &format.style {
                    styles.insert(("paragraph", style.clone()));
                }
                for run in paragraph.runs() {
                    check_inline(run, styles)?;
                }
            }
            ResolvedBlock::ContentControl(_) => {
                return Err(unsupported("block content controls are not yet supported").into());
            }
        }
    }
    Ok(())
}
