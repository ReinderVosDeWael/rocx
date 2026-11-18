//! Resolve style identities once per compilation; ECMA-376 Part 1 §17.7.
use crate::xml::Node;
use crate::{Diagnostic, Error, Result, Rule, StyleId};
use std::collections::BTreeMap;
pub(super) struct StyleCatalog {
    ids: BTreeMap<String, StyleId>,
    names: BTreeMap<String, StyleId>,
}
impl StyleCatalog {
    pub fn new(styles: &Node) -> Result<Self> {
        let mut catalog = Self {
            ids: BTreeMap::new(),
            names: BTreeMap::new(),
        };
        for style in styles.elements().filter(|n| n.name == "w:style") {
            if let Some(value) = style.get("w:styleId") {
                let id = StyleId::new(value)?;
                catalog.ids.insert(value.into(), id.clone());
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
}
