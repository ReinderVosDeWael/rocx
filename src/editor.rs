//! Read and edit an existing Transitional DOCX without rebuilding it.
//!
//! The initial surface covers direct body blocks and paragraph alignment.
//! The package, not the typed inspection view, is the source of output.

use crate::ooxml::{WordParts, open_word_source};
use crate::opc::{Package, PackagePlan, ReadLimits};
use crate::xml::{Content, Node};
use crate::{Alignment, Diagnostic, Error, Result, Rule};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A target valid only for the snapshot that issued it.
#[derive(Clone, Debug)]
pub struct ParagraphAnchor {
    snapshot: Arc<()>,
    body_index: usize,
}

/// Direct body children in document order, excluding the final section properties.
#[derive(Debug)]
#[non_exhaustive]
pub enum BodyBlock {
    Paragraph(ImportedParagraph),
    Table,
    /// An unsupported element, identified by expanded XML name.
    Opaque(String),
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedParagraph {
    pub anchor: ParagraphAnchor,
    /// Text from ordinary runs and hyperlinks; not a rendered-text view.
    pub text: String,
    /// Recognized direct alignment, not inherited/effective alignment.
    pub alignment: Option<Alignment>,
    /// The source value, including an explicit alignment outside rocx's model.
    pub alignment_xml: Option<String>,
    /// Some inline content was omitted from the text projection.
    pub has_opaque_content: bool,
}

#[derive(Clone, Debug)]
pub struct ParagraphAlignmentEdit {
    pub target: ParagraphAnchor,
    pub alignment: Alignment,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct AlignmentChange {
    /// Zero-based index in `ImportedDocument::body()`.
    pub body_index: usize,
    /// The previous direct XML value, or `None` if there was no direct value.
    pub before: Option<String>,
    pub after: Alignment,
}

/// A validated edit batch and its independently writable output.
#[derive(Debug)]
pub struct EditPreview {
    changes: Vec<AlignmentChange>,
    output: PackagePlan,
}

impl EditPreview {
    /// Actual changes in document order. No-op edits are omitted.
    pub fn changes(&self) -> &[AlignmentChange] {
        &self.changes
    }

    pub fn to_docx(&self) -> Result<Vec<u8>> {
        self.output.to_bytes()
    }
}

/// An immutable source snapshot. Opening it does not rewrite the package.
#[derive(Debug)]
pub struct ImportedDocument {
    package: Package,
    main: String,
    body: Vec<BodyBlock>,
    snapshot: Arc<()>,
}

impl ImportedDocument {
    pub fn open(bytes: &[u8]) -> Result<Self> {
        Self::open_with_limits(bytes, ReadLimits::default())
    }

    pub fn open_with_limits(bytes: &[u8], limits: ReadLimits) -> Result<Self> {
        let (package, main, root) = open_word_source(bytes, limits, false)?;
        let snapshot = Arc::new(());
        let body = root
            .child("w:body")
            .ok_or_else(missing_body)?
            .elements()
            .filter(|node| node.name != "w:sectPr")
            .enumerate()
            .map(|(body_index, node)| match node.name.as_str() {
                "w:p" => {
                    let mut text = String::new();
                    let mut has_opaque_content = false;
                    for child in node.elements().filter(|child| child.name != "w:pPr") {
                        read_inline(child, &mut text, &mut has_opaque_content);
                    }
                    let alignment_xml = node
                        .child("w:pPr")
                        .and_then(|props| props.child("w:jc"))
                        .and_then(|jc| jc.get("w:val"));
                    BodyBlock::Paragraph(ImportedParagraph {
                        anchor: ParagraphAnchor {
                            snapshot: snapshot.clone(),
                            body_index,
                        },
                        text,
                        alignment: alignment_xml.and_then(read_alignment),
                        alignment_xml: alignment_xml.map(str::to_owned),
                        has_opaque_content,
                    })
                }
                "w:tbl" => BodyBlock::Table,
                name => BodyBlock::Opaque(name.to_owned()),
            })
            .collect();
        Ok(Self {
            package,
            main,
            body,
            snapshot,
        })
    }

    pub fn body(&self) -> &[BodyBlock] {
        &self.body
    }

    /// Validate and stage the whole batch; neither the snapshot nor the source is modified.
    pub fn preview(&self, edits: &[ParagraphAlignmentEdit]) -> Result<EditPreview> {
        let mut by_index = BTreeMap::new();
        for edit in edits {
            if !Arc::ptr_eq(&self.snapshot, &edit.target.snapshot) {
                return Err(unsupported(
                    "target belongs to a different document snapshot",
                ));
            }
            if by_index
                .insert(edit.target.body_index, edit.alignment)
                .is_some()
            {
                return Err(unsupported("multiple alignment edits target one paragraph"));
            }
        }
        let mut package = self.package.clone();
        let mut changes = Vec::new();
        if !by_index.is_empty() {
            let mut root = package.word(&self.main)?;
            let body = root.child_mut("w:body").ok_or_else(missing_body)?;
            for (body_index, paragraph) in body
                .elements_mut()
                .filter(|node| node.name != "w:sectPr")
                .enumerate()
            {
                let Some(after) = by_index.remove(&body_index) else {
                    continue;
                };
                if paragraph.name != "w:p" {
                    return Err(unsupported("target is not a body paragraph"));
                }
                if let Some(before) = patch_alignment(paragraph, after)? {
                    changes.push(AlignmentChange {
                        body_index,
                        before,
                        after,
                    });
                }
            }
            if !by_index.is_empty() {
                return Err(unsupported("target paragraph not found"));
            }
            if !changes.is_empty() {
                package.put_word(&self.main, root)?;
            }
        }
        Ok(EditPreview {
            changes,
            output: PackagePlan::new(package)?,
        })
    }
}

fn missing_body() -> Error {
    Error::Validation("document has no body".into())
}

fn unsupported(message: &str) -> Error {
    Diagnostic::new(Rule::UnsupportedEdit, message).into()
}

fn read_alignment(value: &str) -> Option<Alignment> {
    use Alignment::*;
    [
        Left,
        Center,
        Right,
        Justify,
        Distribute,
        JustifyMedium,
        JustifyHigh,
        JustifyLow,
        ThaiJustify,
    ]
    .into_iter()
    .find(|alignment| alignment.xml() == value)
}

fn read_inline(node: &Node, text: &mut String, opaque: &mut bool) {
    match node.name.as_str() {
        "w:r" => {
            for item in node.elements() {
                match item.name.as_str() {
                    "w:rPr" => (),
                    "w:t" => {
                        for child in &item.children {
                            match child {
                                Content::Text(value) => text.push_str(value),
                                Content::Element(_) => *opaque = true,
                                Content::Raw(_) => (),
                            }
                        }
                    }
                    "w:tab" => text.push('\t'),
                    "w:cr" => text.push('\n'),
                    "w:br" if item.get("w:type").is_none_or(|v| v == "textWrapping") => {
                        text.push('\n');
                    }
                    _ => *opaque = true,
                }
            }
        }
        "w:hyperlink" => {
            for child in node.elements() {
                read_inline(child, text, opaque);
            }
        }
        _ => *opaque = true,
    }
}

/// Outer `None` means no write; inner `None` means no previous value.
fn patch_alignment(paragraph: &mut Node, after: Alignment) -> Result<Option<Option<String>>> {
    const ALTERNATE: &str =
        "{http://schemas.openxmlformats.org/markup-compatibility/2006}AlternateContent";
    if paragraph.elements().filter(|n| n.name == "w:pPr").count() > 1
        || paragraph.child(ALTERNATE).is_some()
        || paragraph.child("w:pPr").is_some_and(|props| {
            props.elements().filter(|n| n.name == "w:jc").count() > 1
                || props.child(ALTERNATE).is_some()
        })
    {
        return Err(unsupported("paragraph alignment is structurally ambiguous"));
    }

    if paragraph.child("w:pPr").is_none() {
        paragraph
            .children
            .insert(0, Content::Element(Node::new("w:pPr")));
    }
    let props = paragraph.child_mut("w:pPr").unwrap();
    let jc = props.ensure("w:jc");
    let value = after.xml();
    if jc.get("w:val") == Some(value) {
        return Ok(None);
    }
    let before = jc.get("w:val").map(str::to_owned);
    jc.set("w:val", value);
    Ok(Some(before))
}
