//! OPC identities and references: ECMA-376 Part 2 (2021), §§6.2, 6.4–6.5.
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartName(String);
impl PartName {
    pub fn new(value: &str) -> Result<Self> {
        let path = value.strip_prefix('/').unwrap_or(value);
        if path.is_empty()
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with('.'))
            || path
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "\\?#:".contains(c))
        {
            return Err(Error::Validation(format!(
                "invalid OPC part name {value:?}"
            )));
        }
        let bytes = path.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                let encoded = path
                    .get(i + 1..i + 3)
                    .and_then(|s| u8::from_str_radix(s, 16).ok())
                    .ok_or_else(|| {
                        Error::Validation("invalid percent escape in part name".into())
                    })?;
                if encoded.is_ascii_alphanumeric() || b"-._~/\\".contains(&encoded) {
                    return Err(Error::Validation(
                        "encoded separator/unreserved character in part name".into(),
                    ));
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        Ok(Self(format!("/{path}")))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn zip_name(&self) -> &str {
        &self.0[1..]
    }
    pub fn resolve(owner: &str, target: &str) -> Result<Self> {
        if target.is_empty() {
            return Err(Error::Validation("empty relationship target".into()));
        }
        let mut segments: Vec<&str> = if target.starts_with('/') || owner.is_empty() {
            Vec::new()
        } else {
            owner
                .trim_start_matches('/')
                .rsplit_once('/')
                .map(|(dir, _)| dir.split('/').collect())
                .unwrap_or_default()
        };
        for segment in target.split('/') {
            match segment {
                "" if segments.is_empty() => (),
                "." => (),
                ".." => {
                    segments.pop().ok_or_else(|| {
                        Error::Validation("relationship target escapes package root".into())
                    })?;
                }
                other => segments.push(other),
            }
        }
        Self::new(&segments.join("/"))
    }
    pub fn relative_to(&self, owner: &str) -> Result<String> {
        if owner.is_empty() {
            return Ok(self.zip_name().into());
        }
        let owner = Self::new(owner)?;
        let directories: Vec<_> = owner.zip_name().split('/').collect();
        let directories = &directories[..directories.len() - 1];
        let target: Vec<_> = self.zip_name().split('/').collect();
        let shared = directories
            .iter()
            .zip(&target)
            .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
            .count();
        let mut parts = vec![".."; directories.len() - shared];
        parts.extend_from_slice(&target[shared..]);
        Ok(parts.join("/"))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipId(String);
impl RelationshipId {
    pub fn new(value: &str) -> Result<Self> {
        let mut chars = value.chars();
        if !chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
            || !chars
                .all(|c| c == '_' || c == '-' || c == '.' || c.is_alphanumeric() || c == '\u{b7}')
        {
            return Err(Error::Validation(format!(
                "invalid relationship ID {value:?}"
            )));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipType(String);
impl RelationshipType {
    pub fn new(value: &str) -> Result<Self> {
        if !value.contains(':') || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(Error::Validation(
                "relationship type must be an absolute URI".into(),
            ));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentType(String);
impl ContentType {
    pub fn new(value: &str) -> Result<Self> {
        if !value
            .split_once('/')
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(Error::Validation("invalid OPC content type".into()));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Internal(PartName),
    External(String),
}

pub(super) fn relationships_path(owner: &str) -> String {
    if owner.is_empty() {
        return "_rels/.rels".into();
    }
    let owner = owner.trim_start_matches('/');
    match owner.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{owner}.rels"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_are_relative_to_actual_owner_and_cannot_escape_root() {
        let target = PartName::new("media/image.png").unwrap();
        assert_eq!(
            target.relative_to("content/stories/header.xml").unwrap(),
            "../../media/image.png"
        );
        assert_eq!(
            PartName::resolve("content/stories/header.xml", "../../media/image.png").unwrap(),
            target
        );
        assert!(PartName::resolve("main.xml", "../outside.xml").is_err());
        assert_eq!(relationships_path("main.xml"), "_rels/main.xml.rels");
        for invalid in [
            "/",
            "word//x.xml",
            "../x",
            "word/%2F.xml",
            "word/a#b",
            "word/invalid.",
        ] {
            assert!(PartName::new(invalid).is_err(), "{invalid}");
        }
    }
}
