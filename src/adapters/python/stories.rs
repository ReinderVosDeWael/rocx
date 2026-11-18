//! Native conversion into resolved domain values.
use super::*;
pub(super) fn section(o: &Bound<'_, PyAny>) -> PyResult<ResolvedSection> {
    let mut section = ResolvedSection {
        blocks: blocks(&o.getattr("children")?)?,
        properties: optional_record(o, "properties", sectionproperties)?,
        headers: Default::default(),
        footers: Default::default(),
    };
    for name in ["headers", "footers"] {
        let value = o.getattr(name)?;
        if value.is_none() {
            continue;
        }
        for (k, v) in items(&value)? {
            let key = headerfootertype(&k)?;
            let content = ResolvedHeaderFooter {
                blocks: blocks(&v.getattr("children")?)?,
            };
            if name == "headers" {
                section.headers.insert(key, content);
            } else {
                section.footers.insert(key, content);
            }
        }
    }
    Ok(section)
}

use crate::{ResolvedHeaderFooter, ResolvedSection};
