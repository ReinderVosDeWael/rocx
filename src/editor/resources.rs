//! Inspection of relationship-selected document resources.
use super::{ImportedDocument, unsupported};
use crate::Result;
use crate::ooxml::WordParts;
use crate::xml::Node;

/// The source style kind. Unknown values are retained rather than inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleKind {
    Paragraph,
    Character,
    Table,
    Numbering,
    Unsupported(String),
    Missing,
}

impl StyleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Character => "character",
            Self::Table => "table",
            Self::Numbering => "numbering",
            Self::Unsupported(_) => "unsupported",
            Self::Missing => "missing",
        }
    }
}

/// Direct style metadata, not inherited or effective formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedStyle {
    pub id: Option<String>,
    pub kind: StyleKind,
    pub name: Option<String>,
    pub based_on: Option<String>,
    pub next_style: Option<String>,
    pub linked_style: Option<String>,
    /// Standalone XML of direct paragraph/run/table/row/cell property groups
    /// and conditional table regions, in physical source order.
    pub property_xml: Vec<String>,
}

/// The physical styles part and its styles in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedStyles {
    pub part_name: String,
    pub styles: Vec<ImportedStyle>,
}

impl ImportedDocument {
    /// Inspect the styles relationship. `None` means no styles relationship;
    /// malformed or unsupported parts are errors, not an empty styles list.
    pub fn styles(&self) -> Result<Option<ImportedStyles>> {
        let Some((path, root)) = self.style_part()? else {
            return Ok(None);
        };
        Ok(Some(ImportedStyles {
            part_name: path,
            styles: inspect(&root),
        }))
    }

    pub(super) fn style_part(&self) -> Result<Option<(String, Node)>> {
        style_part(&self.package, &self.main)
    }
}

pub(super) fn style_part(
    package: &crate::opc::Package,
    main: &str,
) -> Result<Option<(String, Node)>> {
    let links = package.relationships(main, "styles")?;
    if links.is_empty() {
        return Ok(None);
    }
    if links.len() != 1 {
        return Err(unsupported("ambiguous styles part"));
    }
    let (_, path) = package
        .related(main, "styles")?
        .ok_or_else(|| unsupported("missing styles part"))?;
    if !package.content_type_for(&path)?.is_some_and(|ty| {
        ty.eq_ignore_ascii_case(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
        )
    }) {
        return Err(unsupported("invalid styles content type"));
    }
    let root = package.word(&path)?;
    if root.name != "w:styles" {
        return Err(unsupported("invalid styles root"));
    }
    Ok(Some((path, root)))
}

fn inspect(root: &Node) -> Vec<ImportedStyle> {
    root.elements()
        .filter(|n| n.name == "w:style")
        .map(|node| {
            let kind = match node.get("w:type") {
                None => StyleKind::Missing,
                Some("paragraph") => StyleKind::Paragraph,
                Some("character") => StyleKind::Character,
                Some("table") => StyleKind::Table,
                Some("numbering") => StyleKind::Numbering,
                Some(other) => StyleKind::Unsupported(other.to_owned()),
            };
            ImportedStyle {
                id: node.get("w:styleId").map(str::to_owned),
                kind,
                name: node
                    .child("w:name")
                    .and_then(|n| n.get("w:val"))
                    .map(str::to_owned),
                based_on: node
                    .child("w:basedOn")
                    .and_then(|n| n.get("w:val"))
                    .map(str::to_owned),
                next_style: node
                    .child("w:next")
                    .and_then(|n| n.get("w:val"))
                    .map(str::to_owned),
                linked_style: node
                    .child("w:link")
                    .and_then(|n| n.get("w:val"))
                    .map(str::to_owned),
                property_xml: node
                    .elements()
                    .filter(|n| {
                        matches!(
                            n.name.as_str(),
                            "w:rPr" | "w:pPr" | "w:tblPr" | "w:trPr" | "w:tcPr" | "w:tblStylePr"
                        )
                    })
                    .map(Node::xml)
                    .collect(),
            }
        })
        .collect()
}
