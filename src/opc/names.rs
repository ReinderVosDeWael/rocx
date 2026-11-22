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
        if target.starts_with("//") {
            return Err(Error::Validation(
                "internal relationship cannot have a URI authority".into(),
            ));
        }
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
        if !crate::xml::is_ncname(value) {
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

pub(super) fn relationship_type(kind: &str) -> String {
    if kind.contains(':') {
        kind.into()
    } else if kind == "core-properties" {
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties"
            .into()
    } else {
        format!("http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}")
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

    #[test]
    fn part_names_normalize_leading_slash_and_preserve_zip_identity() {
        for name in [
            "document.xml",
            "word/media/image-1.png",
            "a/b/c/item%20name.xml",
            "a/%C3%A9.xml",
        ] {
            let relative = PartName::new(name).unwrap();
            let absolute = PartName::new(&format!("/{name}")).unwrap();
            assert_eq!(relative, absolute, "{name}");
            assert_eq!(relative.as_str(), format!("/{name}"));
            assert_eq!(relative.zip_name(), name);
        }
    }

    #[test]
    fn relative_targets_round_trip_across_owner_and_target_matrix() {
        let owners = [
            "",
            "document.xml",
            "/word/document.xml",
            "word/stories/header.xml",
            "word/stories/nested/footer.xml",
        ];
        let targets = [
            "document.xml",
            "word/document.xml",
            "word/stories/header.xml",
            "word/stories/nested/item%20one.xml",
            "media/image.png",
        ];
        for owner in owners {
            for name in targets {
                let target = PartName::new(name).unwrap();
                let relative = target.relative_to(owner).unwrap();
                assert_eq!(
                    PartName::resolve(owner, &relative).unwrap(),
                    target,
                    "owner={owner:?}, target={name:?}, relative={relative:?}"
                );
                assert_eq!(
                    PartName::resolve(owner, target.as_str()).unwrap(),
                    target,
                    "absolute target for owner={owner:?}, target={name:?}"
                );
            }
        }
    }

    #[test]
    fn resolution_handles_dot_segments_and_rejects_root_escape() {
        for (owner, target, expected) in [
            ("", "./word/document.xml", "/word/document.xml"),
            (
                "word/document.xml",
                "./media/../styles.xml",
                "/word/styles.xml",
            ),
            (
                "word/stories/header.xml",
                "../../media/image.png",
                "/media/image.png",
            ),
            (
                "word/stories/header.xml",
                "/word/./media/../styles.xml",
                "/word/styles.xml",
            ),
        ] {
            assert_eq!(
                PartName::resolve(owner, target).unwrap().as_str(),
                expected,
                "owner={owner:?}, target={target:?}"
            );
        }
        for (owner, target) in [
            ("", "../x.xml"),
            ("main.xml", "../x.xml"),
            ("word/main.xml", "../../x.xml"),
            ("word/stories/header.xml", "../../../x.xml"),
            ("word/stories/header.xml", "/../x.xml"),
        ] {
            assert!(
                PartName::resolve(owner, target).is_err(),
                "owner={owner:?}, target={target:?}"
            );
        }
    }

    #[test]
    fn invalid_names_and_targets_are_rejected() {
        for invalid in [
            "",
            "/",
            "//word/x.xml",
            "word//x.xml",
            "word/./x.xml",
            "word/../x.xml",
            "word/x.",
            "word/x y.xml",
            "word/x\n.xml",
            "word/x\\y.xml",
            "word/x?y.xml",
            "word/x#y.xml",
            "word/x:y.xml",
            "word/%.xml",
            "word/%2.xml",
            "word/%GG.xml",
            "word/%2f.xml",
            "word/%5C.xml",
            "word/%41.xml",
            "word/%2e.xml",
        ] {
            assert!(PartName::new(invalid).is_err(), "name={invalid:?}");
        }
        for target in [
            "",
            "//host/x.xml",
            "x//y.xml",
            "x y.xml",
            "x?query",
            "x#fragment",
            "http://host/x.xml",
            "%2F.xml",
            "%41.xml",
            "%GG.xml",
        ] {
            assert!(
                PartName::resolve("word/document.xml", target).is_err(),
                "target={target:?}"
            );
        }
        assert!(
            PartName::new("word/x.xml")
                .unwrap()
                .relative_to("word//document.xml")
                .is_err()
        );
    }
}
