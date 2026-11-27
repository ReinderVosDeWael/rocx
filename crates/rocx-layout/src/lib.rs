//! Experimental, font-pinned layout for fixed grids and physical pages.
//! Coordinates are 1/64 point; source offsets count Unicode scalar values.

mod bound;
mod import;
mod navigation;
mod pages;
mod physical;
mod selection;
pub use bound::OpenedPages;
pub use import::plain_section;
pub use navigation::{CaretMove, VisualPosition};
pub use physical::{PageSpec, PhysicalSnapshot, Row};
pub use selection::{Affinity, CaretBox, SelectionRect, TextPoint};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FlowAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FlowStyle {
    pub align: FlowAlign,
    pub before: i32,
    pub after: i32,
    pub left: i32,
    pub right: i32,
    pub first_line: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Widths {
    pub first: i32,
    pub rest: i32,
}
mod text;

use std::{mem::size_of, sync::Arc};

use lru::LruCache;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Glyph {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub advance: i32,
    pub cluster: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Caret {
    pub offset: u32,
    pub x: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Line {
    pub start: u32,
    pub end: u32,
    pub width: i32,
    pub glyphs: Vec<Glyph>,
    pub carets: Vec<Caret>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlacedLine {
    pub paragraph: usize,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Page {
    pub lines: Arc<[PlacedLine]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Reuse {
    pub shaped_hits: usize,
    pub line_hits: usize,
    pub page_hits: usize,
}

/// Current session-owned paragraph-cache charges. These exclude fonts,
/// checkpoint arrays, allocator/bookkeeping overhead and externally held
/// snapshots; `estimated_bytes` is not process or Wasm linear-memory usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CacheStats {
    pub paragraphs: usize,
    pub wrap_variants: usize,
    pub estimated_bytes: usize,
    pub budget_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutSnapshot {
    pub paragraphs: Vec<Arc<Vec<Line>>>,
    pub pages: Vec<Page>,
    pub reuse: Reuse,
}

impl LayoutSnapshot {
    /// Return the paragraph index and nearest scalar caret on a visual row.
    pub fn hit_row(&self, page: usize, row: usize, x: i32) -> Option<(usize, u32)> {
        let placement = self.pages.get(page)?.lines.get(row)?;
        let line = self
            .paragraphs
            .get(placement.paragraph)?
            .get(placement.line)?;
        if !(0..=line.width).contains(&x) {
            return None;
        }
        // LTR shaping produces monotone x. Resolve equidistant hits to the
        // earlier stop, including several stops rounded to the same x.
        let caret = nearest_caret(line, x)?;
        Some((placement.paragraph, caret.offset))
    }
}

fn nearest_caret(line: &Line, x: i32) -> Option<&Caret> {
    let right = line.carets.partition_point(|c| c.x < x);
    match (
        right.checked_sub(1).map(|i| &line.carets[i]),
        line.carets.get(right),
    ) {
        (Some(left), r) if r.is_none_or(|r| left.x.abs_diff(x) <= r.x.abs_diff(x)) => line
            .carets
            .get(line.carets.partition_point(|c| c.x < left.x)),
        (_, r) => r,
    }
}

struct Entry {
    shaped: text::Shaped,
    // MRU first. A repeated string can occur in several differently indented
    // paragraphs in the same revision without the widths evicting each other.
    wraps: [Option<Wrapped>; 4],
    base_cost: usize,
    cost: usize,
}

struct Wrapped {
    widths: Widths,
    lines: Arc<Vec<Line>>,
    cost: usize,
}

impl Entry {
    fn lines(&mut self, widths: Widths) -> Option<Arc<Vec<Line>>> {
        let index = self
            .wraps
            .iter()
            .position(|v| v.as_ref().is_some_and(|v| v.widths == widths))?;
        self.wraps[..=index].rotate_right(1);
        Some(Arc::clone(&self.wraps[0].as_ref()?.lines))
    }

    fn remember(&mut self, widths: Widths, lines: &Arc<Vec<Line>>, budget: usize) {
        let cost = lines.capacity() * size_of::<Line>()
            + lines
                .iter()
                .map(|l| {
                    l.glyphs.capacity() * size_of::<Glyph>()
                        + l.carets.capacity() * size_of::<Caret>()
                })
                .sum::<usize>();
        // A large output can be returned to the caller without evicting every
        // usable wrap variant or dropping an otherwise cacheable shape.
        if self.base_cost > budget || cost > budget - self.base_cost {
            return;
        }
        while self.cost > budget - cost || self.wraps.last().unwrap().is_some() {
            let last = self
                .wraps
                .iter()
                .rposition(Option::is_some)
                .expect("variant to evict");
            self.cost -= self.wraps[last].take().unwrap().cost;
        }
        self.wraps.rotate_right(1);
        self.wraps[0] = Some(Wrapped {
            widths,
            lines: Arc::clone(lines),
            cost,
        });
        self.cost += cost;
    }
}

type ParagraphGeometry = Vec<Arc<Vec<Line>>>;

/// A font and size are immutable session inputs. A different font needs a new
/// session; within a session paragraph text keys the shaping cache exactly.
pub struct LayoutSession {
    font: Vec<u8>,
    size: i32,
    entries: LruCache<Arc<str>, Entry>,
    used: usize,
    budget: usize,
    pages: pages::PageCache,
    physical: physical::PhysicalCache,
}

impl LayoutSession {
    pub fn new(font: Vec<u8>, font_size: i32) -> Result<Self, String> {
        Self::with_cache_budget(font, font_size, 16 << 20)
    }

    /// Inspect cache occupancy on demand, without changing recency.
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            paragraphs: self.entries.len(),
            wrap_variants: self
                .entries
                .iter()
                .map(|(_, e)| e.wraps.iter().flatten().count())
                .sum(),
            estimated_bytes: self.used,
            budget_bytes: self.budget,
        }
    }

    /// Discard reusable paragraph geometry and pagination checkpoints under
    /// host memory pressure. Existing snapshots and their positions stay valid.
    pub fn clear_caches(&mut self) {
        self.entries = LruCache::unbounded();
        self.used = 0;
        self.pages = Default::default();
        self.physical = Default::default();
    }

    /// Approximate byte budget for paragraph shaping and wrapping. Page
    /// composition separately retains the previous document's checkpoint data.
    pub fn with_cache_budget(font: Vec<u8>, size: i32, budget: usize) -> Result<Self, String> {
        if size <= 0 || rustybuzz::Face::from_slice(&font, 0).is_none() {
            return Err("expected a valid font face and positive size".into());
        }
        Ok(Self {
            font,
            size,
            // Grow on demand. Enforce both limits at insertion, without
            // allocating a full hash table for a one-paragraph session.
            entries: LruCache::unbounded(),
            budget,
            used: 0,
            pages: Default::default(),
            physical: Default::default(),
        })
    }

    pub fn layout(
        &mut self,
        paragraphs: &[String],
        width: i32,
        lines_per_page: usize,
    ) -> Result<LayoutSnapshot, String> {
        if width <= 0 || lines_per_page == 0 {
            return Err("expected positive width and lines per page".into());
        }
        let (geometry, mut reuse) = self.paragraph_geometry(paragraphs, width)?;
        let (pages, hits) = self.pages.paginate(&geometry, lines_per_page)?;
        reuse.page_hits = hits;
        Ok(LayoutSnapshot {
            paragraphs: geometry,
            pages,
            reuse,
        })
    }

    fn paragraph_geometry(
        &mut self,
        paragraphs: &[String],
        width: i32,
    ) -> Result<(ParagraphGeometry, Reuse), String> {
        self.paragraph_geometry_for(paragraphs, |_| {
            Ok(Widths {
                first: width,
                rest: width,
            })
        })
    }

    fn paragraph_geometry_for(
        &mut self,
        paragraphs: &[String],
        width_for: impl Fn(usize) -> Result<Widths, String>,
    ) -> Result<(ParagraphGeometry, Reuse), String> {
        let mut reuse = Reuse::default();
        let mut geometry = Vec::with_capacity(paragraphs.len());
        for (index, paragraph) in paragraphs.iter().enumerate() {
            let text = paragraph.as_str();
            let widths = width_for(index)?;
            if let Some(lines) = self.entries.get_mut(text).and_then(|e| e.lines(widths)) {
                reuse.shaped_hits += 1;
                reuse.line_hits += 1;
                geometry.push(lines);
                continue;
            }
            let (key, mut entry) = match self.entries.pop_entry(text) {
                Some((key, entry)) => {
                    self.used -= entry.cost;
                    reuse.shaped_hits += 1;
                    (key, entry)
                }
                None => {
                    let shaped = text::shape(text, &self.font, self.size)
                        .map_err(|e| format!("paragraph {index}: {e}"))?;
                    let cost = size_of::<Entry>() + text.len() + shaped.heap_bytes();
                    (
                        Arc::from(text),
                        Entry {
                            shaped,
                            wraps: std::array::from_fn(|_| None),
                            base_cost: cost,
                            cost,
                        },
                    )
                }
            };
            let lines = Arc::new(
                text::wrap_widths(&entry.shaped, widths)
                    .map_err(|e| format!("paragraph {index}: {e}"))?,
            );
            entry.remember(widths, &lines, self.budget);
            geometry.push(lines);
            if entry.cost <= self.budget {
                while self.used > self.budget - entry.cost || self.entries.len() >= 8192 {
                    self.used -= self.entries.pop_lru().unwrap().1.cost;
                }
                self.used += entry.cost;
                self.entries.put(key, entry);
            }
        }
        Ok((geometry, reuse))
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn clear_releases_charged_geometry_but_preserves_prior_hittable_snapshots() {
        let mut s = LayoutSession::new(
            include_bytes!("../tests/fonts/DejaVuSans-subset.ttf").to_vec(),
            768,
        )
        .unwrap();
        let paras = vec!["alpha".to_string(), "beta".to_string()];
        let before = s.layout(&paras, 5_000, 2).unwrap();
        s.layout(&paras, 7_000, 2).unwrap();
        let stats = s.cache_stats();
        assert_eq!((stats.paragraphs, stats.wrap_variants), (2, 4));
        assert!(stats.estimated_bytes > 0 && stats.estimated_bytes <= stats.budget_bytes);
        s.clear_caches();
        assert_eq!(
            s.cache_stats(),
            CacheStats {
                paragraphs: 0,
                wrap_variants: 0,
                estimated_bytes: 0,
                budget_bytes: stats.budget_bytes
            }
        );
        let cold = s.layout(&paras, 5_000, 2).unwrap();
        assert_eq!(cold.reuse, Reuse::default());
        assert_eq!(cold.paragraphs, before.paragraphs);
        assert_eq!(before.hit_row(0, 1, 0), Some((1, 0)));
    }

    #[test]
    fn distinct_first_line_widths_share_shape_and_retain_recent_wraps() {
        let mut s = LayoutSession::new(
            include_bytes!("../tests/fonts/DejaVuSans-subset.ttf").to_vec(),
            768,
        )
        .unwrap();
        let text = vec!["aa aa aa".to_owned(); 2];
        let widths = |i| {
            Ok(Widths {
                first: if i == 0 { 600 } else { 2400 },
                rest: 3000,
            })
        };
        let (first, stats) = s.paragraph_geometry_for(&text, widths).unwrap();
        assert_eq!((stats.shaped_hits, stats.line_hits), (1, 0));
        assert_ne!(first[0][0].end, first[1][0].end);
        let (second, stats) = s.paragraph_geometry_for(&text, widths).unwrap();
        assert_eq!((stats.shaped_hits, stats.line_hits), (2, 2));
        assert!(Arc::ptr_eq(&first[0], &second[0]) && Arc::ptr_eq(&first[1], &second[1]));
        // Insert three further profiles; recently hit second profile survives
        // the fourth slot's LRU eviction, the original first profile does not.
        for rest in [3100, 3200, 3300] {
            s.paragraph_geometry_for(&text[..1], |_| Ok(Widths { first: 600, rest }))
                .unwrap();
        }
        let (_, hit) = s.paragraph_geometry_for(&text[..1], |_| widths(1)).unwrap();
        assert_eq!(hit.line_hits, 1);
        let (_, miss) = s.paragraph_geometry_for(&text[..1], |_| widths(0)).unwrap();
        assert_eq!((miss.shaped_hits, miss.line_hits), (1, 0));
    }

    #[test]
    fn nearest_caret_uses_first_coincident_stop_at_an_exact_hit_or_tie() {
        let line = Line {
            start: 0,
            end: 3,
            width: 10,
            glyphs: vec![],
            carets: [0, 4, 4, 10]
                .into_iter()
                .enumerate()
                .map(|(i, x)| Caret {
                    offset: i as u32,
                    x,
                })
                .collect(),
        };
        let snapshot = LayoutSnapshot {
            paragraphs: vec![Arc::new(vec![line])],
            pages: vec![Page {
                lines: vec![PlacedLine {
                    paragraph: 0,
                    line: 0,
                }]
                .into(),
            }],
            reuse: Reuse::default(),
        };
        for (x, expected) in [(2, 0), (4, 1), (5, 1), (7, 1), (8, 3)] {
            assert_eq!(snapshot.hit_row(0, 0, x), Some((0, expected)));
        }
    }

    #[test]
    fn byte_pressure_evicts_least_recent_paragraph_and_keeps_old_hits_valid() {
        let font = include_bytes!("../tests/fonts/DejaVuSans-subset.ttf");
        let make = || LayoutSession::new(font.to_vec(), 12 * 64).unwrap();
        let input = |text: &str| vec![text.to_owned()];
        let mut probe = make();
        let costs = ["aaa", "bbb", "ccc"].map(|s| {
            probe.layout(&input(s), 10_000, 1).unwrap();
            probe.entries.peek(s).unwrap().cost
        });
        let budget = costs.iter().sum::<usize>() - costs.iter().min().unwrap();
        let mut session = LayoutSession::with_cache_budget(font.to_vec(), 12 * 64, budget).unwrap();
        let first = session.layout(&input("aaa"), 10_000, 1).unwrap();
        session.layout(&input("bbb"), 10_000, 1).unwrap();
        assert_eq!(
            session
                .layout(&input("aaa"), 10_000, 1)
                .unwrap()
                .reuse
                .line_hits,
            1
        );
        session.layout(&input("ccc"), 10_000, 1).unwrap();
        let reused = session.layout(&input("aaa"), 10_000, 1).unwrap();
        assert_eq!(reused.reuse.line_hits, 1);
        assert!(Arc::ptr_eq(&first.paragraphs[0], &reused.paragraphs[0]));
        assert_eq!(
            session
                .layout(&input("bbb"), 10_000, 1)
                .unwrap()
                .reuse
                .shaped_hits,
            0
        );
        assert_eq!(
            first.hit_row(0, 0, first.paragraphs[0][0].width),
            Some((0, 3))
        );
    }
}
