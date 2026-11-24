//! Attribute edits operate on physical OOXML values rather than evaluated formatting.
use super::{ALTERNATE, Node, Result, unique_child, unsupported};
use crate::xml::Content;

pub(super) struct Attribute {
    group: &'static str,
    tag: String,
    key: String,
    required_val: Option<&'static str>,
    tab: Option<i32>,
}

fn hex(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|c| c.is_ascii_hexdigit())
}
fn theme(s: &str) -> bool {
    matches!(
        s,
        "dark1"
            | "light1"
            | "dark2"
            | "light2"
            | "accent1"
            | "accent2"
            | "accent3"
            | "accent4"
            | "accent5"
            | "accent6"
            | "hyperlink"
            | "followedHyperlink"
            | "none"
            | "background1"
            | "background2"
            | "text1"
            | "text2"
    )
}
fn on_off(s: &str) -> bool {
    matches!(s, "true" | "false" | "0" | "1" | "on" | "off")
}

pub(super) fn select(
    field: &str,
    kind: Option<&str>,
    value: Option<&str>,
) -> Result<Option<Attribute>> {
    if !field.contains('.') {
        return Ok(None);
    }
    let (tag, attr) = field.split_once('.').unwrap();
    let mut key = attr;
    let mut tab = None;
    let mut required_val = None;
    let (group, valid) = match (tag, attr) {
        ("rFonts", "ascii" | "hAnsi" | "eastAsia" | "cs") => {
            ("w:rPr", value.is_none_or(|s| !s.is_empty()))
        }
        ("rFonts", "asciiTheme" | "hAnsiTheme" | "eastAsiaTheme" | "cstheme") => (
            "w:rPr",
            value.is_none_or(|s| {
                matches!(
                    s,
                    "majorAscii"
                        | "majorHAnsi"
                        | "majorEastAsia"
                        | "majorBidi"
                        | "minorAscii"
                        | "minorHAnsi"
                        | "minorEastAsia"
                        | "minorBidi"
                )
            }),
        ),
        ("lang", "val" | "eastAsia" | "bidi") => {
            ("w:rPr", value.is_none_or(|s| !s.trim().is_empty()))
        }
        ("sz" | "szCs", "val") => {
            required_val = Some("");
            (
                "w:rPr",
                value.is_none_or(|s| s.parse::<u32>().is_ok_and(|n| n > 0)),
            )
        }
        ("vertAlign", "val") => {
            required_val = Some("");
            (
                "w:rPr",
                value.is_none_or(|s| matches!(s, "subscript" | "superscript" | "baseline")),
            )
        }
        ("runSnapToGrid", "val") => {
            required_val = Some("");
            ("w:rPr", value.is_none_or(on_off))
        }
        ("u", "val") => (
            "w:rPr",
            value.is_none_or(|s| {
                matches!(
                    s,
                    "none"
                        | "single"
                        | "double"
                        | "words"
                        | "dotted"
                        | "dash"
                        | "wave"
                        | "thick"
                        | "dottedHeavy"
                        | "dashedHeavy"
                        | "dashLong"
                        | "dashLongHeavy"
                        | "dotDash"
                        | "dashDotHeavy"
                        | "dotDotDash"
                        | "dashDotDotHeavy"
                        | "wavyHeavy"
                        | "wavyDouble"
                )
            }),
        ),
        ("shd", "val") => {
            required_val = Some("clear");
            (
                "w:tcPr",
                value.is_none_or(|s| {
                    matches!(
                        s,
                        "nil"
                            | "clear"
                            | "solid"
                            | "horzStripe"
                            | "vertStripe"
                            | "reverseDiagStripe"
                            | "diagStripe"
                            | "horzCross"
                            | "diagCross"
                            | "thinHorzStripe"
                            | "thinVertStripe"
                            | "thinReverseDiagStripe"
                            | "thinDiagStripe"
                            | "thinHorzCross"
                            | "thinDiagCross"
                            | "pct5"
                            | "pct10"
                            | "pct12"
                            | "pct15"
                            | "pct20"
                            | "pct25"
                            | "pct30"
                            | "pct35"
                            | "pct37"
                            | "pct40"
                            | "pct45"
                            | "pct50"
                            | "pct55"
                            | "pct60"
                            | "pct62"
                            | "pct65"
                            | "pct70"
                            | "pct75"
                            | "pct80"
                            | "pct85"
                            | "pct87"
                            | "pct90"
                            | "pct95"
                    )
                }),
            )
        }
        ("color", "val" | "themeColor" | "themeTint" | "themeShade")
        | ("u", "color" | "themeColor" | "themeTint" | "themeShade")
        | (
            "shd",
            "fill" | "color" | "themeColor" | "themeFill" | "themeTint" | "themeShade"
            | "themeFillTint" | "themeFillShade",
        ) => {
            required_val = match tag {
                "shd" => Some("clear"),
                "color" => Some("auto"),
                _ => None,
            };
            (
                if tag == "shd" { "w:tcPr" } else { "w:rPr" },
                value.is_none_or(|s| match attr {
                    "val" | "fill" | "color" => s == "auto" || hex(s, 6),
                    "themeColor" | "themeFill" => theme(s),
                    _ => hex(s, 2),
                }),
            )
        }
        ("spacing", "lineRule") => (
            "w:pPr",
            value.is_none_or(|s| matches!(s, "auto" | "exact" | "atLeast")),
        ),
        ("spacing", "beforeAutospacing" | "afterAutospacing") => {
            ("w:pPr", value.is_none_or(on_off))
        }
        ("spacing", "before" | "after" | "beforeLines" | "afterLines")
        | ("ind", "firstLine" | "hanging") => {
            ("w:pPr", value.is_none_or(|s| s.parse::<u32>().is_ok()))
        }
        ("spacing", "line")
        | (
            "ind",
            "left" | "right" | "start" | "end" | "leftChars" | "rightChars" | "startChars"
            | "endChars" | "firstLineChars" | "hangingChars",
        ) => ("w:pPr", value.is_none_or(|s| s.parse::<i32>().is_ok())),
        ("tab", selector) => {
            let (pos, k) = selector
                .split_once('.')
                .ok_or_else(|| unsupported("expected tab.<twips>.val or .leader"))?;
            let p: i32 = pos
                .parse()
                .map_err(|_| unsupported("invalid tab position"))?;
            if pos != p.to_string() {
                return Err(unsupported("noncanonical tab position"));
            }
            tab = Some(p);
            key = k;
            required_val = Some("left");
            (
                "w:pPr",
                match k {
                    "val" => value.is_none_or(|s| {
                        matches!(
                            s,
                            "left"
                                | "right"
                                | "center"
                                | "decimal"
                                | "bar"
                                | "clear"
                                | "start"
                                | "end"
                                | "num"
                        )
                    }),
                    "leader" => value.is_none_or(|s| {
                        matches!(
                            s,
                            "none" | "dot" | "hyphen" | "underscore" | "heavy" | "middleDot"
                        )
                    }),
                    _ => false,
                },
            )
        }
        _ => return Err(unsupported("unsupported style attribute")),
    };
    if !valid
        || !matches!(kind, Some("paragraph" | "character" | "table"))
        || group == "w:tcPr" && kind != Some("table")
        || group == "w:pPr" && kind == Some("character")
    {
        return Err(unsupported("invalid style attribute value or kind"));
    }
    if let Some(s) = value {
        crate::xml::validate_text(s)?;
    }
    Ok(Some(Attribute {
        group,
        tag: format!(
            "w:{}",
            if tag == "runSnapToGrid" {
                "snapToGrid"
            } else {
                tag
            }
        ),
        key: format!("w:{key}"),
        required_val,
        tab,
    }))
}

impl Attribute {
    pub(super) fn apply(self, container: &mut Node, value: Option<&str>) -> Result<bool> {
        if unique_child(container, self.group)?.is_none() && value.is_none() {
            return Ok(false);
        }
        let group = container.ensure(self.group);
        if group.child(ALTERNATE).is_some() {
            return Err(unsupported("ambiguous style property group"));
        }
        if let Some(pos) = self.tab {
            if unique_child(group, "w:tabs")?.is_none() && value.is_none() {
                return Ok(false);
            }
            let tabs = group.ensure("w:tabs");
            let mut slot = None;
            for (i, c) in tabs.children.iter().enumerate() {
                if let Content::Element(n) = c {
                    if n.name == ALTERNATE {
                        return Err(unsupported("ambiguous tab stops"));
                    }
                    if n.name == "w:tab"
                        && n.get("w:pos")
                            .and_then(|p| p.parse::<i32>().ok())
                            .ok_or_else(|| unsupported("invalid tab position"))?
                            == pos
                        && slot.replace(i).is_some()
                    {
                        return Err(unsupported("duplicate tab position"));
                    }
                }
            }
            if slot.is_none() && value.is_none() {
                return Ok(false);
            }
            let i = slot.unwrap_or_else(|| {
                tabs.children.push(Content::Element(
                    Node::new("w:tab").attr("w:pos", pos).attr("w:val", "left"),
                ));
                tabs.children.len() - 1
            });
            let Content::Element(n) = &mut tabs.children[i] else {
                unreachable!()
            };
            if value.is_none() && self.key == "w:val" {
                if !n.children.is_empty()
                    || n.attrs.iter().any(|(k, _)| {
                        k.declaration().is_none()
                            && !matches!(k.as_str(), "w:pos" | "w:val" | "w:leader")
                    })
                {
                    return Err(unsupported("cannot discard annotated tab"));
                }
                tabs.children.remove(i);
                if !tabs.elements().any(|n| n.name == "w:tab") {
                    if tabs.attrs.iter().any(|(k, _)| k.declaration().is_none())
                        || tabs
                            .children
                            .iter()
                            .any(|c| !matches!(c,Content::Text(s) if s.trim().is_empty()))
                    {
                        return Err(unsupported("cannot discard annotated tabs container"));
                    }
                    group.remove("w:tabs");
                }
                return Ok(true);
            }
            return Ok(set(n, &self.key, value) || slot.is_none());
        }
        let before = unique_child(group, &self.tag)?;
        let Some(s) = value else {
            let Some(n) = before else { return Ok(false) };
            if n.get(&self.key).is_none() {
                return Ok(false);
            }
            if self.key == "w:val" && self.required_val.is_some() {
                if !n.children.is_empty()
                    || n.attrs
                        .iter()
                        .any(|(k, _)| k.declaration().is_none() && k != self.key.as_str())
                {
                    return Err(unsupported(
                        "cannot clear required value and retain sibling attributes",
                    ));
                }
                group.remove(&self.tag);
            } else {
                let n = group.child_mut(&self.tag).unwrap();
                set(n, &self.key, None);
                if n.children.is_empty() && n.attrs.iter().all(|(k, _)| k.declaration().is_some()) {
                    group.remove(&self.tag);
                }
            }
            return Ok(true);
        };
        let n = group.ensure(&self.tag);
        if self.key != "w:val"
            && n.get("w:val").is_none()
            && let Some(default) = self.required_val
        {
            n.set("w:val", default);
        }
        Ok(set(n, &self.key, Some(s)))
    }
}

fn set(n: &mut Node, key: &str, value: Option<&str>) -> bool {
    if n.get(key) == value {
        return false;
    }
    if let Some(s) = value {
        n.set(key, s);
    } else {
        n.attrs.retain(|(k, _)| k != key);
    }
    true
}
