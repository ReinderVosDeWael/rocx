use super::{ALTERNATE, unsupported};
use crate::Result;
use crate::xml::{Content, Node};
use std::collections::BTreeMap;

macro_rules! flags {
    ($name:ident { $($variant:ident: $xml:literal, $api:literal, $python:literal;)+ }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($variant,)+ }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];
            pub const fn name(self) -> &'static str { match self { $(Self::$variant => $api,)+ } }
            pub const fn python_name(self) -> &'static str { match self { $(Self::$variant => $python,)+ } }
            pub(super) const fn xml(self) -> &'static str { match self { $(Self::$variant => concat!("w:", $xml),)+ } }
            pub(super) fn read(props: Option<&Node>) -> BTreeMap<Self,bool> {
                let Some(p) = props.filter(|p| p.child(ALTERNATE).is_none()) else { return BTreeMap::new() };
                Self::ALL.iter().filter_map(|&flag| {
                    let mut nodes = p.elements().filter(|n| n.name == flag.xml());
                    let first = nodes.next()?;
                    nodes.next().is_none().then(|| on_off(first))?.map(|value| (flag,value))
                }).collect()
            }
        }
    }
}
flags!(ParagraphFlag {
    KeepWithNext: "keepNext", "keepWithNext", "keep_with_next";
    KeepTogether: "keepLines", "keepTogether", "keep_together";
    PageBreakBefore: "pageBreakBefore", "pageBreakBefore", "page_break_before";
    WidowControl: "widowControl", "widowControl", "widow_control";
    SuppressLineNumbers: "suppressLineNumbers", "suppressLineNumbers", "suppress_line_numbers";
    Bidi: "bidi", "bidi", "bidi";
    SnapToGrid: "snapToGrid", "snapToGrid", "snap_to_grid";
    ContextualSpacing: "contextualSpacing", "contextualSpacing", "contextual_spacing";
    MirrorIndents: "mirrorIndents", "mirrorIndents", "mirror_indents";
});
flags!(SectionFlag {
    TitlePage: "titlePg", "titlePage", "title_page";
    Bidi: "bidi", "bidi", "bidi";
    RtlGutter: "rtlGutter", "rtlGutter", "rtl_gutter";
});

fn on_off(n: &Node) -> Option<bool> {
    match n.get("w:val") {
        None | Some("true" | "on" | "1") => Some(true),
        Some("false" | "off" | "0") => Some(false),
        _ => None,
    }
}

pub(super) fn patch(props: &mut Node, name: &str, value: Option<&str>) -> Result<bool> {
    if props.elements().filter(|n| n.name == name).count() > 1
        || props.elements().any(|n| {
            matches!(
                n.name.as_str(),
                ALTERNATE | "w:pPrChange" | "w:sectPrChange"
            )
        })
    {
        return Err(unsupported("ambiguous or revised properties"));
    }
    let current = props.child(name);
    if let Some(value) = value {
        if current.is_some_and(|n| {
            n.get("w:val") == Some(value)
                || (name != "w:type"
                    && on_off(n).is_some_and(|v| value == if v { "1" } else { "0" }))
        }) {
            return Ok(false);
        }
        props.ensure(name).set("w:val", value);
    } else if let Some(n) = current {
        if n.attrs
            .iter()
            .any(|(k, _)| k != "w:val" && k.declaration().is_none())
            || n.children
                .iter()
                .any(|c| !matches!(c, Content::Text(s) if s.trim().is_empty()))
        {
            return Err(unsupported(
                "cannot clear property containing opaque markup",
            ));
        }
        props.remove(name);
    } else {
        return Ok(false);
    }
    Ok(true)
}
