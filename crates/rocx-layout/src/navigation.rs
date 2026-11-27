//! Visual caret navigation for the horizontal left-to-right profile.
use serde::Serialize;
use std::sync::Arc;

use crate::{Affinity, PhysicalSnapshot, Row, TextPoint, nearest_caret};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretMove {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
}

/// Snapshot-local caret and the target x to retain on successive vertical moves.
#[derive(Clone, Debug, Serialize)]
pub struct VisualPosition {
    pub point: TextPoint,
    pub affinity: Affinity,
    pub preferred_x: Option<i32>,
    #[serde(skip)]
    owner: Arc<()>,
}

impl PhysicalSnapshot {
    fn adjacent_row(&self, page: usize, row: usize, forward: bool) -> Option<(usize, usize, &Row)> {
        let rows = self.rows.get(page)?;
        if forward {
            if let Some(r) = rows.get(row + 1) {
                Some((page, row + 1, r))
            } else {
                Some((page + 1, 0, self.rows.get(page + 1)?.first()?))
            }
        } else if let Some(index) = row.checked_sub(1) {
            Some((page, index, rows.get(index)?))
        } else {
            let page = page.checked_sub(1)?;
            let rows = &self.rows[page];
            Some((page, rows.len().checked_sub(1)?, rows.last()?))
        }
    }

    fn at_row_stop(&self, row: &Row, offset: u32, preferred_x: Option<i32>) -> VisualPosition {
        let lines = &self.layout.paragraphs[row.paragraph];
        let affinity =
            if row.line > 0 && lines[row.line].start == offset && lines[row.line - 1].end == offset
            {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            };
        VisualPosition {
            point: TextPoint {
                paragraph: row.paragraph,
                offset,
            },
            affinity,
            preferred_x,
            owner: Arc::clone(&self.owner),
        }
    }

    /// Bind caller-supplied scalar coordinates to this snapshot.
    pub fn visual_position(
        &self,
        point: TextPoint,
        affinity: Affinity,
        preferred_x: Option<i32>,
    ) -> Option<VisualPosition> {
        let caret = self.caret_box(point, affinity)?;
        Some(self.at_row_stop(&self.rows[caret.page][caret.row], point.offset, preferred_x))
    }

    /// Like `hit_test`, retaining which side of a soft wrap the user hit.
    pub fn hit_position(&self, page: usize, x: i32, y: i32) -> Option<VisualPosition> {
        let rows = self.rows.get(page)?;
        let i = rows.partition_point(|r| r.bottom <= y);
        let row = rows.get(i)?;
        if y < row.top {
            return None;
        }
        let (_, offset) = self.layout.hit_row(page, i, x.checked_sub(row.x)?)?;
        Some(self.at_row_stop(row, offset, None))
    }

    /// Move one visual stop or row. Left/right visit both visual sides of a
    /// soft wrap, and never stop inside a grapheme. Up/down preserve page-local
    /// preferred x across shorter lines and margins. At document edges a move
    /// past the available rows returns `None`.
    pub fn move_caret(
        &self,
        position: &VisualPosition,
        direction: CaretMove,
    ) -> Option<VisualPosition> {
        if !Arc::ptr_eq(&position.owner, &self.owner) {
            return None;
        }
        let current = self.caret_box(position.point, position.affinity)?;
        let row = &self.rows[current.page][current.row];
        let line = &self.layout.paragraphs[row.paragraph][row.line];
        let stops = &line.carets;
        match direction {
            CaretMove::LineStart => Some(self.at_row_stop(row, stops.first()?.offset, None)),
            CaretMove::LineEnd => Some(self.at_row_stop(row, stops.last()?.offset, None)),
            CaretMove::Left | CaretMove::Right => {
                let forward = direction == CaretMove::Right;
                let i = stops
                    .binary_search_by_key(&position.point.offset, |c| c.offset)
                    .ok()?;
                let stop = if forward {
                    stops.get(i + 1)
                } else {
                    i.checked_sub(1).and_then(|j| stops.get(j))
                };
                if let Some(stop) = stop {
                    return Some(self.at_row_stop(row, stop.offset, None));
                }
                let (_, _, next) = self.adjacent_row(current.page, current.row, forward)?;
                let stops = &self.layout.paragraphs[next.paragraph][next.line].carets;
                let next_stop = if forward {
                    stops.first()?
                } else {
                    stops.last()?
                };
                Some(self.at_row_stop(next, next_stop.offset, None))
            }
            CaretMove::Up | CaretMove::Down => {
                let (_, _, next) =
                    self.adjacent_row(current.page, current.row, direction == CaretMove::Down)?;
                let preferred = position.preferred_x.unwrap_or(current.x);
                let target = &self.layout.paragraphs[next.paragraph][next.line];
                let stop = nearest_caret(target, preferred.saturating_sub(next.x))?;
                Some(self.at_row_stop(next, stop.offset, Some(preferred)))
            }
        }
    }
}
