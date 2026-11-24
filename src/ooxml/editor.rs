//! Render declaration payloads into an existing package without running the
//! whole-document compiler or changing document defaults.

use super::{
    Compiler, StoryContent, WordParts, resources::MediaRegistry, style_catalog::StyleCatalog,
};
use crate::{
    DeclarationPath, Diagnostic, DrawingId, Error, ResolvedBlock, ResolvedImage, ResolvedInline,
    ResolvedParagraph, ResolvedTable, Result, Rule, StoryKind,
    opc::{Package, PartName},
    xml::Node,
};
use std::collections::BTreeSet;

pub(crate) fn encode_editor_paragraphs(
    package: Package,
    main: &str,
    paragraphs: Vec<ResolvedParagraph>,
) -> Result<(Package, Vec<Node>)> {
    let blocks = paragraphs
        .into_iter()
        .map(ResolvedBlock::Paragraph)
        .collect::<Vec<_>>();
    encode_editor_blocks(package, main, &blocks, None, "editor.paragraph")
}

/// Render supported declaration blocks for body paragraphs and existing cells.
/// Only a nested auto-width table needs an unambiguous parent cell width.
pub(crate) fn encode_editor_blocks(
    package: Package,
    main: &str,
    blocks: &[ResolvedBlock],
    cell_width: Option<i64>,
    scope: &'static str,
) -> Result<(Package, Vec<Node>)> {
    let mut style_refs = BTreeSet::new();
    let path = DeclarationPath::root("sections").index(0).field("children");
    check_blocks(blocks, &path, &mut style_refs, scope)?;
    let styles = editor_styles(&package, main, style_refs)?;
    let mut renderer = fragment_renderer(package, main, styles);
    let story =
        renderer
            .stories
            .register(StoryKind::Body, PartName::new(main)?, StoryContent::Blocks)?;
    let mut output = Vec::with_capacity(blocks.len());
    for block in blocks {
        let node = match block {
            ResolvedBlock::Paragraph(p) => renderer.paragraph(p, &renderer.context(story, 8640))?,
            ResolvedBlock::Table(t) => {
                if t.columns() == 0 {
                    return Err(unsupported("empty replacement table").into());
                }
                let width = if let Some(widths) = t.column_widths()
                    && t.layout() != Some(crate::TableLayout::Autofit)
                {
                    widths.iter().copied().map(i64::from).sum()
                } else {
                    cell_width.ok_or_else(|| unsupported(
                        "nested table requires a positive absolute cell width or explicit table widths",
                    ))?
                };
                renderer.table(t, &renderer.context(story, width))?
            }
            ResolvedBlock::ContentControl(_) => unreachable!("validated above"),
        };
        output.push(node);
    }
    Ok((renderer.package, output))
}

fn editor_styles(package: &Package, main: &str, style_refs: StyleRefs) -> Result<StyleCatalog> {
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
    Ok(styles)
}

pub(crate) fn encode_editor_inlines(
    package: Package,
    main: &str,
    owner: &str,
    kind: StoryKind,
    inlines: &[&ResolvedInline],
) -> Result<(Package, Vec<Vec<Node>>)> {
    let mut refs = StyleRefs::new();
    let mut bookmarks = BTreeSet::new();
    let scope = if kind == StoryKind::Body {
        "editor.inline"
    } else {
        "editor.entryInline"
    };
    for inline in inlines {
        check_inline(inline, &mut refs, scope)?;
        if kind == StoryKind::Body {
            collect_bookmark_targets(inline, &mut bookmarks);
        }
        inline.validate()?;
    }
    if !bookmarks.is_empty() {
        validate_bookmarks(&package, main, &bookmarks)?;
    }
    let styles = editor_styles(&package, main, refs)?;
    let mut renderer = fragment_renderer(package, main, styles);
    let story = renderer
        .stories
        .register(kind, PartName::new(owner)?, StoryContent::Blocks)?;
    let context = renderer.context(story, 8640);
    let output = inlines
        .iter()
        .map(|inline| renderer.inline(inline, &context))
        .collect::<Result<_>>()?;
    Ok((renderer.package, output))
}

fn validate_bookmarks(package: &Package, main: &str, requested: &BTreeSet<&str>) -> Result<()> {
    use std::collections::BTreeMap;
    // A target must be a direct main-story pair. Count shadowing names and
    // numeric aliases even in nested XML and other related stories.
    fn scan(n: &Node, names: &mut BTreeMap<String, usize>, ids: &mut BTreeMap<String, usize>) {
        if matches!(n.name.as_str(), "w:bookmarkStart" | "w:bookmarkEnd") {
            if let Some(id) = n.get("w:id").and_then(super::decimal_id) {
                *ids.entry(id).or_default() += 1;
            }
            if n.name == "w:bookmarkStart"
                && let Some(s) = n.get("w:name")
            {
                *names.entry(s.to_ascii_lowercase()).or_default() += 1;
            }
        }
        for child in n.elements() {
            scan(child, names, ids);
        }
    }
    let (mut names, mut ids) = (BTreeMap::new(), BTreeMap::new());
    let root = package.word(main)?;
    scan(&root, &mut names, &mut ids);
    for part in super::related_story_parts(package, main)? {
        scan(&package.word(&part)?, &mut names, &mut ids);
    }
    let mut starts = BTreeMap::new();
    let mut valid = BTreeSet::new();
    for p in root
        .child("w:body")
        .into_iter()
        .flat_map(Node::elements)
        .filter(|p| p.name == "w:p")
    {
        for n in p.elements() {
            if !matches!(n.name.as_str(), "w:bookmarkStart" | "w:bookmarkEnd") {
                continue;
            }
            let Some(id) = n.get("w:id").and_then(super::decimal_id) else {
                continue;
            };
            if ids.get(&id) != Some(&2) {
                continue;
            }
            if n.name == "w:bookmarkStart" {
                if let Some(name) = n.get("w:name")
                    && names.get(&name.to_ascii_lowercase()) == Some(&1)
                {
                    starts.insert(id, name);
                }
            } else if let Some(name) = starts.remove(&id) {
                valid.insert(name);
            }
        }
    }
    if !requested.is_subset(&valid) {
        return Err(unsupported("expected one paired direct body bookmark target").into());
    }
    Ok(())
}

fn collect_bookmark_targets<'a>(inline: &'a ResolvedInline, names: &mut BTreeSet<&'a str>) {
    let (target, children) = match inline {
        ResolvedInline::Field(f) => (f.instruction().bookmark_name(), f.children()),
        ResolvedInline::Hyperlink(h) => (h.target().bookmark_name(), h.children()),
        _ => return,
    };
    if let Some(name) = target {
        names.insert(name.as_str());
    }
    for child in children {
        collect_bookmark_targets(child, names);
    }
}

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
    let styles = editor_styles(&package, main, style_refs)?;
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

/// Drawing and media allocations shared by the header images in one edit batch.
pub(crate) struct HeaderImageResources {
    drawing: DrawingId,
    media: MediaRegistry,
}

pub(crate) fn encode_header_image(
    package: Package,
    main: &str,
    header: &str,
    image: &ResolvedImage,
    resources: &mut Option<HeaderImageResources>,
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
    let resources = match resources {
        Some(resources) => resources,
        None => resources.insert(HeaderImageResources {
            drawing: DrawingId::new(super::defaults::drawing_ceiling(&package)),
            media: MediaRegistry::from_package(&package)?,
        }),
    };
    let mut renderer = fragment_renderer(package, main, styles);
    renderer.drawing = resources.drawing;
    renderer.media = std::mem::take(&mut resources.media);
    let node = Node::new("w:p").with(renderer.image(image, &owner)?);
    resources.drawing = renderer.drawing;
    resources.media = renderer.media;
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
            check_blocks(
                cell.blocks(),
                &path.field("children"),
                styles,
                "editor.table",
            )?;
        }
    }
    Ok(())
}

fn check_inline(run: &ResolvedInline, styles: &mut StyleRefs, scope: &'static str) -> Result<()> {
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
        ResolvedInline::Field(field) if scope == "editor.inline" => {
            for child in field.children() {
                check_inline(child, styles, scope)?;
            }
            Ok(())
        }
        ResolvedInline::Hyperlink(link)
            if scope == "editor.inline" || link.target().bookmark_name().is_none() =>
        {
            for child in link.children() {
                check_inline(child, styles, scope)?;
            }
            Ok(())
        }
        _ => Err(Diagnostic::new(
            Rule::UnsupportedEdit,
            match scope {
                "editor.table" => "unsupported inline content in an inserted table",
                "editor.inline" | "editor.entryInline" => "unsupported inline declaration",
                _ => "unsupported inline content in an inserted paragraph",
            },
        )
        .at(scope)
        .into()),
    }
}

fn check_blocks(
    blocks: &[ResolvedBlock],
    path: &DeclarationPath,
    styles: &mut StyleRefs,
    scope: &'static str,
) -> Result<()> {
    for (index, block) in blocks.iter().enumerate() {
        let path = path.index(index);
        match block {
            ResolvedBlock::Table(table) => check_table(table, &path, styles)?,
            ResolvedBlock::Paragraph(paragraph) => {
                if paragraph.numbering().is_some() || paragraph.comment().is_some() {
                    return Err(Diagnostic::new(
                        Rule::UnsupportedEdit,
                        "numbering and comments are not yet supported",
                    )
                    .at(scope)
                    .into());
                }
                let format = paragraph.format();
                if let Some(heading) = format.heading.filter(|value| *value > 0) {
                    styles.insert(("paragraph", format!("Heading{heading}")));
                } else if let Some(style) = &format.style {
                    styles.insert(("paragraph", style.clone()));
                }
                for run in paragraph.runs() {
                    check_inline(run, styles, scope)?;
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
