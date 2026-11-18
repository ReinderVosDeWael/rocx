//! Interpret authoring programs into the pure document domain.
use crate::document::Metadata;
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
        let rows = self
            .rows
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.rows")))?;
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
            self.column_widths,
            self.layout,
            self.style,
            self.borders,
        )
        .map_err(|d| {
            let location = d
                .location
                .as_ref()
                .map(|l| format!("{path}.{l}"))
                .unwrap_or(path);
            Error::from(d).at(location)
        })
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
        let span = self.grid_span.unwrap_or(1);
        if span == 0 {
            return Err(Error::Validation("grid_span must be positive".into())
                .at(format!("{path}.grid_span")));
        }
        let children = self
            .children
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.children")))?;
        let blocks = resolve_blocks(children, format!("{path}.children")).await?;
        Ok(Some(ResolvedTableCell {
            blocks,
            grid_span: span,
            vmerge: self.vmerge,
            borders: self.borders,
            background_color: self.background_color,
            vertical_alignment: self.vertical_alignment,
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
        ))
        .await?;
        Ok(ResolvedDocument {
            sections: sections.into_iter().flatten().collect(),
            metadata: Metadata {
                creator: self.creator,
                title: self.title,
                subject: self.subject,
                description: self.description,
                keywords: self.keywords,
                category: self.category,
                comments: self.comments,
                version: self.version,
            },
            comment_author: self.comment_author,
            styles: self.styles,
            numbering: self.numbering,
            timestamp: self
                .timestamp
                .unwrap_or_else(crate::adapters::io::timestamp),
        })
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
        let (blocks, headers, footers) =
            futures_util::future::try_join3(content, header_work, footer_work).await?;
        Ok(Some(ResolvedSection {
            blocks,
            properties: self.properties,
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
                    try_join_all(children.into_iter().enumerate().map(|(index, child)| {
                        let path = format!("{path}[{index}]");
                        async move {
                            match child.resolve().await.map_err(|e| e.at(&path))? {
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
                    .collect()
                }
            })
        };
        let (runs, comment, format): (Vec<ResolvedInline>, _, _) = futures_util::future::try_join3(
            content,
            resolve_comment(self.comment_text, self.comment_author, &path),
            async {
                self.format
                    .resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.format")))
            },
        )
        .await?;
        let text = runs.iter().map(ResolvedInline::text).collect();
        Ok(Some(ResolvedParagraph {
            text,
            runs,
            comment,
            numbering: self.numbering,
            format,
        }))
    }
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
