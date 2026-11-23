//! Render declaration payloads into an existing package without running the
//! whole-document compiler or changing document defaults.

use super::{Compiler, StoryContent, WordParts, style_catalog::StyleCatalog};
use crate::{
    DeclarationPath, Diagnostic, DrawingId, Error, ResolvedBlock, ResolvedImage, ResolvedInline,
    ResolvedTable, Result, Rule, StoryKind,
    opc::{Package, PartName},
    xml::Node,
};
use std::collections::BTreeSet;

pub(crate) fn encode_editor_tables(
    package: Package,
    main: &str,
    tables: Vec<(ResolvedTable, i64)>,
) -> Result<(Package, Vec<Node>)> {
    let mut style_refs = BTreeSet::new();
    let path = DeclarationPath::root("sections").index(0).field("children");
    for (index, (table, _)) in tables.iter().enumerate() {
        check_table(table, &path.index(index), &mut style_refs)?;
    }
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
    let mut renderer = fragment_renderer(package, main, styles);
    let story =
        renderer
            .stories
            .register(StoryKind::Body, PartName::new(main)?, StoryContent::Blocks)?;
    let mut output = Vec::with_capacity(tables.len());
    for (table, width) in tables {
        let context = renderer.context(story, width);
        output.push(renderer.table(&table, &context)?);
    }
    Ok((renderer.package, output))
}

pub(crate) fn encode_header_image(
    package: Package,
    main: &str,
    header: &str,
    image: &ResolvedImage,
) -> Result<(Package, Node)> {
    if image.anchor.is_some() {
        return Err(Diagnostic::new(
            Rule::UnsupportedEdit,
            "header image insertion requires an inline image",
        )
        .at("editor.header")
        .into());
    }
    let owner = PartName::new(header)?;
    let styles = StyleCatalog::new(&Node::new("w:styles"))?;
    let drawing = DrawingId::new(super::defaults::drawing_ceiling(&package));
    let media = super::resources::MediaRegistry::from_package(&package)?;
    let mut renderer = fragment_renderer(package, main, styles);
    renderer.drawing = drawing;
    renderer.media = media;
    let node = Node::new("w:p").with(renderer.image(image, &owner)?);
    Ok((renderer.package, node))
}

fn fragment_renderer(package: Package, main: &str, styles: StyleCatalog) -> Compiler<'static> {
    Compiler {
        package,
        main: main.into(),
        drawing: Default::default(),
        revisions: None,
        content_control_ids: None,
        media: Default::default(),
        stories: Default::default(),
        header_stories: Default::default(),
        ranges: Default::default(),
        styles,
        comment_defaults: None,
    }
}

fn unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(Rule::UnsupportedEdit, message).at("editor.table")
}

type StyleRefs = BTreeSet<(&'static str, String)>;

fn check_table(
    table: &ResolvedTable,
    path: &DeclarationPath,
    styles: &mut StyleRefs,
) -> Result<()> {
    if let Some(style) = table.style() {
        styles.insert(("table", style.into()));
    }
    for (row_index, row) in table.rows().iter().enumerate() {
        let path = path.field("rows").index(row_index);
        if row.properties().content_control.is_some() {
            return Err(unsupported("row content controls are not yet supported").into());
        }
        row.properties()
            .validate()
            .map_err(|error| Error::from(error).at(path.field("properties").to_string()))?;
        for (cell_index, cell) in row.cells().iter().enumerate() {
            if cell.content_control().is_some() {
                return Err(unsupported("cell content controls are not yet supported").into());
            }
            // ResolvedTable's checked constructor validates grid spans.
            let path = path.field("children").index(cell_index);
            if let Some(margins) = cell.margins() {
                margins
                    .validate()
                    .map_err(|error| Error::from(error).at(path.field("margins").to_string()))?;
            }
            check_blocks(cell.blocks(), &path.field("children"), styles)?;
        }
    }
    Ok(())
}

fn check_blocks(
    blocks: &[ResolvedBlock],
    path: &DeclarationPath,
    styles: &mut StyleRefs,
) -> Result<()> {
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
    for (index, block) in blocks.iter().enumerate() {
        let path = path.index(index);
        match block {
            ResolvedBlock::Table(table) => check_table(table, &path, styles)?,
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
                paragraph
                    .validate()
                    .map_err(|error| Error::from(error).at(path.to_string()))?;
            }
            ResolvedBlock::ContentControl(_) => {
                return Err(unsupported("block content controls are not yet supported").into());
            }
        }
    }
    Ok(())
}
