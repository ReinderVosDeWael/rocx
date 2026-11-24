use super::{
    InlineAnchor, InlineLocation, ParagraphAnchor, RunTextEdit, insertion::BoundaryState,
    unsupported,
};
use crate::{
    Result,
    xml::{Content, Node},
};
use std::collections::BTreeMap;

/// Replace `[start, end)` Unicode scalar values in a plain direct body paragraph.
/// Coordinates refer to the original snapshot. Empty selections are rejected;
/// an empty replacement deletes text while retaining all run containers.
#[derive(Clone, Debug)]
pub struct TextRangeEdit {
    pub target: ParagraphAnchor,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Insert nonempty text at an original Unicode scalar boundary. At a run
/// boundary the right-hand run owns the insertion; at paragraph end the last
/// run owns it. An initially runless paragraph gets a plain run.
#[derive(Clone, Debug)]
pub struct TextInsertion {
    pub target: ParagraphAnchor,
    pub position: usize,
    pub text: String,
}

struct Patch<'a> {
    target: &'a ParagraphAnchor,
    start: usize,
    end: usize,
    text: &'a str,
}

fn plain_children(n: &Node) -> bool {
    n.children.iter().all(|c| {
        matches!(c, Content::Element(_))
            || matches!(c, Content::Text(s) if s.bytes().all(|b| matches!(b,b' '|b'\t'|b'\n'|b'\r')))
    })
}

fn tracked_properties(n: &Node) -> bool {
    matches!(
        n.name.as_str(),
        "w:ins"
            | "w:del"
            | "w:moveFrom"
            | "w:moveTo"
            | "w:rPrChange"
            | "w:pPrChange"
            | "w:sectPrChange"
    ) || n.elements().any(tracked_properties)
}

pub(super) fn expand(
    body: &mut Node,
    ranges: &[TextRangeEdit],
    points: &[TextInsertion],
) -> Result<Vec<RunTextEdit>> {
    let mut grouped = BTreeMap::<usize, Vec<Patch<'_>>>::new();
    for edit in ranges {
        if edit.start >= edit.end {
            return Err(unsupported("empty or reversed text range"));
        }
        grouped
            .entry(edit.target.body_index)
            .or_default()
            .push(Patch {
                target: &edit.target,
                start: edit.start,
                end: edit.end,
                text: &edit.text,
            });
    }
    for edit in points {
        if edit.text.is_empty() {
            return Err(unsupported("text insertion requires nonempty text"));
        }
        grouped
            .entry(edit.target.body_index)
            .or_default()
            .push(Patch {
                target: &edit.target,
                start: edit.position,
                end: edit.position,
                text: &edit.text,
            });
    }
    let mut result = Vec::new();
    let mut state = BoundaryState::default();
    for (index, p) in body
        .elements_mut()
        .filter(|n| n.name != "w:sectPr")
        .enumerate()
    {
        if grouped.is_empty() {
            break;
        }
        if let Some(mut edits) = grouped.remove(&index) {
            let invalid = || {
                unsupported(
                    "range requires a plain direct body paragraph outside fields and ranges",
                )
            };
            if !state.safe()
                || p.name != "w:p"
                || !plain_children(p)
                || p.elements()
                    .filter(|n| n.name == "w:pPr")
                    .any(tracked_properties)
            {
                return Err(invalid());
            }
            let mut original = String::new();
            let mut ends = Vec::new();
            for run in p.elements().filter(|n| n.name != "w:pPr") {
                if run.name != "w:r"
                    || !plain_children(run)
                    || run
                        .elements()
                        .filter(|n| n.name == "w:rPr")
                        .any(tracked_properties)
                    || run
                        .elements()
                        .filter(|n| n.name != "w:rPr")
                        .map(|n| n.name.as_str())
                        .ne(["w:t"])
                {
                    return Err(invalid());
                }
                for c in &run.child("w:t").unwrap().children {
                    let Content::Text(s) = c else {
                        return Err(invalid());
                    };
                    if s.contains(['\t', '\r', '\n']) {
                        return Err(invalid());
                    }
                    original.push_str(s);
                }
                ends.push(original.len());
            }
            if ends.is_empty() {
                // Materialize an insertion host in the private staging tree.
                // Bounds and payload validation below still apply.
                let mut run = Node::new("w:r");
                run.children.push(Content::Element(Node::new("w:t")));
                p.children.push(Content::Element(run));
                ends.push(0);
            }
            edits.sort_by_key(|r| (r.start, r.end));
            let mut prior_end = 0;
            let mut prior_byte = 0;
            let mut prior_point = None;
            // Accumulate patches against original byte coordinates, even for
            // adjacent ranges in one run. Keep unchanged targets for conflict checks.
            let mut patches = BTreeMap::<usize, Vec<(usize, usize, &str)>>::new();
            let anchor = edits[0].target;
            for r in edits {
                let point = r.start == r.end;
                if r.start < prior_end || (point && prior_point == Some(r.start)) {
                    return Err(unsupported("overlapping text edits"));
                }
                prior_point = point.then_some(r.start);
                let advance = |byte, count| {
                    original[byte..]
                        .char_indices()
                        .map(|(i, _)| i)
                        .chain([original.len() - byte])
                        .nth(count)
                        .map(|i| byte + i)
                        .ok_or_else(|| unsupported("out-of-bounds text range"))
                };
                let start = advance(prior_byte, r.start - prior_end)?;
                let end = advance(start, r.end - r.start)?;
                prior_end = r.end;
                prior_byte = end;
                let first = ends
                    .partition_point(|&pos| pos <= start)
                    .min(ends.len() - 1);
                let last = if point {
                    first
                } else {
                    ends.partition_point(|&pos| pos < end)
                };
                let unchanged = &original[start..end] == r.text;
                for run in first..=last {
                    let from = start.max(if run == 0 { 0 } else { ends[run - 1] });
                    let to = end.min(ends[run]);
                    let replacement = if unchanged {
                        &original[from..to]
                    } else if run == first {
                        r.text
                    } else {
                        ""
                    };
                    patches
                        .entry(run)
                        .or_default()
                        .push((from, to, replacement));
                }
            }
            for (run, fragments) in patches {
                let mut cursor = if run == 0 { 0 } else { ends[run - 1] };
                let mut text = String::new();
                for (from, to, s) in fragments {
                    text.push_str(&original[cursor..from]);
                    text.push_str(s);
                    cursor = to;
                }
                text.push_str(&original[cursor..ends[run]]);
                result.push(RunTextEdit {
                    target: InlineAnchor {
                        snapshot: anchor.snapshot.clone(),
                        location: InlineLocation::Body(index),
                        path: vec![run],
                    },
                    text,
                });
            }
        }
        state.advance(p);
    }
    if !grouped.is_empty() {
        return Err(unsupported("range paragraph not found"));
    }
    Ok(result)
}
