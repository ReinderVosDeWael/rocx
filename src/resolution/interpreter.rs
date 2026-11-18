//! Interpret authoring programs into the pure document domain.
use crate::{
    Block, BlockChildren, Break, Document, Error, Footer, Header, ImageData, ImageRun, Inline,
    Paragraph, ParagraphContent, ResolvedBlock, ResolvedComment, ResolvedDocument,
    ResolvedHeaderFooter, ResolvedInline, ResolvedParagraph, ResolvedSection, ResolvedTable,
    ResolvedTableCell, ResolvedTableRow, Result, RunFormat, Section, Tab, Table, TableCell,
    TableRow, TextRun, Value,
};
use futures_util::future::{BoxFuture, try_join_all};
impl Table {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTable>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let (rows, properties) = futures_util::future::try_join(
            field(self.rows, format!("{path}.rows")),
            field(self.properties, format!("{path}.properties")),
        )
        .await?;
        let rows: Vec<_> = try_join_all(rows.into_iter().enumerate().map(|(i, row)| {
            let p = format!("{path}.rows[{i}]");
            async move {
                row.resolve()
                    .await
                    .map_err(|e| e.at(&p))?
                    .resolve_at(p)
                    .await
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect();
        ResolvedTable::checked(
            rows,
            properties.column_widths,
            properties.layout,
            properties.style,
            properties.borders,
        )
        .map_err(|d| Error::from(d).at(path))
    }
}
impl TableRow {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTableRow>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let children = self
            .children
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.children")))?;
        let cells = try_join_all(children.into_iter().enumerate().map(|(i, cell)| {
            let p = format!("{path}.children[{i}]");
            async move {
                cell.resolve()
                    .await
                    .map_err(|e| e.at(&p))?
                    .resolve_at(p)
                    .await
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect();
        Ok(Some(ResolvedTableRow { cells }))
    }
}
impl TableCell {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTableCell>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let (children, properties) = futures_util::future::try_join(
            field(self.children, format!("{path}.children")),
            field(self.properties, format!("{path}.properties")),
        )
        .await?;
        properties
            .validate()
            .map_err(|e| Error::from(e).at(format!("{path}.properties")))?;
        let blocks = resolve_blocks(children, format!("{path}.children")).await?;
        Ok(Some(ResolvedTableCell {
            blocks,
            grid_span: properties.grid_span.unwrap_or(1),
            vmerge: properties.vmerge,
            borders: properties.borders,
            background_color: properties.background_color,
            vertical_alignment: properties.vertical_alignment,
        }))
    }
}
impl ImageRun {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedInline>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let (data, transformation, alt_text) = futures_util::future::try_join3(
            async {
                self.data
                    .resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.data")))
            },
            async {
                self.transformation
                    .resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.transformation")))
            },
            async {
                self.alt_text
                    .resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.alt_text")))
            },
        )
        .await?;
        let (bytes, name) = match data {
            ImageData::Bytes(b) => (b, None),
            ImageData::Path(p) => {
                let b = crate::adapters::io::read(&p).map_err(|e| e.at(format!("{path}.data")))?;
                (b, p.file_name().map(|s| s.to_string_lossy().into_owned()))
            }
        };
        let image = crate::adapters::inspect_image(bytes, name, transformation, alt_text)
            .map_err(|e| e.at(path))?;
        Ok(Some(ResolvedInline::Image {
            image,
            format: RunFormat::default(),
            comment: None,
        }))
    }
}
macro_rules! resolve_header_footer {
    ($t:ty) => {
        impl $t {
            pub(crate) async fn resolve_at(
                self,
                path: String,
            ) -> Result<Option<ResolvedHeaderFooter>> {
                if !self.condition.evaluate() {
                    return Ok(None);
                }
                let path = format!("{path}.children");
                let children = self.children.resolve().await.map_err(|e| e.at(&path))?;
                Ok(Some(ResolvedHeaderFooter {
                    blocks: resolve_blocks(children, path).await?,
                }))
            }
        }
    };
}
resolve_header_footer!(Header);
resolve_header_footer!(Footer);
impl Tab {
    pub(crate) async fn resolve_at(self, _path: String) -> Result<Option<ResolvedInline>> {
        Ok(self.condition.evaluate().then(|| ResolvedInline::Tab {
            format: RunFormat::default(),
            comment: None,
        }))
    }
}
impl Break {
    pub(crate) async fn resolve_at(self, _path: String) -> Result<Option<ResolvedInline>> {
        Ok(self.condition.evaluate().then(|| ResolvedInline::Break {
            kind: self.r#type,
            format: RunFormat::default(),
            comment: None,
        }))
    }
}
impl Document {
    pub async fn resolve(self) -> Result<ResolvedDocument> {
        crate::resolution::in_pass(self.resolve_in_pass()).await
    }
}
impl Document {
    async fn resolve_in_pass(self) -> Result<ResolvedDocument> {
        let sections = try_join_all(self.sections.into_iter().enumerate().map(
            |(index, section)| async move {
                let path = format!("sections[{index}]");
                section
                    .resolve()
                    .await
                    .map_err(|e| e.at(&path))?
                    .resolve_at(path)
                    .await
            },
        ));
        let ((metadata, comment_author, timestamp), (sections, styles, numbering)) =
            futures_util::future::try_join(
                futures_util::future::try_join3(
                    field(self.metadata, "metadata"),
                    field(self.comment_author, "comment_author"),
                    field(self.timestamp, "timestamp"),
                ),
                futures_util::future::try_join3(
                    sections,
                    field(self.styles, "styles"),
                    field(self.numbering, "numbering"),
                ),
            )
            .await?;
        let document = ResolvedDocument {
            sections: sections.into_iter().flatten().collect(),
            metadata,
            comment_author,
            styles,
            numbering,
            timestamp: timestamp.unwrap_or_else(crate::adapters::io::timestamp),
        };
        document.validate()?;
        Ok(document)
    }
}
impl Section {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedSection>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let content = async {
            let child_path = format!("{path}.children");
            let children = self
                .children
                .resolve()
                .await
                .map_err(|e| e.at(&child_path))?;
            resolve_blocks(children, child_path).await
        };
        let header_work = async {
            Ok::<_, Error>(
                try_join_all(self.headers.into_iter().map(|(kind, header)| {
                    let path = format!("{path}.headers.{}", kind.xml());
                    async move {
                        Ok::<_, Error>((
                            kind,
                            header
                                .resolve()
                                .await
                                .map_err(|e| e.at(&path))?
                                .resolve_at(path)
                                .await?,
                        ))
                    }
                }))
                .await?
                .into_iter()
                .filter_map(|(kind, h)| h.map(|h| (kind, h)))
                .collect(),
            )
        };
        let footer_work = async {
            Ok::<_, Error>(
                try_join_all(self.footers.into_iter().map(|(kind, footer)| {
                    let path = format!("{path}.footers.{}", kind.xml());
                    async move {
                        Ok::<_, Error>((
                            kind,
                            footer
                                .resolve()
                                .await
                                .map_err(|e| e.at(&path))?
                                .resolve_at(path)
                                .await?,
                        ))
                    }
                }))
                .await?
                .into_iter()
                .filter_map(|(kind, h)| h.map(|h| (kind, h)))
                .collect(),
            )
        };
        let ((blocks, headers, footers), properties) = futures_util::future::try_join(
            futures_util::future::try_join3(content, header_work, footer_work),
            field(self.properties, format!("{path}.properties")),
        )
        .await?;
        if let Some(properties) = &properties {
            properties
                .validate()
                .map_err(|error| Error::from(error).at(format!("{path}.properties")))?;
        }
        Ok(Some(ResolvedSection {
            blocks,
            properties,
            headers,
            footers,
        }))
    }
}
impl Paragraph {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedParagraph>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let content = async {
            Ok(match self.content {
                ParagraphContent::Text(text) => {
                    let path = format!("{path}.text");
                    let text = text.resolve().await.map_err(|e| e.at(&path))?;
                    crate::xml::validate_text(&text).map_err(|e| e.at(&path))?;
                    if text.is_empty() {
                        Vec::new()
                    } else {
                        vec![ResolvedInline::Text {
                            text,
                            format: RunFormat::default(),
                            comment: None,
                        }]
                    }
                }
                ParagraphContent::Inlines(children) => {
                    let path = format!("{path}.children");
                    let children = children.resolve().await.map_err(|e| e.at(&path))?;
                    resolve_inlines(children, path, true).await?
                }
            })
        };
        let (runs, comment, (format, numbering)): (Vec<ResolvedInline>, _, _) =
            futures_util::future::try_join3(
                content,
                resolve_comment(self.comment_text, self.comment_author, &path),
                async {
                    futures_util::future::try_join(
                        field(self.format, format!("{path}.format")),
                        field(self.numbering, format!("{path}.numbering")),
                    )
                    .await
                },
            )
            .await?;
        format
            .validate()
            .map_err(|e| Error::from(e).at(format!("{path}.format")))?;
        let text = crate::inline_text(&runs);
        Ok(Some(ResolvedParagraph {
            text,
            runs,
            comment,
            numbering,
            format,
        }))
    }
}
impl crate::Hyperlink {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedInline>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let children = async {
            let children = field(self.children, format!("{path}.children")).await?;
            resolve_inlines(children, format!("{path}.children"), false).await
        };
        let (target, children, properties) = futures_util::future::try_join3(
            field(self.target, format!("{path}.target")),
            children,
            field(self.properties, format!("{path}.properties")),
        )
        .await?;
        if let Some(tooltip) = &properties.tooltip {
            crate::xml::validate_text(tooltip)
                .map_err(|e| e.at(format!("{path}.properties.tooltip")))?;
        }
        let link = crate::ResolvedHyperlink::new(target, children, properties)
            .map_err(|e| Error::from(e).at(path))?;
        Ok(Some(ResolvedInline::Hyperlink(link)))
    }
}
fn resolve_inlines(
    children: crate::InlineChildren,
    path: String,
    allow_links: bool,
) -> BoxFuture<'static, Result<Vec<ResolvedInline>>> {
    Box::pin(async move {
        Ok(
            try_join_all(children.into_iter().enumerate().map(|(index, child)| {
                let path = format!("{path}[{index}]");
                async move {
                    match child.resolve().await.map_err(|e| e.at(&path))? {
                        Inline::Resolved(run) => Ok(Some(run)),
                        Inline::Hyperlink(link) if !allow_links => {
                            if !link.condition.evaluate() {
                                return Ok(None);
                            }
                            Err(crate::Diagnostic::new(
                                crate::Rule::InvalidContent,
                                "nested hyperlinks are not supported",
                            )
                            .at(path)
                            .into())
                        }
                        Inline::Hyperlink(link) => link.resolve_at(path).await,
                        Inline::TextRun(run) => run.resolve_at(path).await,
                        Inline::Tab(tab) => tab.resolve_at(path).await,
                        Inline::Break(br) => br.resolve_at(path).await,
                        Inline::ImageRun(image) => image.resolve_at(path).await,
                    }
                }
            }))
            .await?
            .into_iter()
            .flatten()
            .collect(),
        )
    })
}
impl TextRun {
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedInline>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let text_path = format!("{path}.text");
        let text_work = async {
            let text = self.text.resolve().await.map_err(|e| e.at(&text_path))?;
            crate::xml::validate_text(&text).map_err(|e| e.at(&text_path))?;
            Ok::<_, Error>(text)
        };
        let (text, comment, format) = futures_util::future::try_join3(
            text_work,
            resolve_comment(self.comment_text, self.comment_author, &path),
            async {
                self.format
                    .resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.format")))
            },
        )
        .await?;
        format
            .validate()
            .map_err(|e| Error::from(e).at(format!("{path}.format")))?;
        Ok(Some(ResolvedInline::Text {
            text,
            comment,
            format,
        }))
    }
}
pub(crate) async fn resolve_comment(
    text: Option<Value<String>>,
    author: Option<Value<String>>,
    path: &str,
) -> Result<Option<ResolvedComment>> {
    let Some(text) = text else { return Ok(None) };
    let text_work = async {
        text.resolve()
            .await
            .map_err(|e| e.at(format!("{path}.comment_text")))
    };
    let author_work = async {
        match author {
            Some(a) => Ok(Some(
                a.resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.comment_author")))?,
            )),
            None => Ok::<_, Error>(None),
        }
    };
    let (text, author) = futures_util::future::try_join(text_work, author_work).await?;
    crate::xml::validate_text(&text).map_err(|e| e.at(format!("{path}.comment_text")))?;
    if let Some(a) = &author {
        crate::xml::validate_text(a).map_err(|e| e.at(format!("{path}.comment_author")))?;
    }
    Ok((!text.is_empty()).then_some(ResolvedComment { text, author }))
}
pub(crate) fn resolve_blocks(
    children: BlockChildren,
    path: String,
) -> BoxFuture<'static, Result<Vec<ResolvedBlock>>> {
    Box::pin(async move {
        Ok(try_join_all(children.into_iter().enumerate().map(|(i, b)| {
            let path = format!("{path}[{i}]");
            async move {
                match b.resolve().await.map_err(|e| e.at(&path))? {
                    Block::Resolved(block) => Ok(Some(block)),
                    Block::Paragraph(p) => {
                        Ok::<_, Error>(p.resolve_at(path).await?.map(ResolvedBlock::Paragraph))
                    }
                    Block::Table(t) => Ok(t.resolve_at(path).await?.map(ResolvedBlock::Table)),
                }
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect())
    })
}

async fn field<T>(value: Value<T>, path: impl Into<String>) -> Result<T> {
    let path = path.into();
    value.resolve().await.map_err(|error| error.at(path))
}
