//! Resolve style identities once per compilation; ECMA-376 Part 1 §17.7.
use crate::xml::Node;
use crate::{Diagnostic, Error, Result, Rule, StyleId};
use std::collections::BTreeMap;
pub(super) struct StyleCatalog {
    ids: BTreeMap<String, StyleId>,
    names: BTreeMap<String, StyleId>,
    kinds: BTreeMap<String, String>,
}
impl StyleCatalog {
    pub fn new(styles: &Node) -> Result<Self> {
        let mut catalog = Self {
            ids: BTreeMap::new(),
            names: BTreeMap::new(),
            kinds: BTreeMap::new(),
        };
        for style in styles.elements().filter(|n| n.name == "w:style") {
            if let Some(value) = style.get("w:styleId") {
                let id = StyleId::new(value)?;
                catalog.ids.insert(value.into(), id.clone());
                catalog.kinds.insert(
                    value.into(),
                    style.get("w:type").unwrap_or("paragraph").into(),
                );
                if let Some(name) = style.child("w:name").and_then(|n| n.get("w:val")) {
                    catalog.names.insert(name.to_ascii_lowercase(), id);
                }
            }
        }
        Ok(catalog)
    }
    pub fn get(&self, name: &str) -> Result<StyleId> {
        self.ids
            .get(name)
            .or_else(|| self.names.get(&name.to_ascii_lowercase()))
            .cloned()
            .ok_or_else(|| {
                Error::from(Diagnostic::new(
                    Rule::UnknownStyle,
                    format!("unknown style {name:?}"),
                ))
            })
    }
    pub fn get_typed(&self, name: &str, kind: &str) -> Result<StyleId> {
        let id = self.get(name)?;
        if self.kinds.get(id.as_str()).map(String::as_str) != Some(kind) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                format!("style {name:?} must be a {kind} style"),
            )
            .into());
        }
        Ok(id)
    }
}
