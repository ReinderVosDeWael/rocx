//! Pure OOXML compilation into an inspectable package write plan.
//! ECMA-376 Part 1 (2016), §17; Part 2 (2021), §§6–8.
use crate::opc::Package;
use crate::xml::{Content, Node};
use crate::{
    DrawingId, Error, HeaderFooterType, ResolvedDocument, Result, StoryId, StoryKind, Twips,
};
use wordprocessing::sections::{section_properties, usable_width};
pub(crate) mod defaults;
mod drawing;
mod profile;
mod properties;
pub(crate) mod wordprocessing;
pub use profile::OutputProfile;
mod story;
pub use story::{StoryContent, StoryDescriptor, StoryRegistry};
mod editor;
mod notes;
mod ranges;
mod revisions;
pub(crate) use editor::{
    encode_editor_blocks, encode_editor_paragraphs, encode_editor_tables, encode_header_image,
};
pub(crate) struct Compiler<'a> {
    package: Package,
    main: String,
    drawing: DrawingId,
    revisions: Option<revisions::RevisionIds>,
    content_control_ids: Option<std::collections::BTreeSet<u32>>,
    media: resources::MediaRegistry,
    stories: StoryRegistry,
    header_stories: std::collections::BTreeMap<String, StoryId>,
    ranges: ranges::RangeCatalog,
    styles: style_catalog::StyleCatalog,
    comment_defaults: Option<CommentDefaults<'a>>,
}
struct CommentDefaults<'a> {
    author: Option<&'a str>,
    timestamp: &'a str,
}
const DOCX_MAIN: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const DOTX_MAIN: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml";

pub(crate) fn open_word_source(
    bytes: &[u8],
    limits: crate::opc::ReadLimits,
    allow_template: bool,
) -> Result<(Package, String, Node)> {
    let package = Package::load_with_limits(bytes, limits)?;
    // ECMA-376 Part 2: the package-level signature origin relationship
    // identifies the signing graph, regardless of its target part name.
    const SIGNATURE_ORIGIN: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/origin";
    if !package.relationships("", SIGNATURE_ORIGIN)?.is_empty() {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "digitally signed templates are not supported; use an unsigned template",
        )
        .at("_rels/.rels")
        .into());
    }
    let main = package
        .related("", "officeDocument")?
        .map(|(_, p)| p)
        .ok_or_else(|| Error::Validation("package has no officeDocument relationship".into()))?;
    let mime = package.content_type_for(&main)?;
    if mime.as_deref() != Some(DOCX_MAIN) && !(allow_template && mime.as_deref() == Some(DOTX_MAIN))
    {
        let required = if allow_template { "DOCX/DOTX" } else { "DOCX" };
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            format!("unsupported main document content type ({required} required)"),
        )
        .into());
    }
    let main_root = package.word(&main)?;
    if main_root.name != "w:document" {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "unsupported WordprocessingML profile; expected Transitional",
        )
        .into());
    }
    Ok((package, main, main_root))
}

pub(crate) enum CompileSource<'a> {
    Fresh,
    Template {
        bytes: &'a [u8],
        edits: Option<&'a crate::template::TemplateEdits>,
    },
}

pub(crate) fn compile(
    doc: &ResolvedDocument,
    source: CompileSource<'_>,
    options: &crate::application::RenderOptions,
) -> Result<crate::opc::PackagePlan> {
    doc.validate()?;
    if options.profile != OutputProfile::Transitional {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "Strict output is not implemented",
        )
        .at("options.profile")
        .into());
    }
    let fresh = matches!(source, CompileSource::Fresh);
    let (bytes, template) = match source {
        CompileSource::Fresh => (include_bytes!("../../assets/default.docx").as_slice(), None),
        CompileSource::Template { bytes, edits } => (bytes, edits),
    };
    let (mut package, main, _) = open_word_source(bytes, options.read_limits, true)?;
    package.content_type(&main, DOCX_MAIN)?;
    let drawing = crate::ooxml::defaults::prepare(&mut package, &main)?;
    let mut stories = vec![main.clone()];
    let scope = template.map(|edits| edits.scope).unwrap_or_default();
    let kinds: &[&str] = match scope {
        crate::template::EditScope::Body => &[],
        crate::template::EditScope::BodyHeadersFooters => &["header", "footer"],
        crate::template::EditScope::SupportedStories => {
            &["header", "footer", "comments", "footnotes", "endnotes"]
        }
    };
    for kind in kinds {
        for (id, _) in package.relationships(&main, kind)? {
            if let Some((_, path)) = package.related(&main, &format!("#{id}"))?
                && !stories.contains(&path)
            {
                stories.push(path);
            }
        }
    }
    if let Some(t) = template {
        let mut controls =
            crate::template::ContentControlEdits::new(&t.content_control_replacements)?;
        for path in &stories {
            let mut root = package.word(path)?;
            let mut dirty = crate::template::replacements(&mut root, &t.replacements)?;
            dirty |= controls.apply(&mut root).map_err(|error| error.at(path))?;
            if let Some(action) = t.text_revision_action {
                dirty |= crate::template::edit_text_revisions(&mut root, action)
                    .map_err(|error| error.at(path))?;
            }
            if dirty {
                package.put_word(path, root)?;
            }
        }
        controls.finish()?;
    }
    let styles = crate::ooxml::wordprocessing::styles::apply(
        &mut package,
        &main,
        &doc.styles,
        &doc.numbering,
        doc.defaults.as_ref(),
    )?;
    if let Some(defaults) = &doc.defaults {
        defaults::apply_settings(&mut package, &main, defaults)?;
    }
    properties::metadata(
        &mut package,
        doc,
        options.timestamp.as_deref().unwrap_or(&doc.timestamp),
    )?;
    let media = resources::MediaRegistry::from_package(&package)?;
    let ranges = ranges::RangeCatalog::new(&package, &main, doc)?;
    let notes = notes::NoteCatalog::new(&package, &main, doc)?;
    let mut renderer = Compiler {
        package,
        media,
        stories: Default::default(),
        header_stories: Default::default(),
        ranges,
        styles: style_catalog::StyleCatalog::new(&styles)?,
        comment_defaults: Some(CommentDefaults {
            author: doc.comment_author(),
            timestamp: doc.timestamp(),
        }),
        main: main.clone(),
        drawing: DrawingId::new(drawing),
        revisions: None,
        content_control_ids: None,
    };
    let mut root = renderer.package.word(&main)?;
    root.namespaces();
    let body = root
        .child_mut("w:body")
        .ok_or_else(|| Error::Validation("document has no body".into()))?;
    let mut props = body
        .child("w:sectPr")
        .cloned()
        .unwrap_or_else(|| Node::new("w:sectPr"));
    body.remove("w:sectPr");
    let mut paragraph_cursor = template.and_then(|t| t.paragraph_index);
    let count = doc.sections.len();
    let body_story = renderer.stories.register(
        StoryKind::Body,
        crate::opc::PartName::new(&main)?,
        StoryContent::Blocks,
    )?;
    for (index, section) in doc.sections.iter().enumerate() {
        props = section_properties(props, &section.properties)?;
        let width = usable_width(&props);
        for (kind, headers) in [("header", &section.headers), ("footer", &section.footers)] {
            for (typ, hf) in headers {
                let tag = format!("w:{kind}Reference");
                let existing = props
                    .elements()
                    .find(|n| n.name == tag && n.get("w:type") == Some(typ.xml()))
                    .and_then(|n| n.get("r:id"))
                    .map(str::to_owned);
                let id = renderer.header_footer(&hf.blocks, kind, width, existing.as_deref())?;
                props.children.retain(|c|!matches!(c,Content::Element(n) if n.name==tag&&n.get("w:type")==Some(typ.xml())));
                props.children.insert(
                    0,
                    Content::Element(Node::new(tag).attr("w:type", typ.xml()).attr("r:id", id)),
                );
                if *typ == HeaderFooterType::Even {
                    let settings_path = renderer
                        .package
                        .related(&main, "settings")?
                        .map(|(_, p)| p)
                        .unwrap_or("word/settings.xml".into());
                    let mut settings = renderer.package.word(&settings_path)?;
                    settings.replace(Node::new("w:evenAndOddHeaders"));
                    renderer.package.put_word(&settings_path, settings)?;
                }
            }
        }
        let context = renderer.context(body_story, width);
        let mut insert = |block: Node| {
            let insertion = paragraph_cursor
                .and_then(|index| {
                    body.children
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| matches!(c,Content::Element(n) if n.name=="w:p"))
                        .nth(index)
                        .map(|(i, _)| i)
                })
                .unwrap_or(body.children.len());
            let is_paragraph = block.name == "w:p";
            body.children.insert(insertion, Content::Element(block));
            if is_paragraph && let Some(cursor) = &mut paragraph_cursor {
                *cursor += 1;
            }
        };
        for block in renderer.blocks(&section.blocks, &context)? {
            insert(block);
        }
        if index + 1 < count {
            insert(Node::new("w:p").with(Node::new("w:pPr").with(props.clone())));
            if fresh {
                // Unlike page layout, these references are local to the section.
                // Missing references inherit in Word; declared stories get new parts.
                props.remove("w:headerReference");
                props.remove("w:footerReference");
            }
        }
    }
    let note_width = usable_width(&props);
    body.push(props);
    renderer.package.put_word(&main, root)?;
    renderer.compile_notes(notes, doc.notes(), note_width)?;
    crate::opc::PackagePlan::new(renderer.package)
}

pub(crate) fn root(name: &str) -> Node {
    let mut node = Node::new(name);
    node.namespaces();
    node
}

mod compatibility;
pub(crate) mod schema;
pub(crate) trait WordParts {
    fn word(&self, path: &str) -> Result<Node>;
    fn put_word(&mut self, path: impl Into<String>, node: Node) -> Result<()>;
}
impl WordParts for Package {
    fn word(&self, path: &str) -> Result<Node> {
        let mut node = self.xml(path)?;
        compatibility::check(self, path, &node)?;
        node.configure_ordering(schema::order);
        Ok(node)
    }
    fn put_word(&mut self, path: impl Into<String>, mut node: Node) -> Result<()> {
        node.configure_ordering(schema::order);
        node.sort_properties();
        self.put(path, node)
    }
}

mod values;

mod resources;
pub(crate) struct PartContext {
    story: StoryId,
    kind: StoryKind,
    source: crate::opc::PartName,
    width: Twips,
    content: StoryContent,
}
impl Compiler<'_> {
    fn context(&self, story: StoryId, width: i64) -> PartContext {
        let descriptor = self.stories.get(story).expect("registered logical story");
        PartContext {
            story,
            kind: descriptor.kind,
            source: descriptor.owner.clone(),
            width: Twips::new(width),
            content: descriptor.content,
        }
    }
}

mod style_catalog;
