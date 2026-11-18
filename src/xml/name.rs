//! Expanded XML names and namespace-safe lexical spelling (XML Namespaces §§2, 6).
use super::NS;
use std::{collections::BTreeMap, fmt, ops::Deref};

#[derive(Clone, Debug)]
pub(crate) struct Name {
    namespace: Option<String>,
    local: String,
    prefix: Option<String>,
    canonical: String,
}
impl Name {
    pub fn expanded(namespace: Option<&str>, local: &str, prefix: Option<&str>) -> Self {
        let namespace = namespace.filter(|uri| !uri.is_empty());
        let canonical = match namespace {
            Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{local}"),
            Some("http://www.w3.org/2000/xmlns/") => {
                if local.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{local}")
                }
            }
            Some(uri) => NS
                .iter()
                .find(|(_, u)| *u == uri)
                .map(|(p, _)| format!("{p}:{local}"))
                .unwrap_or_else(|| format!("{{{uri}}}{local}")),
            None => local.into(),
        };
        Self {
            namespace: namespace.map(str::to_owned),
            local: local.into(),
            prefix: prefix.map(str::to_owned),
            canonical,
        }
    }
    pub fn local(&self) -> &str {
        &self.local
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
    pub fn as_str(&self) -> &str {
        &self.canonical
    }
    pub fn declaration(&self) -> Option<&str> {
        (self.namespace() == Some("http://www.w3.org/2000/xmlns/")).then_some(self.local.as_str())
    }
    pub fn lexical(
        &self,
        scope: &mut BTreeMap<String, String>,
        declarations: &mut Vec<(String, String)>,
        attribute: bool,
    ) -> String {
        let Some(uri) = self.namespace() else {
            return self.local.clone();
        };
        if uri == "http://www.w3.org/XML/1998/namespace" {
            return format!("xml:{}", self.local);
        }
        let preferred = self.prefix.as_deref().unwrap_or("");
        let valid = |prefix: &str| !attribute || !prefix.is_empty();
        let prefix = if valid(preferred) && scope.get(preferred).is_some_and(|u| u == uri) {
            preferred.to_owned()
        } else if let Some((p, _)) = scope.iter().find(|(p, u)| valid(p) && u.as_str() == uri) {
            p.clone()
        } else {
            let mut prefix = preferred.to_owned();
            if !valid(&prefix) || scope.contains_key(&prefix) {
                let mut i = 1;
                while scope.contains_key(&format!("ns{i}")) {
                    i += 1;
                }
                prefix = format!("ns{i}");
            }
            scope.insert(prefix.clone(), uri.into());
            declarations.push((
                if prefix.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{prefix}")
                },
                uri.into(),
            ));
            prefix
        };
        if prefix.is_empty() {
            self.local.clone()
        } else {
            format!("{prefix}:{}", self.local)
        }
    }
}
impl From<&str> for Name {
    fn from(value: &str) -> Self {
        if value == "xmlns" {
            return Self::expanded(Some("http://www.w3.org/2000/xmlns/"), "", Some("xmlns"));
        }
        if let Some(local) = value.strip_prefix("xmlns:") {
            return Self::expanded(Some("http://www.w3.org/2000/xmlns/"), local, Some("xmlns"));
        }
        if let Some(rest) = value.strip_prefix('{')
            && let Some((uri, local)) = rest.split_once('}')
        {
            return Self::expanded(Some(uri), local, None);
        }
        if let Some((prefix, local)) = value.split_once(':') {
            let uri = if prefix == "xml" {
                Some("http://www.w3.org/XML/1998/namespace")
            } else {
                NS.iter().find(|(p, _)| *p == prefix).map(|(_, u)| *u)
            };
            if let Some(uri) = uri {
                return Self::expanded(Some(uri), local, Some(prefix));
            }
        }
        Self::expanded(None, value, None)
    }
}
impl From<String> for Name {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.namespace == other.namespace && self.local == other.local
    }
}
impl Eq for Name {}
impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.canonical == other
    }
}
impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}
impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
