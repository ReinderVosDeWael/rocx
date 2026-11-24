//! Ownership of direct formatting slots. The authoring encoder supplies canonical
//! wire values; this layer overlays only attributes owned by a selected slot.
use super::unsupported;
use crate::{
    Color, Result, RunFormat, RunLanguage, ThemeColor, Underline,
    xml::{Content, Node},
};
use std::collections::BTreeMap;

macro_rules! fields {
    ($($field:ident, $name:literal, $tag:literal, [$($attr:literal),+];)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub enum RunFormatField { $($field),+ }
        impl RunFormatField {
            const ALL: &'static [Self] = &[$(Self::$field),+];
            pub fn parse(name: &str) -> Option<Self> {
                match name { $($name => Some(Self::$field),)+ _ => None }
            }
            pub(super) fn tag(self) -> &'static str { match self { $(Self::$field => concat!("w:", $tag)),+ } }
            fn attrs(self) -> &'static [&'static str] { match self {
                $(Self::$field => &[$(concat!("w:", $attr)),+]),+
            }}
        }
    }
}
fields! {
    Style, "style", "rStyle", ["val"];
    Font, "font", "rFonts", ["ascii", "hAnsi"];
    FontEastAsia, "fontEastAsia", "rFonts", ["eastAsia"];
    FontComplexScript, "fontComplexScript", "rFonts", ["cs"];
    Bold, "bold", "b", ["val"];
    BoldComplexScript, "boldComplexScript", "bCs", ["val"];
    Italic, "italic", "i", ["val"];
    ItalicComplexScript, "italicComplexScript", "iCs", ["val"];
    AllCaps, "allCaps", "caps", ["val"];
    SmallCaps, "smallCaps", "smallCaps", ["val"];
    Strike, "strike", "strike", ["val"];
    SnapToGrid, "snapToGrid", "snapToGrid", ["val"];
    Hidden, "hidden", "vanish", ["val"];
    Color, "color", "color", ["val","themeColor","themeTint","themeShade"];
    Size, "size", "sz", ["val"];
    SizeComplexScript, "sizeComplexScript", "szCs", ["val"];
    Underline, "underline", "u", ["val"];
    VerticalAlignment, "verticalAlignment", "vertAlign", ["val"];
    Rtl, "rtl", "rtl", ["val"];
    ComplexScript, "complexScript", "cs", ["val"];
    LanguageLatin, "languageLatin", "lang", ["val"];
    LanguageEastAsia, "languageEastAsia", "lang", ["eastAsia"];
    LanguageBidi, "languageBidi", "lang", ["bidi"];
}

impl RunFormatField {
    fn shared(self) -> bool {
        matches!(self.tag(), "w:rFonts" | "w:lang")
    }
    fn present(self, n: &Node) -> bool {
        !self.shared() || self.attrs().iter().any(|a| n.get(a).is_some())
    }
    fn themed(self, n: &Node) -> bool {
        let keys: &[&str] = match self {
            Self::Font => &["w:asciiTheme", "w:hAnsiTheme"],
            Self::FontEastAsia => &["w:eastAsiaTheme"],
            Self::FontComplexScript => &["w:cstheme", "w:csTheme"],
            _ => &[],
        };
        keys.iter().any(|a| n.get(a).is_some())
    }
    /// Decode exactly one slot; unsupported values never invalidate other slots.
    fn decode(self, n: &Node, f: &mut RunFormat, hidden: &mut Option<bool>) -> Option<()> {
        let val = n.get("w:val");
        let toggle = || match val {
            None | Some("on" | "true" | "1") => Some(true),
            Some("off" | "false" | "0") => Some(false),
            _ => None,
        };
        let points = || Some(val?.parse::<u32>().ok()? as f64 / 2.);
        match self {
            Self::Style => f.style = Some(val?.into()),
            Self::Font => {
                let ascii = n.get("w:ascii")?;
                if n.get("w:hAnsi") != Some(ascii) {
                    return None;
                }
                f.font = Some(ascii.into());
            }
            Self::FontEastAsia => f.font_east_asia = Some(n.get("w:eastAsia")?.into()),
            Self::FontComplexScript => f.font_complex_script = Some(n.get("w:cs")?.into()),
            Self::Bold => f.bold = Some(toggle()?),
            Self::Italic => f.italic = Some(toggle()?),
            Self::BoldComplexScript => f.bold_complex_script = Some(toggle()?),
            Self::ItalicComplexScript => f.italic_complex_script = Some(toggle()?),
            Self::AllCaps => f.all_caps = Some(toggle()?),
            Self::SmallCaps => f.small_caps = Some(toggle()?),
            Self::Strike => f.strike = Some(toggle()?),
            Self::SnapToGrid => f.snap_to_grid = Some(toggle()?),
            Self::Hidden => *hidden = Some(toggle()?),
            Self::Rtl => f.rtl = Some(toggle()?),
            Self::ComplexScript => f.complex_script = Some(toggle()?),
            Self::Color => f.color = Some(read_color(n)?),
            Self::Size => f.size = Some(points()?),
            Self::SizeComplexScript => f.size_complex_script = Some(points()?),
            Self::Underline => {
                use Underline::*;
                f.underline = Some(
                    [Single, Words, Double, Dotted, Dash, Wave, Underline::None]
                        .into_iter()
                        .find(|v| Some(v.xml()) == val)?,
                );
            }
            Self::VerticalAlignment => match val? {
                "superscript" => f.superscript = Some(true),
                "subscript" => f.subscript = Some(true),
                "baseline" => {
                    f.superscript = Some(false);
                    f.subscript = Some(false);
                }
                _ => return None,
            },
            Self::LanguageLatin | Self::LanguageEastAsia | Self::LanguageBidi => {
                let value = n.get(self.attrs()[0])?.into();
                let lang = f.language.get_or_insert_with(RunLanguage::default);
                match self {
                    Self::LanguageLatin => lang.latin = Some(value),
                    Self::LanguageEastAsia => lang.east_asia = Some(value),
                    _ => lang.bidi = Some(value),
                }
            }
        }
        Some(())
    }
}

fn read_color(n: &Node) -> Option<Color> {
    let byte = |s: &str| {
        (s.len() == 2 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u8::from_str_radix(s, 16).unwrap())
    };
    let value = match n.get("w:val")? {
        "auto" => Color::Auto,
        rgb if rgb.len() == 6 && rgb.is_ascii() => {
            Color::Rgb([byte(&rgb[..2])?, byte(&rgb[2..4])?, byte(&rgb[4..])?])
        }
        _ => return None,
    };
    let component = |key| match n.get(key) {
        Some(s) => Some(Some(byte(s)?)),
        None => Some(None),
    };
    let tint = component("w:themeTint")?;
    let shade = component("w:themeShade")?;
    if let Some(theme) = n.get("w:themeColor") {
        use ThemeColor::*;
        let name = [
            Dark1,
            Light1,
            Dark2,
            Light2,
            Accent1,
            Accent2,
            Accent3,
            Accent4,
            Accent5,
            Accent6,
            Hyperlink,
            FollowedHyperlink,
            Background1,
            Text1,
            Background2,
            Text2,
        ]
        .into_iter()
        .find(|v| v.xml() == theme)?;
        Some(Color::Theme { name, tint, shade })
    } else {
        (tint.is_none() && shade.is_none()).then_some(value)
    }
}

fn guarded(props: &Node) -> bool {
    props.child(super::ALTERNATE).is_some() || props.child("w:rPrChange").is_some()
}
pub(super) fn inspect(run: &Node) -> (RunFormat, Option<bool>, bool) {
    let mut f = RunFormat::default();
    let mut hidden = None;
    let mut props = run.elements().filter(|n| n.name == "w:rPr");
    let Some(p) = props.next() else {
        return (f, hidden, false);
    };
    if props.next().is_some() || guarded(p) {
        return (f, hidden, true);
    }
    let mut opaque = super::text::has_other_attrs(p, &[]) || super::text::opaque_content(p);
    let mut counts = BTreeMap::new();
    for n in p.elements() {
        *counts.entry(n.name.as_str()).or_insert(0) += 1;
    }
    for n in p.elements() {
        if counts[n.name.as_str()] != 1 {
            opaque = true;
            continue;
        }
        let mut known = false;
        for &field in RunFormatField::ALL
            .iter()
            .filter(|k| k.tag() == n.name.as_str())
        {
            known = true;
            if field.themed(n)
                || (field.present(n) && field.decode(n, &mut f, &mut hidden).is_none())
            {
                opaque = true;
            }
        }
        opaque |= !known
            || !n.children.is_empty()
            || n.attrs.iter().any(|(k, _)| {
                k.declaration().is_none()
                    && !RunFormatField::ALL.iter().any(|field| {
                        field.tag() == n.name.as_str() && field.attrs().contains(&k.as_str())
                    })
            });
    }
    (f, hidden, opaque)
}

// Canonical order makes disjoint patches independent of submission order.
pub(super) type Slots = BTreeMap<RunFormatField, Option<Node>>;
pub(super) fn add(slots: &mut Slots, field: RunFormatField, value: Option<Node>) -> Result<()> {
    if slots.insert(field, value).is_some() {
        return Err(unsupported("duplicate or conflicting run format property"));
    }
    Ok(())
}
pub(super) fn compile(
    slots: &mut Slots,
    f: &RunFormat,
    clear: &[RunFormatField],
    package: &crate::opc::Package,
    main: &str,
) -> Result<()> {
    let mut encoded = crate::ooxml::wordprocessing::format::run(f)?;
    if f.superscript == Some(true) && f.subscript == Some(true) {
        return Err(unsupported("conflicting vertical alignment"));
    }
    if (f.superscript.is_some() || f.subscript.is_some()) && encoded.child("w:vertAlign").is_none()
    {
        encoded.push(crate::xml::val("vertAlign", "baseline"));
    }
    if let Some(name) = &f.style {
        let (_, root) = super::resources::style_part(package, main)?
            .ok_or_else(|| unsupported("missing character style catalog"))?;
        let definitions = || root.elements().filter(|n| n.name == "w:style");
        let mut matching = definitions().filter(|n| {
            n.get("w:styleId") == Some(name)
                || n.elements().any(|v| {
                    v.name == "w:name"
                        && v.get("w:val").is_some_and(|s| s.eq_ignore_ascii_case(name))
                })
        });
        let style = matching
            .next()
            .ok_or_else(|| unsupported("missing character style"))?;
        let id = style
            .get("w:styleId")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| unsupported("missing style ID"))?;
        if matching.next().is_some()
            || style.get("w:type") != Some("character")
            || definitions()
                .filter(|v| v.get("w:styleId") == Some(id))
                .count()
                != 1
        {
            return Err(unsupported("ambiguous or non-character style"));
        }
        encoded.child_mut("w:rStyle").unwrap().set("w:val", id);
    }
    if encoded.elements().next().is_none() && clear.is_empty() {
        return Err(unsupported("empty run format patch"));
    }
    for n in encoded.elements() {
        if n.attrs
            .iter()
            .any(|(_, v)| crate::document::invalid_xml_character(v).is_some())
        {
            return Err(unsupported("invalid XML character in run formatting"));
        }
        for &field in RunFormatField::ALL
            .iter()
            .filter(|k| k.tag() == n.name.as_str() && k.present(n))
        {
            let mut value = Node::new(field.tag());
            if field
                .decode(n, &mut RunFormat::default(), &mut None)
                .is_none()
            {
                return Err(unsupported("run format value exceeds import limits"));
            }
            for attr in field.attrs() {
                if let Some(v) = n.get(attr) {
                    value.set(*attr, v);
                }
            }
            add(slots, field, Some(value))?;
        }
    }
    for field in clear {
        add(slots, *field, None)?;
    }
    Ok(())
}

pub(super) fn apply(run: &mut Node, slots: Slots) -> Result<bool> {
    let mut changed = false;
    for (field, value) in slots {
        let tag = field.tag();
        if let Some(p) = run.child("w:rPr") {
            if guarded(p) || p.elements().filter(|n| n.name == tag).count() > 1 {
                return Err(unsupported("ambiguous run properties"));
            }
            if let Some(n) = p.child(tag) {
                if field.themed(n) {
                    return Err(unsupported("unsupported themed run font"));
                }
                if field.present(n) {
                    let decode = |n: &Node| {
                        let mut f = RunFormat::default();
                        let mut hidden = None;
                        field
                            .decode(n, &mut f, &mut hidden)
                            .ok_or_else(|| unsupported("unsupported run property value"))?;
                        Ok::<_, crate::Error>((f, hidden))
                    };
                    let before = decode(n)?;
                    if let Some(v) = &value {
                        if before == decode(v)? {
                            continue;
                        }
                    } else if !field.shared() {
                        if super::text::has_other_attrs(n, field.attrs()) || !n.children.is_empty()
                        {
                            return Err(unsupported("cannot clear opaque run property"));
                        }
                        run.child_mut("w:rPr").unwrap().remove(tag);
                        changed = true;
                        continue;
                    }
                } else if value.is_none() {
                    continue;
                }
            } else if value.is_none() {
                continue;
            }
        } else if value.is_none() {
            continue;
        }
        if run.child("w:rPr").is_none() {
            run.children.insert(0, Content::Element(Node::new("w:rPr")));
        }
        let p = run.child_mut("w:rPr").unwrap();
        let n = p.ensure(tag);
        n.attrs.retain(|(k, _)| {
            !field.attrs().contains(&k.as_str())
                || value.as_ref().is_some_and(|v| v.get(k.as_str()).is_some())
        });
        if let Some(v) = value {
            for (k, val) in &v.attrs {
                n.set(k.clone(), val);
            }
        } else if !super::text::has_other_attrs(n, &[]) && n.children.is_empty() {
            p.remove(tag);
        }
        changed = true;
    }
    Ok(changed)
}
