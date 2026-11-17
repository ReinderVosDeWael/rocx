//! WordprocessingML serialization and package assembly.
//!
//! References: ECMA-376 Part 1 (2016), §§17.3 (paragraphs/runs), 17.4
//! (tables), 17.6 (sections), 17.10 (headers/footers), 17.13.4 (comments),
//! and 20.4 (WordprocessingML drawings).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
//! These citations identify the relevant vocabulary, not full conformance.
use crate::model::ResolvedInlineKind;
use crate::package::Package;
use crate::xml::{Content, Node, val};
use crate::*;
pub(crate) fn validate_text(text: &str) -> Result<()> {
    for (byte_offset, character) in text.char_indices() {
        if !matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
        {
            return Err(Error::InvalidXmlCharacter {
                character,
                byte_offset,
            });
        }
    }
    Ok(())
}

pub(crate) fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86400) as i64;
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds % 86400 / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}
pub(crate) fn text_run(text: &str) -> Node {
    let mut run = Node::new("w:r");
    let mut segment = String::new();
    for c in text.chars() {
        if matches!(c, '\t' | '\r' | '\n') {
            if !segment.is_empty() {
                run.push(
                    Node::new("w:t")
                        .attr("xml:space", "preserve")
                        .text(std::mem::take(&mut segment)),
                );
            }
            run.push(Node::new(if c == '\t' { "w:tab" } else { "w:br" }));
        } else {
            segment.push(c);
        }
    }
    if !segment.is_empty() {
        run.push(Node::new("w:t").attr("xml:space", "preserve").text(segment));
    }
    run
}
struct Renderer<'a> {
    package: Package,
    styles: Node,
    document: &'a ResolvedDocument,
}
impl Renderer<'_> {
    fn comment(
        &mut self,
        nodes: Vec<Node>,
        comment: &Option<ResolvedComment>,
    ) -> Result<Vec<Node>> {
        let Some(comment) = comment else {
            return Ok(nodes);
        };
        if nodes.is_empty() {
            return Err(Error::Validation(
                "comments must be anchored to at least one run".into(),
            ));
        }
        let comment_path = self
            .package
            .related("word/document.xml", "comments")?
            .map(|(_, path)| path)
            .unwrap_or("word/comments.xml".into());
        let path = comment_path.as_str();
        let mut comments = if self.package.parts.contains_key(path) {
            self.package.xml(path)?
        } else {
            Package::root("w:comments")
        };
        let id = comments
            .elements()
            .filter_map(|n| n.get("w:id"))
            .filter_map(|s| s.parse::<u32>().ok())
            .max()
            .map_or(0, |n| n + 1);
        let author = comment
            .author
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(self.document.comment_author.as_deref())
            .unwrap_or("");
        let mut node = Node::new("w:comment")
            .attr("w:id", id)
            .attr("w:author", author)
            .attr("w:initials", "")
            .attr("w:date", &self.document.timestamp);
        for line in comment.text.split('\n') {
            node.push(
                Node::new("w:p")
                    .with(Node::new("w:pPr").with(val("pStyle", "CommentText")))
                    .with(text_run(line)),
            );
        }
        comments.push(node);
        self.package.put(path, comments)?;
        self.package.content_type(
            path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
        )?;
        self.package.relation(
            "word/document.xml",
            "comments",
            path.strip_prefix("word/").unwrap_or(path),
        )?;
        let mut out = vec![Node::new("w:commentRangeStart").attr("w:id", id)];
        out.extend(nodes);
        out.push(Node::new("w:commentRangeEnd").attr("w:id", id));
        out.push(
            Node::new("w:r")
                .with(Node::new("w:rPr").with(val("rStyle", "CommentReference")))
                .with(Node::new("w:commentReference").attr("w:id", id)),
        );
        Ok(out)
    }
    fn image(&mut self, image: &ResolvedImage, owner: &str) -> Result<Node> {
        let existing = self
            .package
            .parts
            .iter()
            .find(|(p, b)| p.starts_with("word/media/") && **b == image.data)
            .map(|(p, _)| p.clone());
        let path =
            existing.unwrap_or_else(|| self.package.unique("word/media/image", &image.extension));
        self.package.parts.insert(path.clone(), image.data.clone());
        self.package.content_type(&path, &image.content_type)?;
        let id = self
            .package
            .relation(owner, "image", path.strip_prefix("word/").unwrap())?;
        self.package.drawing += 1;
        let drawing = self.package.drawing;
        let alt = image.alt_text.as_ref();
        let name = alt.and_then(|a| a.name.as_deref()).unwrap_or(&image.name);
        let docpr = Node::new("wp:docPr")
            .attr("id", drawing)
            .attr("name", name)
            .attr(
                "descr",
                alt.and_then(|a| a.description.as_deref()).unwrap_or(""),
            )
            .attr("title", alt.and_then(|a| a.title.as_deref()).unwrap_or(""));
        let pic = Node::new("pic:pic")
            .with(
                Node::new("pic:nvPicPr")
                    .with(
                        Node::new("pic:cNvPr")
                            .attr("id", 0)
                            .attr("name", &image.name),
                    )
                    .with(Node::new("pic:cNvPicPr")),
            )
            .with(
                Node::new("pic:blipFill")
                    .with(Node::new("a:blip").attr("r:embed", id))
                    .with(Node::new("a:stretch").with(Node::new("a:fillRect"))),
            )
            .with(
                Node::new("pic:spPr")
                    .with(
                        Node::new("a:xfrm")
                            .with(Node::new("a:off").attr("x", 0).attr("y", 0))
                            .with(
                                Node::new("a:ext")
                                    .attr("cx", image.width)
                                    .attr("cy", image.height),
                            ),
                    )
                    .with(
                        Node::new("a:prstGeom")
                            .attr("prst", "rect")
                            .with(Node::new("a:avLst")),
                    ),
            );
        let inline = Node::new("wp:inline")
            .with(
                Node::new("wp:extent")
                    .attr("cx", image.width)
                    .attr("cy", image.height),
            )
            .with(docpr)
            .with(
                Node::new("wp:cNvGraphicFramePr")
                    .with(Node::new("a:graphicFrameLocks").attr("noChangeAspect", 1)),
            )
            .with(
                Node::new("a:graphic").with(
                    Node::new("a:graphicData")
                        .attr(
                            "uri",
                            "http://schemas.openxmlformats.org/drawingml/2006/picture",
                        )
                        .with(pic),
                ),
            );
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(inline)))
    }
    fn paragraph(&mut self, p: &ResolvedParagraph, owner: &str) -> Result<Node> {
        let mut node = Node::new("w:p");
        let mut props = crate::format::paragraph(&p.format, 20.)?;
        if let Some(style) = props.child_mut("w:pStyle") {
            style.set(
                "w:val",
                crate::styles::style_id(&self.styles, style.get("w:val").unwrap())?,
            );
        }
        if let Some(num) = &p.numbering {
            props.push(
                Node::new("w:numPr")
                    .with(val("ilvl", num.level))
                    .with(val("numId", num.id)),
            );
        }
        if !props.children.is_empty() {
            node.push(props);
        }
        let mut runs = Vec::new();
        for run in &p.runs {
            let mut n = match &run.kind {
                ResolvedInlineKind::Text => text_run(&run.text),
                ResolvedInlineKind::Tab => Node::new("w:r").with(Node::new("w:tab")),
                ResolvedInlineKind::Break(kind) => {
                    Node::new("w:r").with(Node::new("w:br").attr("w:type", kind.xml()))
                }
                ResolvedInlineKind::Image(image) => self.image(image, owner)?,
            };
            let rpr = crate::format::run(&run.format)?;
            if !rpr.children.is_empty() {
                n.children.insert(0, Content::Element(rpr));
            }
            runs.extend(self.comment(vec![n], &run.comment)?);
        }
        for n in self.comment(runs, &p.comment)? {
            node.push(n);
        }
        Ok(node)
    }
    fn blocks(&mut self, blocks: &[ResolvedBlock], owner: &str, width: i64) -> Result<Vec<Node>> {
        blocks
            .iter()
            .map(|b| match b {
                ResolvedBlock::Paragraph(p) => self.paragraph(p, owner),
                ResolvedBlock::Table(t) => self.table(t, owner, width),
            })
            .collect()
    }
    fn table(&mut self, t: &ResolvedTable, owner: &str, width: i64) -> Result<Node> {
        let widths: Vec<i64> = if t.layout != Some(TableLayout::Autofit) {
            t.column_widths
                .as_ref()
                .map(|w| w.iter().map(|n| *n as i64).collect())
                .unwrap_or_else(|| {
                    vec![
                        if t.columns == 0 {
                            0
                        } else {
                            width / t.columns as i64
                        };
                        t.columns
                    ]
                })
        } else {
            vec![
                if t.columns == 0 {
                    0
                } else {
                    width / t.columns as i64
                };
                t.columns
            ]
        };
        let mut props = Node::new("w:tblPr");
        if let Some(style) = &t.style {
            props.push(val(
                "tblStyle",
                crate::styles::style_id(&self.styles, style)?,
            ));
        }
        props.push(
            Node::new("w:tblW")
                .attr(
                    "w:w",
                    if t.column_widths.is_some() && t.layout != Some(TableLayout::Autofit) {
                        widths.iter().sum()
                    } else {
                        0
                    },
                )
                .attr(
                    "w:type",
                    if t.column_widths.is_some() && t.layout != Some(TableLayout::Autofit) {
                        "dxa"
                    } else {
                        "auto"
                    },
                ),
        );
        if t.layout.is_some() || t.column_widths.is_some() {
            props.push(
                Node::new("w:tblLayout")
                    .attr("w:type", t.layout.unwrap_or(TableLayout::Fixed).xml()),
            );
        }
        if !t.borders.is_empty() {
            let mut b = Node::new("w:tblBorders");
            for border in &t.borders {
                b.push(
                    Node::new(format!("w:{}", border.side.xml()))
                        .attr("w:val", border.val.xml())
                        .attr("w:sz", border.sz)
                        .attr("w:color", border.hex_color()),
                );
            }
            props.push(b);
        }
        props.push(
            Node::new("w:tblLook")
                .attr("w:firstColumn", 1)
                .attr("w:firstRow", 1)
                .attr("w:lastColumn", 0)
                .attr("w:lastRow", 0)
                .attr("w:noHBand", 0)
                .attr("w:noVBand", 1)
                .attr("w:val", "04A0"),
        );
        let mut grid = Node::new("w:tblGrid");
        for w in &widths {
            grid.push(Node::new("w:gridCol").attr("w:w", w));
        }
        let mut node = Node::new("w:tbl").with(props).with(grid);
        for row in &t.rows {
            let mut tr = Node::new("w:tr");
            let mut column = 0;
            for cell in &row.cells {
                let span = cell.grid_span as usize;
                let cw = widths[column..column + span].iter().sum::<i64>();
                column += span;
                let mut pr = Node::new("w:tcPr")
                    .with(Node::new("w:tcW").attr("w:w", cw).attr("w:type", "dxa"));
                if span > 1 {
                    pr.push(val("gridSpan", span));
                }
                if let Some(merge) = cell.vmerge {
                    pr.push(val("vMerge", merge.xml()));
                }
                if let Some(color) = cell.background_color {
                    pr.push(crate::format::shading(color));
                }
                if !cell.borders.is_empty() {
                    let mut b = Node::new("w:tcBorders");
                    for border in &cell.borders {
                        b.push(
                            Node::new(format!("w:{}", border.side.xml()))
                                .attr("w:val", border.val.xml())
                                .attr("w:sz", border.sz)
                                .attr("w:color", border.hex_color()),
                        );
                    }
                    pr.push(b);
                }
                if let Some(v) = cell.vertical_alignment {
                    pr.push(val("vAlign", v.xml()));
                }
                let mut tc = Node::new("w:tc").with(pr);
                let blocks = self.blocks(
                    &cell.blocks,
                    owner,
                    if t.columns == 0 {
                        cw
                    } else {
                        (width as f64 / t.columns as f64).round_ties_even() as i64
                    },
                )?;
                if blocks.first().is_none_or(|n| n.name != "w:p") {
                    tc.push(Node::new("w:p"));
                }
                for block in blocks {
                    let table = block.name == "w:tbl";
                    tc.push(block);
                    if table {
                        tc.push(Node::new("w:p"));
                    }
                }
                if tc.elements().last().is_none_or(|n| n.name != "w:p") {
                    tc.push(Node::new("w:p"));
                }
                tr.push(tc);
            }
            while column < t.columns {
                tr.push(
                    Node::new("w:tc")
                        .with(
                            Node::new("w:tcPr").with(
                                Node::new("w:tcW")
                                    .attr("w:w", widths[column])
                                    .attr("w:type", "dxa"),
                            ),
                        )
                        .with(Node::new("w:p")),
                );
                column += 1;
            }
            node.push(tr);
        }
        Ok(node)
    }
    fn header_footer(
        &mut self,
        blocks: &[ResolvedBlock],
        kind: &str,
        width: i64,
        existing: Option<&str>,
    ) -> Result<String> {
        let related = existing
            .map(|id| self.package.related("word/document.xml", &format!("#{id}")))
            .transpose()?
            .flatten();
        let path = related
            .as_ref()
            .map(|(_, path)| path.clone())
            .unwrap_or_else(|| self.package.unique(&format!("word/{kind}"), "xml"));
        let mut root = if related.is_some() {
            self.package.xml(&path)?
        } else {
            Package::root(if kind == "header" { "w:hdr" } else { "w:ftr" })
        };
        if related.is_none() {
            root.push(Node::new("w:p").with(Node::new("w:pPr").with(val(
                "pStyle",
                if kind == "header" { "Header" } else { "Footer" },
            ))));
        }
        for block in self.blocks(blocks, &path, width)? {
            root.push(block);
        }
        self.package.put(&path, root)?;
        self.package.content_type(
            &path,
            &format!("application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}+xml"),
        )?;
        if let Some((id, _)) = related {
            return Ok(id);
        }
        self.package.relation(
            "word/document.xml",
            kind,
            path.strip_prefix("word/").unwrap(),
        )
    }
}
fn section_properties(mut previous: Node, props: &Option<SectionProperties>) -> Result<Node> {
    if let Some(p) = props {
        let size = previous.ensure("w:pgSz");
        let mut w = size
            .get("w:w")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(12240);
        let mut h = size
            .get("w:h")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(15840);
        if let Some(size) = &p.page_size {
            if let Some(v) = size.width {
                w = (v as f64 / 635.).round() as i64;
            }
            if let Some(v) = size.height {
                h = (v as f64 / 635.).round() as i64;
            }
        }
        if let Some(o) = p.page_orientation {
            if (o == Orientation::Landscape && w < h) || (o == Orientation::Portrait && w > h) {
                std::mem::swap(&mut w, &mut h);
            }
            size.set("w:orient", o.xml());
        }
        if w <= 0 || h <= 0 {
            return Err(Error::Validation("page size must be positive".into()));
        }
        size.set("w:w", w);
        size.set("w:h", h);
        if let Some(m) = &p.page_margins {
            for (k, v) in [
                ("top", m.top),
                ("bottom", m.bottom),
                ("left", m.left),
                ("right", m.right),
            ] {
                if let Some(v) = v {
                    previous
                        .ensure("w:pgMar")
                        .set(format!("w:{k}"), (v as f64 / 635.).round() as i64);
                }
            }
        }
        if let Some(n) = &p.page_numbering {
            let node = previous.ensure("w:pgNumType");
            if let Some(start) = n.start {
                node.set("w:start", start);
            }
            if let Some(format) = &n.format {
                node.set("w:fmt", format);
            }
        }
        if let Some(c) = &p.columns {
            let node = previous.ensure("w:cols");
            if let Some(count) = c.count {
                if count == 0 {
                    return Err(Error::Validation("column count must be positive".into()));
                }
                node.set("w:num", count);
            }
            if let Some(space) = c.space {
                node.set("w:space", space);
            }
            if let Some(separator) = c.separator {
                node.set("w:sep", u8::from(separator));
            }
        }
        if let Some(v) = p.vertical_align {
            previous.replace(val("vAlign", v.xml()));
        }
        if let Some(v) = p.title_page {
            previous.remove("w:titlePg");
            if v {
                previous.push(Node::new("w:titlePg"));
            }
        }
        if let Some(t) = p.r#type {
            previous.replace(val("type", t.xml()));
        }
    }
    Ok(previous)
}
fn usable_width(props: &Node) -> i64 {
    let size = props
        .child("w:pgSz")
        .and_then(|n| n.get("w:w"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(12240);
    let margin = props.child("w:pgMar");
    let l = margin
        .and_then(|n| n.get("w:left"))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1800);
    let r = margin
        .and_then(|n| n.get("w:right"))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1800);
    (size - l - r).max(1)
}
fn metadata(package: &mut Package, doc: &ResolvedDocument) -> Result<()> {
    let core_path = package
        .related("", "core-properties")?
        .map(|(_, p)| p)
        .unwrap_or("docProps/core.xml".into());
    let mut core = if package.parts.contains_key(&core_path) {
        package.xml(&core_path)?
    } else {
        Package::root("cp:coreProperties")
    };
    let m = &doc.metadata;
    for (name, value) in [
        ("dc:creator", &m.creator),
        ("dc:title", &m.title),
        ("dc:subject", &m.subject),
        (
            "dc:description",
            if m.comments.as_ref().is_some_and(|s| !s.is_empty()) {
                &m.comments
            } else {
                &m.description
            },
        ),
        ("cp:keywords", &m.keywords),
        ("cp:category", &m.category),
        ("cp:version", &m.version),
    ] {
        if let Some(s) = value {
            if s.is_empty() {
                continue;
            }
            if s.chars().count() > 255 {
                return Err(Error::Validation(format!(
                    "metadata {name} exceeds 255 characters"
                )));
            }
            core.replace(Node::new(name).text(s));
        }
    }
    for name in ["dcterms:created", "dcterms:modified"] {
        core.replace(
            Node::new(name)
                .attr("xsi:type", "dcterms:W3CDTF")
                .text(&doc.timestamp),
        );
    }
    package.put(&core_path, core)?;
    package.relation(
        "",
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties",
        &core_path,
    )?;
    package.content_type(
        &core_path,
        "application/vnd.openxmlformats-package.core-properties+xml",
    )?;
    Ok(())
}
pub(crate) fn render(doc: &ResolvedDocument) -> Result<Vec<u8>> {
    render_with_template(doc, None)
}
pub(crate) fn render_with_template(
    doc: &ResolvedDocument,
    template: Option<&DocumentTemplate>,
) -> Result<Vec<u8>> {
    let bytes = if let Some(t) = template {
        std::fs::read(&t.path)?
    } else {
        include_bytes!("../assets/default.docx").to_vec()
    };
    let mut package = Package::load(&bytes)?;
    if let Some(t) = template {
        for path in package
            .parts
            .keys()
            .filter(|p| {
                p.starts_with("word/")
                    && (p.as_str() == "word/document.xml"
                        || p.starts_with("word/header")
                        || p.starts_with("word/footer"))
                    && p.ends_with(".xml")
            })
            .cloned()
            .collect::<Vec<_>>()
        {
            let mut root = package.xml(&path)?;
            crate::template::replacements(&mut root, &t.replacements)?;
            package.put(path, root)?;
        }
    }
    let styles = crate::styles::apply(&mut package, &doc.styles, &doc.numbering)?;
    metadata(&mut package, doc)?;
    let mut renderer = Renderer {
        package,
        styles,
        document: doc,
    };
    let mut root = renderer.package.xml("word/document.xml")?;
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
                        .related("word/document.xml", "settings")?
                        .map(|(_, p)| p)
                        .unwrap_or("word/settings.xml".into());
                    let mut settings = renderer.package.xml(&settings_path)?;
                    settings.replace(Node::new("w:evenAndOddHeaders"));
                    renderer.package.put(&settings_path, settings)?;
                }
            }
        }
        for block in renderer.blocks(&section.blocks, "word/document.xml", width)? {
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
    renderer.package.put("word/document.xml", root)?;
    renderer.package.finish()
}
