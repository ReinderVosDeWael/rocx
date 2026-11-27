use rocx::editor::PairedRangeEdit;
use serde_json::{Value, json};

use super::{ReadError, Snapshot};
use crate::schema::RangeEndpoints;

impl Snapshot {
    pub fn range_marker_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        let mut markers = self.document.body_range_markers()?.into_iter().skip(start);
        let rows: Vec<_> = markers
            .by_ref()
            .take(limit)
            .map(|marker| {
                json!({
                    "body_index": marker.body_index,
                    "inline_index": marker.inline_index,
                    "kind": marker.kind,
                    "id": marker.id,
                    "name": marker.name,
                })
            })
            .collect();
        let next = markers.next().map(|_| start + rows.len());
        Ok((rows, next))
    }

    pub(super) fn paired_range_edit(
        &self,
        id: u32,
        bookmark: Option<String>,
        endpoints: Option<RangeEndpoints>,
    ) -> Result<PairedRangeEdit, String> {
        let endpoints = if let Some(endpoints) = endpoints {
            let first = self.body_inline_anchor(&endpoints.first)?.clone();
            let last = self.body_inline_anchor(&endpoints.last)?.clone();
            Some((first, last))
        } else {
            None
        };
        Ok(PairedRangeEdit {
            id,
            bookmark,
            endpoints,
        })
    }
}
