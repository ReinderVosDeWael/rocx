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
pub(crate) struct Compiler<'a> {
    package: Package,
    main: String,
    drawing: DrawingId,
    media: resources::MediaRegistry,
    stories: std::collections::BTreeMap<(String, StoryKind), StoryId>,
    styles: style_catalog::StyleCatalog,
    document: &'a ResolvedDocument,
}
pub(crate) fn compile(
    doc: &ResolvedDocument,
    bytes: &[u8],
    template: Option<&crate::template::TemplateEdits>,
    options: &crate::application::RenderOptions,
) -> Result<crate::opc::PackagePlan> {
    if options.profile != OutputProfile::Transitional {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "Strict output is not implemented",
        )
        .into());
    }
    let mut package = Package::load_with_limits(bytes, options.read_limits)?;
    let main = package
        .related("", "officeDocument")?
        .map(|(_, p)| p)
        .ok_or_else(|| Error::Validation("package has no officeDocument relationship".into()))?;
    let mime = package.content_type_for(&main)?;
    if !matches!(
        mime.as_deref(),
        Some(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml"
        )
    ) {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "unsupported main document content type (DOCX/DOTX required)",
        )
        .into());
    }
    package.content_type(
        &main,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
    )?;
    let main_root = package.word(&main)?;
    if main_root.name != "w:document" {
        return Err(crate::Diagnostic::new(
            crate::Rule::UnsupportedProfile,
            "unsupported WordprocessingML profile; expected Transitional",
        )
        .into());
    }
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
            if let Some((_, path)) = package.related(&main, &format!("#{id}"))? {
                stories.push(path);
            }
        }
    }
    if let Some(t) = template {
        for path in &stories {
            let mut root = package.word(path)?;
            crate::template::replacements(&mut root, &t.replacements)?;
            package.put_word(path, root)?;
        }
    }
    let styles = crate::ooxml::wordprocessing::styles::apply(
        &mut package,
        &main,
        &doc.styles,
        &doc.numbering,
    )?;
    properties::metadata(
        &mut package,
        doc,
        options.timestamp.as_deref().unwrap_or(&doc.timestamp),
    )?;
    let media = resources::MediaRegistry::from_package(&package)?;
    let mut renderer = Compiler {
        package,
        media,
        stories: Default::default(),
        styles: style_catalog::StyleCatalog::new(&styles)?,
        document: doc,
        main: main.clone(),
        drawing: DrawingId::new(drawing),
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
        let context = renderer.context(&main, StoryKind::Body, width);
        for block in renderer.blocks(&section.blocks, &context)? {
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
        }
        if index + 1 < count {
            body.push(Node::new("w:p").with(Node::new("w:pPr").with(props.clone())));
        }
    }
    body.push(props);
    renderer.package.put_word(&main, root)?;
    crate::opc::PackagePlan::new(renderer.package)
}

pub(crate) fn root(name: &str) -> Node {
    let mut node = Node::new(name);
    node.namespaces();
    node
}

pub(crate) mod schema;
pub(crate) trait WordParts {
    fn word(&self, path: &str) -> Result<Node>;
    fn put_word(&mut self, path: impl Into<String>, node: Node) -> Result<()>;
}
impl WordParts for Package {
    fn word(&self, path: &str) -> Result<Node> {
        let mut node = self.xml(path)?;
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
}
impl Compiler<'_> {
    fn context(&mut self, source: &str, kind: StoryKind, width: i64) -> PartContext {
        let next = StoryId::new(self.stories.len() as u32);
        let story = *self.stories.entry((source.into(), kind)).or_insert(next);
        PartContext {
            story,
            kind,
            source: crate::opc::PartName::new(source)
                .expect("package part names validated on load/creation"),
            width: Twips::new(width),
        }
    }
}

mod style_catalog;
