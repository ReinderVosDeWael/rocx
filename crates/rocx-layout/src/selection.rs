//! Map scalar boundaries back to their visual extent in a physical snapshot.
use crate::{Line, PhysicalSnapshot, Row};
use serde::Serialize;

/// Logical position in this snapshot, measured in Unicode scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TextPoint {
    pub paragraph: usize,
    pub offset: u32,
}

/// Which visual caret to use when a soft wrap gives one offset two locations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Affinity {
    Upstream,
    Downstream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CaretBox {
    pub page: usize,
    pub row: usize,
    pub x: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SelectionRect {
    pub page: usize,
    pub row: usize,
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl PhysicalSnapshot {
    fn text_line(&self, row: &Row) -> &Line {
        &self.layout.paragraphs[row.paragraph][row.line]
    }

    /// Look up an exact legal caret stop; never split a combining sequence.
    /// Coordinates are page relative. Hard breaks have distinct before/after
    /// offsets, whereas soft wraps (including page wraps) need affinity.
    pub fn caret_box(&self, point: TextPoint, affinity: Affinity) -> Option<CaretBox> {
        let ends_before =
            |r: &Row| (r.paragraph, self.text_line(r).end) < (point.paragraph, point.offset);
        let mut page = self
            .rows
            .partition_point(|rows| rows.last().is_none_or(ends_before));
        let rows = self.rows.get(page)?;
        let mut index = rows.partition_point(ends_before);
        let mut row = rows.get(index)?;
        if affinity == Affinity::Downstream && self.text_line(row).end == point.offset {
            let next = if index + 1 < rows.len() {
                Some((page, index + 1, &rows[index + 1]))
            } else {
                self.rows
                    .get(page + 1)
                    .and_then(|rows| rows.first())
                    .map(|r| (page + 1, 0, r))
            };
            if let Some((p, i, r)) = next
                && r.paragraph == point.paragraph
                && self.text_line(r).start == point.offset
            {
                page = p;
                index = i;
                row = r;
            }
        }
        if row.paragraph != point.paragraph {
            return None;
        }
        let line = self.text_line(row);
        let i = line
            .carets
            .binary_search_by_key(&point.offset, |stop| stop.offset)
            .ok()?;
        Some(CaretBox {
            page,
            row: index,
            x: row.x.checked_add(line.carets[i].x)?,
            top: row.top,
            bottom: row.bottom,
        })
    }

    /// Visible text extents in document order, one per nonzero-width selected
    /// line. Paragraph terminators, LF and spacing have no painted advance.
    /// Reverse selections are accepted; invalid scalar stops return `None`.
    pub fn selection_rects(
        &self,
        anchor: TextPoint,
        focus: TextPoint,
    ) -> Option<Vec<SelectionRect>> {
        let (a, b) = if anchor <= focus {
            (anchor, focus)
        } else {
            (focus, anchor)
        };
        let first = self.caret_box(a, Affinity::Downstream)?;
        if a == b {
            return Some(vec![]);
        }
        let last = self.caret_box(b, Affinity::Upstream)?;
        let mut rects = Vec::new();
        for page in first.page..=last.page {
            let rows = &self.rows[page];
            let from = if page == first.page { first.row } else { 0 };
            let until = if page == last.page {
                last.row + 1
            } else {
                rows.len()
            };
            for (i, row) in rows[from..until].iter().enumerate() {
                let index = from + i;
                let left = if (page, index) == (first.page, first.row) {
                    first.x
                } else {
                    row.x
                };
                let right = if (page, index) == (last.page, last.row) {
                    last.x
                } else {
                    row.x.checked_add(self.text_line(row).width)?
                };
                if left < right {
                    rects.push(SelectionRect {
                        page,
                        row: index,
                        left,
                        right,
                        top: row.top,
                        bottom: row.bottom,
                    });
                }
            }
        }
        Some(rects)
    }
}
