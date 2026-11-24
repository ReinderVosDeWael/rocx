//! Work in original XML child slots; apply body splices only after all other
//! anchored edits have resolved. A section owns its END marker in Word XML.
use super::{
    BlockEdit, BoundaryState, ImportedDocument,
    blocks::Plan,
    header::{Slot, rewrite_section_ranges, section_props, section_slots},
    missing_body, unsupported,
};
use crate::Result;
use crate::xml::{Content, Node};
use std::collections::BTreeSet;

const HEADER: &str = "w:headerReference";
const FOOTER: &str = "w:footerReference";

// Only duplicate/discard recognized value markup. In particular printer
// settings, revision history, extension identities and MCE are not values.
fn portable(n: &Node, parent: &str) -> bool {
    let attrs = match (parent, n.name.as_str()) {
        ("", "w:sectPr") => "w:rsidR w:rsidDel w:rsidRPr w:rsidSect",
        ("w:sectPr", HEADER | FOOTER) => "w:type r:id",
        ("w:sectPr", "w:pgSz") => "w:w w:h w:orient w:code",
        ("w:sectPr", "w:pgMar") => "w:top w:left w:right w:bottom w:gutter w:header w:footer",
        ("w:sectPr", "w:docGrid") => "w:type w:linePitch w:charSpace",
        ("w:sectPr", "w:cols") => "w:num w:space w:sep w:equalWidth",
        ("w:cols", "w:col") => "w:w w:space",
        ("w:sectPr", "w:paperSrc") => "w:first w:other",
        ("w:sectPr", "w:pgNumType") => "w:fmt w:start w:chapStyle w:chapSep",
        ("w:sectPr", "w:lnNumType") => "w:countBy w:start w:distance w:restart",
        (
            "w:sectPr",
            "w:type" | "w:titlePg" | "w:bidi" | "w:rtlGutter" | "w:noEndnote" | "w:formProt"
            | "w:textDirection" | "w:vAlign",
        ) => "w:val",
        ("w:sectPr", "w:pgBorders") => "w:offsetFrom w:display w:zOrder",
        ("w:pgBorders", "w:top" | "w:bottom" | "w:left" | "w:right") => {
            "w:val w:sz w:space w:color w:themeColor w:themeTint w:themeShade w:shadow w:frame"
        }
        _ => return false,
    };
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.split_whitespace().any(|a| k == a))
        && n.children.iter().all(|c| match c {
            Content::Element(child) => portable(child, n.name.as_str()),
            Content::Text(t) => t.trim().is_empty(),
            Content::Raw(_) => false,
        })
}

fn empty(n: &Node) -> bool {
    n.attrs.iter().all(|(k, _)| k.declaration().is_some())
        && n.children
            .iter()
            .all(|c| matches!(c, Content::Text(t) if t.trim().is_empty()))
}

pub(super) fn build(
    doc: &ImportedDocument,
    root: &mut Node,
    edits: &[BlockEdit],
    occupied: &BTreeSet<usize>,
    after: &BTreeSet<usize>,
) -> Result<Plan> {
    let ranges = rewrite_section_ranges(root, &doc.package, &doc.main)?
        .into_iter()
        .map(|(r, _)| r)
        .collect::<Vec<_>>();
    let slots = section_slots(root)?
        .into_iter()
        .map(|s| s.0)
        .collect::<Vec<_>>();
    let body = root.child_mut("w:body").ok_or_else(missing_body)?;
    let mut locations = Vec::new();
    let mut safe = Vec::new();
    let mut state = BoundaryState::default();
    for (i, child) in body.children.iter().enumerate() {
        if let Content::Element(n) = child {
            locations.push(i);
            safe.push(state.safe());
            if n.name != "w:sectPr" {
                state.advance(n);
            }
        }
    }
    locations.push(body.children.len());
    safe.push(state.safe());
    let mut claimed = BTreeSet::new();
    let mut plan = Plan::default();
    for edit in edits {
        let (anchor, split) = match edit {
            BlockEdit::SplitSection(a, offset) => (a, Some(*offset)),
            BlockEdit::MergeSection(a) => (a, None),
            _ => {
                return Err(unsupported(
                    "section lifecycle conflicts with other block edits",
                ));
            }
        };
        doc.validate_snapshot(&anchor.snapshot)?;
        let i = anchor.index;
        let last = if split.is_some() { i } else { i + 1 };
        if last >= slots.len() {
            return Err(unsupported("cannot remove the final section boundary"));
        }
        for j in i..=last {
            let r = &ranges[j];
            if !claimed.insert(j)
                || occupied.range(r.clone()).next().is_some()
                || r.start.checked_sub(1).is_some_and(|n| after.contains(&n))
            {
                return Err(unsupported("conflicting edits in affected section range"));
            }
            if !safe[r.end] || matches!(slots[j], Slot::Paragraph(_)) && !safe[r.end - 1] {
                return Err(unsupported("protected section boundary"));
            }
            if let Slot::Paragraph(slot) = slots[j] {
                let Content::Element(p) = &body.children[slot] else {
                    unreachable!()
                };
                let props = p.child("w:pPr").unwrap();
                if props.child("w:pPrChange").is_some()
                    || props.child(super::ALTERNATE).is_some()
                    || props.child("w:rPr").is_some_and(|r| {
                        r.elements().any(|n| {
                            matches!(
                                n.name.as_str(),
                                "w:ins" | "w:del" | "w:moveFrom" | "w:moveTo"
                            )
                        })
                    })
                {
                    return Err(unsupported("revised section boundary owner"));
                }
            }
        }
        let donor = match slots[i] {
            Slot::Terminal => body.child("w:sectPr"),
            Slot::Paragraph(_) => Some(&*section_props(body, slots[i])?),
        };
        if donor.is_some_and(|n| !portable(n, "")) {
            return Err(unsupported(
                "section contains opaque or resource-bearing properties",
            ));
        }
        let donor = donor.cloned().unwrap_or_else(|| Node::new("w:sectPr"));
        if let Some(offset) = split {
            let r = &ranges[i];
            if offset > r.len() || offset == r.len() && matches!(slots[i], Slot::Paragraph(_)) {
                return Err(unsupported("split gap outside section"));
            }
            let gap = r.start + offset;
            if !safe[gap] {
                return Err(unsupported("split crosses a protected range"));
            }
            // The inserted prefix takes the original explicit slots. The
            // suffix inherits those exact parts (no copied relationships).
            if donor
                .elements()
                .any(|n| matches!(n.name.as_str(), HEADER | FOOTER))
            {
                let old = section_props(body, slots[i])?;
                old.remove(HEADER);
                old.remove(FOOTER);
            }
            let mut p = Node::new("w:p");
            p.ensure("w:pPr").push(donor);
            plan.boundaries.push((locations[gap], p));
            plan.changes.push(gap);
        } else {
            for reference in donor
                .elements()
                .filter(|n| matches!(n.name.as_str(), HEADER | FOOTER))
            {
                let next = section_props(body, slots[last])?;
                if next
                    .elements()
                    .any(|n| n.name == reference.name && n.get("w:type") == reference.get("w:type"))
                {
                    continue;
                }
                if next.child("w:sectPrChange").is_some() {
                    return Err(unsupported("revised section reference target"));
                }
                next.push(reference.clone());
            }
            let Slot::Paragraph(slot) = slots[i] else {
                unreachable!()
            };
            let Content::Element(p) = &mut body.children[slot] else {
                unreachable!()
            };
            let props = p.child_mut("w:pPr").unwrap();
            props.remove("w:sectPr");
            if empty(props) {
                p.remove("w:pPr");
                if empty(p) {
                    plan.deletes.insert(slot);
                }
            }
            plan.changes.push(ranges[i].end - 1);
        }
    }
    plan.changes.sort_unstable();
    Ok(plan)
}
