//! Physical defaults in styles/settings. Shared XML elements (e.g. spacing)
//! are edited one attribute at a time; no effective formatting is computed.
use super::{ImportedDocument, Node, Package, Part, ResourceEdit, Result, unique, unsupported};
use crate::ooxml::WordParts;
use crate::xml::Content;
use std::collections::{BTreeMap, BTreeSet};

/// One physical default tab stop; the leader is absent unless explicitly set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultTab {
    pub position_twips: i32,
    pub alignment: String,
    pub leader: Option<String>,
}

/// Direct property value; absence is represented by `None` on an edit or a
/// missing map entry on inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefaultValue {
    Bool(bool),
    Integer(i64),
    Text(String),
    Tabs(Vec<DefaultTab>),
}

#[derive(Clone, Copy)]
enum Kind {
    Toggle,
    Integer(i64, i64),
    Text,
    Choice(&'static str),
    Hex(usize),
    Color,
    Tabs,
}
use Kind::*;
const U: Kind = Integer(0, u32::MAX as i64);
const I: Kind = Integer(i32::MIN as i64, i32::MAX as i64);
const THEME: Kind = Choice(
    "dark1 light1 dark2 light2 accent1 accent2 accent3 accent4 accent5 accent6 hyperlink followedHyperlink background1 text1 background2 text2 none",
);
const FONT_THEME: Kind = Choice(
    "majorEastAsia majorBidi majorAscii majorHAnsi minorEastAsia minorBidi minorAscii minorHAnsi",
);
const FORMAT: Kind = Choice(
    "decimal upperRoman lowerRoman upperLetter lowerLetter ordinal cardinalText ordinalText hex chicago ideographDigital japaneseCounting aiueo iroha decimalFullWidth decimalHalfWidth japaneseLegal japaneseDigitalTenThousand decimalEnclosedCircle decimalFullWidth2 aiueoFullWidth irohaFullWidth decimalZero bullet ganada chosung decimalEnclosedFullstop decimalEnclosedParen decimalEnclosedCircleChinese ideographEnclosedCircle ideographTraditional ideographZodiac ideographZodiacTraditional taiwaneseCounting ideographLegalTraditional taiwaneseCountingThousand taiwaneseDigital chineseCounting chineseLegalSimplified chineseCountingThousand koreanDigital koreanCounting koreanLegal koreanDigital2 vietnameseCounting russianLower russianUpper none numberInDash hebrew1 hebrew2 arabicAlpha arabicAbjad hindiVowels hindiConsonants hindiNumbers hindiCounting thaiLetters thaiNumbers thaiCounting bahtText dollarText custom",
);
const RESTART: Kind = Choice("continuous eachSect eachPage");
const RUN: &[&str] = &["w:docDefaults", "w:rPrDefault", "w:rPr"];
const PARA: &[&str] = &["w:docDefaults", "w:pPrDefault", "w:pPr"];
const FOOT: &[&str] = &["w:footnotePr"];
const END: &[&str] = &["w:endnotePr"];
const SETTINGS: &[&str] = &[];
type Spec = (&'static [&'static str], &'static str, &'static str, Kind);
macro_rules! fields {
    ($($field:ident: $key:literal, $path:ident, $leaf:literal, $attr:literal, $kind:expr;)*) => {
        /// A modeled direct property of the document's styles or settings part.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum DefaultField { $($field,)* }
        impl DefaultField {
            pub const ALL: &'static [Self] = &[$(Self::$field,)*];
            pub fn key(self) -> &'static str { match self { $(Self::$field => $key,)* } }
            pub fn from_key(key: &str) -> Option<Self> { match key { $($key => Some(Self::$field),)* _ => None } }
            fn spec(self) -> Spec { match self { $(Self::$field => ($path,concat!("w:",$leaf),concat!("w:",$attr),$kind),)* } }
        }
    }
}
fields! {
    Bold: "bold", RUN, "b", "val", Toggle;
    Italic: "italic", RUN, "i", "val", Toggle;
    BoldCs: "boldCs", RUN, "bCs", "val", Toggle;
    ItalicCs: "italicCs", RUN, "iCs", "val", Toggle;
    RunRtl: "runRtl", RUN, "rtl", "val", Toggle;
    ComplexScript: "complexScript", RUN, "cs", "val", Toggle;
    Caps: "caps", RUN, "caps", "val", Toggle;
    SmallCaps: "smallCaps", RUN, "smallCaps", "val", Toggle;
    Strike: "strike", RUN, "strike", "val", Toggle;
    RunSnapToGrid: "runSnapToGrid", RUN, "snapToGrid", "val", Toggle;
    FontAscii: "fontAscii", RUN, "rFonts", "ascii", Text;
    FontHAnsi: "fontHAnsi", RUN, "rFonts", "hAnsi", Text;
    FontEastAsia: "fontEastAsia", RUN, "rFonts", "eastAsia", Text;
    FontCs: "fontCs", RUN, "rFonts", "cs", Text;
    FontAsciiTheme: "fontAsciiTheme", RUN, "rFonts", "asciiTheme", FONT_THEME;
    FontHAnsiTheme: "fontHAnsiTheme", RUN, "rFonts", "hAnsiTheme", FONT_THEME;
    FontEastAsiaTheme: "fontEastAsiaTheme", RUN, "rFonts", "eastAsiaTheme", FONT_THEME;
    FontCsTheme: "fontCsTheme", RUN, "rFonts", "cstheme", FONT_THEME;
    SizeHalfPoints: "sizeHalfPoints", RUN, "sz", "val", U;
    SizeCsHalfPoints: "sizeCsHalfPoints", RUN, "szCs", "val", U;
    Language: "language", RUN, "lang", "val", Text;
    LanguageEastAsia: "languageEastAsia", RUN, "lang", "eastAsia", Text;
    LanguageBidi: "languageBidi", RUN, "lang", "bidi", Text;
    Underline: "underline", RUN, "u", "val", Choice("none single double dotted dash wave words thick dotDash dotDotDash dottedHeavy dashedHeavy dashLong dashLongHeavy dashDotHeavy dashDotDotHeavy wavyHeavy wavyDouble");
    VerticalAlign: "verticalAlign", RUN, "vertAlign", "val", Choice("baseline superscript subscript");
    RunColor: "runColor", RUN, "color", "val", Color;
    ColorTheme: "colorTheme", RUN, "color", "themeColor", THEME;
    ColorTint: "colorTint", RUN, "color", "themeTint", Hex(2);
    ColorShade: "colorShade", RUN, "color", "themeShade", Hex(2);
    ParagraphBidi: "paragraphBidi", PARA, "bidi", "val", Toggle;
    KeepTogether: "keepTogether", PARA, "keepLines", "val", Toggle;
    KeepWithNext: "keepWithNext", PARA, "keepNext", "val", Toggle;
    PageBreakBefore: "pageBreakBefore", PARA, "pageBreakBefore", "val", Toggle;
    WidowControl: "widowControl", PARA, "widowControl", "val", Toggle;
    SuppressLineNumbers: "suppressLineNumbers", PARA, "suppressLineNumbers", "val", Toggle;
    ContextualSpacing: "contextualSpacing", PARA, "contextualSpacing", "val", Toggle;
    MirrorIndents: "mirrorIndents", PARA, "mirrorIndents", "val", Toggle;
    ParagraphSnapToGrid: "paragraphSnapToGrid", PARA, "snapToGrid", "val", Toggle;
    BeforeTwips: "beforeTwips", PARA, "spacing", "before", U;
    BeforeLines: "beforeLines", PARA, "spacing", "beforeLines", I;
    AfterLines: "afterLines", PARA, "spacing", "afterLines", I;
    BeforeAutoSpacing: "beforeAutoSpacing", PARA, "spacing", "beforeAutospacing", Toggle;
    AfterAutoSpacing: "afterAutoSpacing", PARA, "spacing", "afterAutospacing", Toggle;
    AfterTwips: "afterTwips", PARA, "spacing", "after", U;
    Line: "line", PARA, "spacing", "line", I;
    LineRule: "lineRule", PARA, "spacing", "lineRule", Choice("auto exact atLeast");
    LeftTwips: "leftTwips", PARA, "ind", "left", I;
    StartTwips: "startTwips", PARA, "ind", "start", I;
    EndTwips: "endTwips", PARA, "ind", "end", I;
    StartChars: "startChars", PARA, "ind", "startChars", I;
    EndChars: "endChars", PARA, "ind", "endChars", I;
    Justification: "justification", PARA, "jc", "val", Choice("start center end both mediumKashida distribute numTab highKashida lowKashida thaiDistribute left right");
    RightTwips: "rightTwips", PARA, "ind", "right", I;
    FirstLineTwips: "firstLineTwips", PARA, "ind", "firstLine", U;
    HangingTwips: "hangingTwips", PARA, "ind", "hanging", U;
    LeftChars: "leftChars", PARA, "ind", "leftChars", I;
    RightChars: "rightChars", PARA, "ind", "rightChars", I;
    FirstLineChars: "firstLineChars", PARA, "ind", "firstLineChars", I;
    HangingChars: "hangingChars", PARA, "ind", "hangingChars", I;
    TabStops: "tabStops", PARA, "tabs", "", Tabs;
    MirrorMargins: "mirrorMargins", SETTINGS, "mirrorMargins", "val", Toggle;
    GutterAtTop: "gutterAtTop", SETTINGS, "gutterAtTop", "val", Toggle;
    TrackRevisions: "trackRevisions", SETTINGS, "trackRevisions", "val", Toggle;
    FootnotePosition: "footnotePosition", FOOT, "pos", "val", Choice("pageBottom beneathText sectEnd docEnd");
    EndnotePosition: "endnotePosition", END, "pos", "val", Choice("sectEnd docEnd");
    FootnoteFormat: "footnoteFormat", FOOT, "numFmt", "val", FORMAT;
    EndnoteFormat: "endnoteFormat", END, "numFmt", "val", FORMAT;
    FootnoteStart: "footnoteStart", FOOT, "numStart", "val", U;
    EndnoteStart: "endnoteStart", END, "numStart", "val", U;
    FootnoteRestart: "footnoteRestart", FOOT, "numRestart", "val", RESTART;
    EndnoteRestart: "endnoteRestart", END, "numRestart", "val", RESTART;
}

fn bad() -> crate::Error {
    unsupported("invalid or unsupported document default")
}
fn xml_integer(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\r', '\n'])
}
fn parse(kind: Kind, text: &str) -> Result<DefaultValue> {
    Ok(match kind {
        Toggle => DefaultValue::Bool(match text {
            "1" | "true" | "on" => true,
            "0" | "false" | "off" => false,
            _ => return Err(bad()),
        }),
        Integer(lo, hi) => DefaultValue::Integer(
            xml_integer(text)
                .parse()
                .ok()
                .filter(|n| (lo..=hi).contains(n))
                .ok_or_else(bad)?,
        ),
        Choice(values) if !values.split_whitespace().any(|v| v == text) => return Err(bad()),
        Hex(n) if !(text.len() == n && text.bytes().all(|b| b.is_ascii_hexdigit())) => {
            return Err(bad());
        }
        Color if text != "auto" => return parse(Hex(6), text),
        Tabs => return Err(bad()),
        _ => {
            crate::xml::validate_text(text)?;
            DefaultValue::Text(text.into())
        }
    })
}
fn settings(f: DefaultField) -> bool {
    !matches!(f.spec().0.first(), Some(&"w:docDefaults"))
}
fn validate_tabs(tabs: &[DefaultTab]) -> Result<()> {
    let mut positions = BTreeSet::new();
    if tabs.is_empty()
        || tabs.iter().any(|t| {
            !positions.insert(t.position_twips)
                || !matches!(
                    t.alignment.as_str(),
                    "left"
                        | "center"
                        | "right"
                        | "decimal"
                        | "bar"
                        | "clear"
                        | "start"
                        | "end"
                        | "num"
                )
                || !matches!(
                    t.leader.as_deref(),
                    None | Some("none" | "dot" | "hyphen" | "underscore" | "heavy" | "middleDot")
                )
        })
    {
        return Err(bad());
    }
    Ok(())
}
fn read(root: &Node, f: DefaultField) -> Result<Option<DefaultValue>> {
    let (path, leaf, attr, kind) = f.spec();
    let mut parent = root;
    for &name in path.iter().chain([&leaf]) {
        let Some(n) = unique(parent, name)? else {
            return Ok(None);
        };
        parent = n;
    }
    if matches!(kind, Tabs) {
        let mut tabs = Vec::new();
        for n in parent.elements() {
            if n.name != "w:tab" {
                return Err(bad());
            }
            tabs.push(DefaultTab {
                position_twips: n
                    .get("w:pos")
                    .and_then(|s| xml_integer(s).parse().ok())
                    .ok_or_else(bad)?,
                alignment: n.get("w:val").ok_or_else(bad)?.into(),
                leader: n.get("w:leader").map(str::to_owned),
            });
        }
        validate_tabs(&tabs)?;
        return Ok(Some(DefaultValue::Tabs(tabs)));
    }
    if parent.elements().next().is_some()
        || parent
            .children
            .iter()
            .any(|c| matches!(c,crate::xml::Content::Text(s) if !s.trim().is_empty()))
    {
        return Err(bad());
    }
    match parent.get(attr) {
        Some(s) => parse(kind, s).map(Some),
        None if matches!(kind, Toggle) && attr == "w:val" => Ok(Some(DefaultValue::Bool(true))),
        None if attr == "w:val" && !matches!(leaf, "w:u" | "w:lang") => Err(bad()),
        None => Ok(None),
    }
}
fn replaceable(n: &Node, attrs: &[&str]) -> bool {
    n.children.is_empty()
        && n.attrs
            .iter()
            .all(|(a, _)| a.declaration().is_some() || attrs.iter().any(|s| a == *s))
}
fn simple_tab_list(node: &Node) -> bool {
    node.attrs
        .iter()
        .all(|(name, _)| name.declaration().is_some())
        && node.children.iter().all(|child| match child {
            Content::Element(tab) => {
                tab.name == "w:tab" && replaceable(tab, &["w:val", "w:pos", "w:leader"])
            }
            _ => false,
        })
}
fn patch(root: &mut Node, f: DefaultField, value: Option<&DefaultValue>) -> Result<bool> {
    let before = read(root, f)?;
    if before.as_ref() == value {
        return Ok(false);
    }
    let (path, leaf, attr, kind) = f.spec();
    let parent = path.iter().fold(root, |p, s| p.ensure(s));
    if matches!(kind, Tabs) {
        if parent.child(leaf).is_some_and(|n| !simple_tab_list(n)) {
            return Err(unsupported("annotated tab stops"));
        }
        if let Some(DefaultValue::Tabs(tabs)) = value {
            validate_tabs(tabs)?;
            let mut n = Node::new(leaf);
            for t in tabs {
                let mut tab = Node::new("w:tab")
                    .attr("w:pos", t.position_twips)
                    .attr("w:val", &t.alignment);
                if let Some(l) = &t.leader {
                    tab.set("w:leader", l)
                }
                n.push(tab);
            }
            parent.replace(n);
        } else if value.is_none() {
            parent.remove(leaf)
        } else {
            return Err(bad());
        }
    } else if let Some(v) = value {
        let s = match v {
            DefaultValue::Bool(b) => if *b { "1" } else { "0" }.into(),
            DefaultValue::Integer(i) => i.to_string(),
            DefaultValue::Text(s) => s.clone(),
            DefaultValue::Tabs(_) => return Err(bad()),
        };
        if &parse(kind, &s)? != v {
            return Err(bad());
        }
        parent.ensure(leaf).set(attr, s);
    } else {
        let n = parent.child_mut(leaf).ok_or_else(bad)?;
        if replaceable(n, &[attr]) {
            parent.remove(leaf)
        } else if attr == "w:val" && !matches!(leaf, "w:lang" | "w:u" | "w:color") {
            return Err(unsupported("cannot clear an annotated document default"));
        } else {
            n.attrs.retain(|(a, _)| a != attr);
        }
    }
    Ok(true)
}
impl ImportedDocument {
    /// Directly present scalar defaults and physical tab stops (not inherited values).
    pub fn document_defaults(&self) -> Result<BTreeMap<DefaultField, DefaultValue>> {
        let mut values = BTreeMap::new();
        for (s, part) in [(false, Part::Defaults), (true, Part::Settings)] {
            if let Some((_, root)) = part.load(&self.package, &self.main)? {
                for &f in DefaultField::ALL.iter().filter(|&&f| settings(f) == s) {
                    if let Some(v) = read(&root, f)? {
                        values.insert(f, v);
                    }
                }
            }
        }
        Ok(values)
    }
}
pub(super) fn apply(pkg: &mut Package, main: &str, edits: &[ResourceEdit]) -> Result<()> {
    for (s, part) in [(false, Part::Defaults), (true, Part::Settings)] {
        let mut changes = BTreeMap::new();
        for edit in edits {
            if let ResourceEdit::Default(f, v) = edit
                && settings(*f) == s
                && changes.insert(*f, v.as_ref()).is_some()
            {
                return Err(unsupported("duplicate document default edit"));
            }
        }
        let legacy_set = !s
            && edits
                .iter()
                .any(|e| matches!(e, ResourceEdit::DefaultAlignment(Some(_))));
        if changes.is_empty() && !legacy_set {
            continue;
        }
        if changes.contains_key(&DefaultField::Justification)
            && edits
                .iter()
                .any(|e| matches!(e, ResourceEdit::DefaultAlignment(_)))
        {
            return Err(unsupported("duplicate default justification edit"));
        }
        let (name, mut root, mut changed) = match part.load(pkg, main)? {
            Some((name, root)) => (name, root, false),
            None if !legacy_set && changes.values().all(Option::is_none) => continue,
            None => {
                let kind = if s { "settings" } else { "styles" };
                let (path, root) = super::create_word_part(pkg, main, kind)?;
                (path, root, true)
            }
        };
        let color_edited = changes.keys().any(|f| f.spec().1 == "w:color");
        for (field, v) in changes {
            changed |= patch(&mut root, field, v)?;
        }
        // A theme still needs a fallback color, including when several color
        // attributes are changed together in one atomic batch.
        if color_edited
            && let Some(color) = RUN
                .iter()
                .try_fold(&root, |n, &k| n.child(k))
                .and_then(|n| n.child("w:color"))
            && color.get("w:val").is_none()
        {
            return Err(unsupported("default color requires a fallback value"));
        }
        if changed {
            pkg.put_word(name, root)?
        }
    }
    Ok(())
}
