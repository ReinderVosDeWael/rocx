use std::sync::{Arc, OnceLock, Weak};

use serde::Serialize;

use crate::{FlowAlign, FlowStyle, LayoutSession, LayoutSnapshot, Line, Page, PlacedLine, Widths};

/// Page dimensions, margin insets and paragraph spacing, all in 1/64 point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PageSpec {
    pub width: i32,
    pub height: i32,
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub space_before: i32,
    pub space_after: i32,
}

/// Page-local line bounds. Leading lies outside the half-open row box.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub paragraph: usize,
    pub line: usize,
    pub x: i32,
    pub top: i32,
    pub baseline: i32,
    pub bottom: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PhysicalSnapshot {
    pub layout: LayoutSnapshot,
    pub rows: Vec<Arc<Vec<Row>>>,
    #[serde(skip)]
    pub(crate) owner: Arc<()>,
}

impl PhysicalSnapshot {
    /// Hit a row by page-local point and resolve the nearest scalar caret.
    pub fn hit_test(&self, page: usize, x: i32, y: i32) -> Option<(usize, u32)> {
        self.hit_position(page, x, y)
            .map(|p| (p.point.paragraph, p.point.offset))
    }
}

#[derive(Debug, Default)]
pub(crate) struct PhysicalCache {
    metrics: OnceLock<Result<Metrics, String>>,
    geometry: Option<PageGeometry>,
    paragraphs: Vec<CachedParagraph>,
    pages: Vec<ComposedPage>,
}

#[derive(Debug)]
struct CachedParagraph {
    lines: Weak<Vec<Line>>,
    style: FlowStyle,
}

#[derive(Clone, Copy)]
enum Styles<'a> {
    Uniform(FlowStyle),
    Explicit(&'a [FlowStyle]),
}

impl Styles<'_> {
    fn get(self, paragraph: usize) -> FlowStyle {
        match self {
            Self::Uniform(style) => style,
            Self::Explicit(styles) => styles[paragraph],
        }
    }

    fn validate(self) -> Result<(), String> {
        let styles = match &self {
            Self::Uniform(style) => std::slice::from_ref(style),
            Self::Explicit(styles) => styles,
        };
        if styles
            .iter()
            .any(|style| style.before < 0 || style.after < 0)
        {
            return Err("expected nonnegative paragraph spacing".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageGeometry {
    width: i32,
    left: i32,
    top: i32,
    limit: i32,
}

struct LineBox {
    left: i64,
    right: i64,
}

impl LineBox {
    fn width(&self) -> i64 {
        self.right - self.left
    }
}

impl PageGeometry {
    /// Endpoints relative to the page content area, before alignment.
    fn line_box(self, style: FlowStyle, first: bool) -> LineBox {
        LineBox {
            left: i64::from(style.left)
                + if first {
                    i64::from(style.first_line)
                } else {
                    0
                },
            right: i64::from(self.width) - i64::from(style.right),
        }
    }

    fn paragraph_widths(self, style: FlowStyle, paragraph: usize) -> Result<Widths, String> {
        let first = self.line_box(style, true);
        let rest = self.line_box(style, false);
        if first.left < 0 || style.left < 0 || style.right < 0 {
            return Err(format!(
                "paragraph {paragraph}: indents must stay within the page content area"
            ));
        }
        let positive_width = |line: LineBox, kind| {
            i32::try_from(line.width())
                .ok()
                .filter(|&width| width > 0)
                .ok_or_else(|| format!("paragraph {paragraph}: {kind} line has no usable width"))
        };
        Ok(Widths {
            first: positive_width(first, "first")?,
            rest: positive_width(rest, "continuation")?,
        })
    }
}

impl TryFrom<PageSpec> for PageGeometry {
    type Error = String;

    fn try_from(spec: PageSpec) -> Result<Self, Self::Error> {
        let width = i64::from(spec.width) - i64::from(spec.left) - i64::from(spec.right);
        let limit = i64::from(spec.height) - i64::from(spec.bottom);
        if [spec.left, spec.right, spec.top, spec.bottom]
            .iter()
            .any(|&value| value < 0)
            || width <= 0
            || limit <= i64::from(spec.top)
        {
            return Err("expected positive page content area and nonnegative margins".into());
        }
        // Nonnegative insets keep both values within the page's i32 dimensions.
        Ok(Self {
            width: width as i32,
            left: spec.left,
            top: spec.top,
            limit: limit as i32,
        })
    }
}

type Cursor = (usize, usize);

#[derive(Clone, Debug)]
struct ComposedPage {
    rows: Arc<Vec<Row>>,
    layout: Page,
    next: Option<Cursor>,
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    ascent: i64,
    descent: i64,
    gap: i64,
}

impl Metrics {
    fn from_font(font: &[u8], size: i32) -> Result<Self, String> {
        let face = rustybuzz::Face::from_slice(font, 0).ok_or("invalid font face at index 0")?;
        let em = i64::from(face.units_per_em());
        let scale = |units: i64| (units * i64::from(size) + em / 2) / em;
        let metrics = Self {
            ascent: scale(i64::from(face.ascender()).max(0)),
            descent: scale((-i64::from(face.descender())).max(0)),
            // Negative leading would overlap row hit boxes.
            gap: scale(i64::from(face.line_gap()).max(0)),
        };
        if !(1..=i64::from(i32::MAX)).contains(&(metrics.ascent + metrics.descent)) {
            return Err("font row height is zero or overflows layout coordinates".into());
        }
        Ok(metrics)
    }
}

impl LayoutSession {
    /// Lay out a single section with fixed page and paragraph geometry.
    ///
    /// Paragraph before-spacing stays with its first row, even at page top.
    /// Between paragraphs on the same page, the larger of before-spacing,
    /// after-spacing and font leading sets the gap. Rows may split a paragraph
    /// across pages.
    pub fn layout_pages(
        &mut self,
        paragraphs: &[String],
        spec: PageSpec,
    ) -> Result<PhysicalSnapshot, String> {
        self.layout_styled_pages(
            paragraphs,
            spec,
            Styles::Uniform(FlowStyle {
                before: spec.space_before,
                after: spec.space_after,
                ..FlowStyle::default()
            }),
        )
    }

    /// Lay out one absolute placement style per paragraph, ignoring the
    /// uniform spacing fields in `spec`.
    pub(crate) fn layout_flow_pages(
        &mut self,
        text: &[String],
        spec: PageSpec,
        styles: &[FlowStyle],
    ) -> Result<PhysicalSnapshot, String> {
        if styles.len() != text.len() {
            return Err("expected one flow style per paragraph".into());
        }
        self.layout_styled_pages(text, spec, Styles::Explicit(styles))
    }

    fn layout_styled_pages(
        &mut self,
        paragraphs: &[String],
        spec: PageSpec,
        styles: Styles<'_>,
    ) -> Result<PhysicalSnapshot, String> {
        let page = PageGeometry::try_from(spec)?;
        styles.validate()?;
        let metrics = self
            .physical
            .metrics
            .get_or_init(|| Metrics::from_font(&self.font, self.size))
            .clone()?;
        let (geometry, mut reuse) = self.paragraph_geometry_for(paragraphs, |index| {
            page.paragraph_widths(styles.get(index), index)
        })?;
        reuse.page_hits = self.physical.paginate(&geometry, styles, page, metrics)?;
        Ok(PhysicalSnapshot {
            owner: Arc::new(()),
            layout: LayoutSnapshot {
                paragraphs: geometry,
                pages: self
                    .physical
                    .pages
                    .iter()
                    .map(|p| p.layout.clone())
                    .collect(),
                reuse,
            },
            rows: self
                .physical
                .pages
                .iter()
                .map(|p| Arc::clone(&p.rows))
                .collect(),
        })
    }
}

impl PhysicalCache {
    fn paginate(
        &mut self,
        paragraphs: &[Arc<Vec<Line>>],
        styles: Styles<'_>,
        geometry: PageGeometry,
        metrics: Metrics,
    ) -> Result<usize, String> {
        let previous: &[_] = if self.geometry == Some(geometry) {
            &self.pages
        } else {
            &[]
        };
        let mut previous = previous.iter().peekable();
        let mut pages = Vec::new();
        let mut hits = 0;
        let mut next = next_line(paragraphs, (0, 0));
        while let Some(start) = next {
            while previous.peek().is_some_and(|p| p.start() < start) {
                previous.next();
            }
            if let Some(cached) = previous.next_if(|p| {
                p.start() == start
                    && p.matches(paragraphs, styles, &self.paragraphs, geometry, metrics)
            }) {
                next = cached.next;
                pages.push(cached.clone());
                hits += 1;
            } else {
                let page = compose_page(paragraphs, styles, geometry, metrics, start)?;
                next = page.next;
                pages.push(page);
            }
        }
        // Preserve the previous successful layout on errors. Weak identities
        // let the paragraph cache evict geometry independently of these rows.
        self.geometry = Some(geometry);
        self.paragraphs = paragraphs
            .iter()
            .enumerate()
            .map(|(index, lines)| CachedParagraph {
                lines: Arc::downgrade(lines),
                style: styles.get(index),
            })
            .collect();
        self.pages = pages;
        Ok(hits)
    }
}

impl ComposedPage {
    fn start(&self) -> Cursor {
        (self.rows[0].paragraph, self.rows[0].line)
    }

    fn matches(
        &self,
        current: &[Arc<Vec<Line>>],
        styles: Styles<'_>,
        old: &[CachedParagraph],
        page: PageGeometry,
        metrics: Metrics,
    ) -> bool {
        let last = self.rows.last().unwrap();
        let first = self.start().0;
        let range = first..=last.paragraph;
        let (Some(current_range), Some(old_range)) = (current.get(range.clone()), old.get(range))
        else {
            return false;
        };
        current_range
            .iter()
            .zip(old_range)
            .enumerate()
            .all(|(index, (lines, old))| {
                Arc::as_ptr(lines) == old.lines.as_ptr() && styles.get(first + index) == old.style
            })
            && self.next == next_line(current, (last.paragraph, last.line + 1))
            // Only the lookahead row's vertical fit can change this page.
            // A lookahead style edit that still forces a break is harmless.
            && self.next.is_none_or(|(paragraph, _)| {
                following_top(last, paragraph, styles, metrics.gap) + metrics.ascent + metrics.descent
                    > i64::from(page.limit)
            })
    }
}

fn next_line(paragraphs: &[Arc<Vec<Line>>], (mut paragraph, mut line): Cursor) -> Option<Cursor> {
    while let Some(lines) = paragraphs.get(paragraph) {
        if line < lines.len() {
            return Some((paragraph, line));
        }
        paragraph += 1;
        line = 0;
    }
    None
}

fn compose_page(
    paragraphs: &[Arc<Vec<Line>>],
    styles: Styles<'_>,
    page: PageGeometry,
    metrics: Metrics,
    start: Cursor,
) -> Result<ComposedPage, String> {
    let mut rows: Vec<Row> = Vec::new();
    let mut next = Some(start);
    while let Some((paragraph, line)) = next {
        let style = styles.get(paragraph);
        let top = if let Some(previous) = rows.last() {
            following_top(previous, paragraph, styles, metrics.gap)
        } else {
            i64::from(page.top)
                + if line == 0 {
                    i64::from(style.before)
                } else {
                    0
                }
        };
        let baseline = top + metrics.ascent;
        let bottom = baseline + metrics.descent;
        if bottom > i64::from(page.limit) {
            if rows.is_empty() {
                return Err(format!(
                    "paragraph {paragraph}, line {line}: row does not fit on a page"
                ));
            }
            break;
        }
        let width = i64::from(paragraphs[paragraph][line].width);
        let line_box = page.line_box(style, line == 0);
        let spare = (line_box.width() - width).max(0);
        let offset = match style.align {
            FlowAlign::Left => 0,
            FlowAlign::Center => spare / 2,
            FlowAlign::Right => spare,
        };
        let x = i64::from(page.left) + line_box.left + offset;
        let overflow =
            |_| format!("paragraph {paragraph}, line {line}: physical x coordinate overflows");
        i32::try_from(x + width).map_err(overflow)?;
        // Nonnegative margins and metrics bound these coordinates by limit.
        rows.push(Row {
            paragraph,
            line,
            x: i32::try_from(x).map_err(overflow)?,
            top: top as i32,
            baseline: baseline as i32,
            bottom: bottom as i32,
        });
        next = next_line(paragraphs, (paragraph, line + 1));
    }
    let lines = rows
        .iter()
        .map(|row| PlacedLine {
            paragraph: row.paragraph,
            line: row.line,
        })
        .collect();
    Ok(ComposedPage {
        rows: Arc::new(rows),
        layout: Page { lines },
        next,
    })
}

fn following_top(previous: &Row, paragraph: usize, styles: Styles<'_>, gap: i64) -> i64 {
    let gap = if previous.paragraph == paragraph {
        gap
    } else {
        gap.max(i64::from(styles.get(previous.paragraph).after))
            .max(i64::from(styles.get(paragraph).before))
    };
    i64::from(previous.bottom) + gap
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../tests/fonts/DejaVuSans-subset.ttf");
    const SIZE: i32 = 12 * 64;
    // The fixture's hhea ascent/descent (1901/-483 design units, 2048 UPEM).
    const ASCENT: i32 = 713;
    const DESCENT: i32 = 181;
    const ROW_HEIGHT: i32 = ASCENT + DESCENT;

    fn session() -> LayoutSession {
        LayoutSession::new(FONT.to_vec(), SIZE).unwrap()
    }

    fn spec(content_height: i32) -> PageSpec {
        PageSpec {
            width: 10_000,
            height: 20 + content_height + 30,
            left: 10,
            right: 15,
            top: 20,
            bottom: 30,
            space_before: 0,
            space_after: 0,
        }
    }

    fn input(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn exact_bottom_fits_and_one_unit_less_breaks_between_rows() {
        let mut engine = session();
        let text = input(&["A😀\nB"]);
        let page = PageSpec {
            space_before: 13,
            ..spec(2 * ROW_HEIGHT + 13)
        };
        let full = engine.layout_pages(&text, page).unwrap();
        assert_eq!(full.rows.len(), 1);
        assert_eq!(full.rows[0].len(), 2);
        let end = &full.rows[0][1];
        assert_eq!((end.paragraph, end.line, end.x), (0, 1, page.left));
        assert_eq!(end.top, page.top + page.space_before + ROW_HEIGHT);
        assert_eq!(end.baseline, end.top + ASCENT);
        assert_eq!(end.bottom, page.height - page.bottom);

        let split = engine
            .layout_pages(
                &text,
                PageSpec {
                    height: page.height - 1,
                    ..page
                },
            )
            .unwrap();
        assert_eq!(split.layout.reuse.page_hits, 0);
        assert_eq!(split.rows.len(), 2);
        assert_eq!(split.rows[1][0].top, page.top);
        assert_eq!(split.hit_test(1, page.left, page.top), Some((0, 3)));
    }

    #[test]
    fn overlaps_paragraph_spacing_and_translates_hits_without_hitting_gaps() {
        let mut engine = session();
        let mut page = spec(2 * ROW_HEIGHT + 18);
        page.space_before = 7;
        page.space_after = 11;
        let physical = engine.layout_pages(&input(&["A😀", ""]), page).unwrap();
        assert_eq!(physical.rows.len(), 1);
        let first = &physical.rows[0][0];
        let second = &physical.rows[0][1];
        assert_eq!(first.top, page.top + page.space_before);
        assert_eq!(second.top - first.bottom, page.space_after);
        assert_eq!(second.bottom, page.height - page.bottom);
        let width = physical.layout.paragraphs[0][0].width;
        assert_eq!(
            physical.hit_test(0, first.x + width, first.baseline),
            Some((0, 2))
        );
        assert_eq!(physical.layout.hit_row(0, 0, width), Some((0, 2)));
        assert_eq!(physical.hit_test(0, first.x + width + 1, first.top), None);
        assert_eq!(physical.hit_test(0, i32::MIN, first.top), None);
        assert_eq!(physical.hit_test(0, first.x, first.top - 1), None);
        assert_eq!(physical.hit_test(0, first.x, first.bottom), None);
        assert_eq!(physical.hit_test(0, second.x, second.top), Some((1, 0)));
        assert_eq!(physical.hit_test(0, second.x, second.bottom), None);
        assert_eq!(physical.hit_test(1, first.x, first.top), None);

        let split = PageSpec {
            height: page.height - 1,
            ..page
        };
        let physical = engine.layout_pages(&input(&["A😀", ""]), split).unwrap();
        assert_eq!(physical.rows.len(), 2);
        assert_eq!(physical.rows[1][0].top, split.top + split.space_before);
    }

    #[test]
    fn wraps_to_content_width_instead_of_full_page_width() {
        let mut engine = session();
        let page = spec(2 * ROW_HEIGHT);
        let prefix = engine.layout_pages(&input(&["a "]), page).unwrap();
        let width = prefix.layout.paragraphs[0][0].width;
        let narrow = PageSpec {
            left: 2 * width,
            width: 3 * width + page.right,
            ..page
        };
        let physical = engine.layout_pages(&input(&["a b"]), narrow).unwrap();
        assert_eq!(physical.rows[0].len(), 2);
        let first_width = physical.layout.paragraphs[0][0].width;
        assert_eq!(
            physical.hit_test(0, narrow.left + first_width, narrow.top),
            Some((0, 2))
        );
    }

    #[test]
    fn first_and_hanging_indents_change_wrapping_but_do_not_repeat_after_lf() {
        let mut engine = session();
        let text = input(&["a b\nc"]);
        let mut page = spec(ROW_HEIGHT);
        let unindented = engine.layout_pages(&text, page).unwrap();
        let first_hard_line = unindented.layout.paragraphs[0][0].width;
        let prefix_width = unindented.layout.paragraphs[0][0]
            .carets
            .iter()
            .find(|caret| caret.offset == 2)
            .unwrap()
            .x;
        let mut style = FlowStyle {
            left: 32,
            right: 16,
            first_line: first_hard_line - prefix_width + 3,
            align: FlowAlign::Center,
            ..FlowStyle::default()
        };
        page.width = page.left + style.left + first_hard_line + 4 + style.right + page.right;

        let positive = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        assert_eq!(positive.rows.len(), 3);
        let lines = &positive.layout.paragraphs[0];
        assert_eq!(lines[0].end, 2);
        assert_eq!(lines[2].start, 4);
        // The first row has a single unit of spare width, rounded down.
        assert_eq!(
            positive.rows[0][0].x,
            page.left + style.left + style.first_line
        );
        for (index, line) in lines.iter().enumerate().skip(1) {
            let centered = (first_hard_line + 4 - line.width) / 2;
            assert_eq!(positive.rows[index][0].x, page.left + style.left + centered);
        }
        let warm = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        assert_eq!(warm.layout.reuse.line_hits, 1);
        assert_eq!(warm.layout.reuse.page_hits, 3);
        assert!(Arc::ptr_eq(lines, &warm.layout.paragraphs[0]));
        assert!(Arc::ptr_eq(&positive.rows[0], &warm.rows[0]));

        style.align = FlowAlign::Left;
        style.first_line = -style.left;
        let hanging = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        assert_eq!(hanging.rows.len(), 2);
        assert_eq!(hanging.layout.paragraphs[0][0].end, 3);
        assert_eq!(hanging.rows[0][0].x, page.left);
        assert_eq!(hanging.rows[1][0].x, page.left + style.left);
        assert_eq!(hanging.layout.reuse.shaped_hits, 1);
        assert_eq!(hanging.layout.reuse.line_hits, 0);
        assert_eq!(hanging.layout.reuse.page_hits, 0);
        let cold = session().layout_flow_pages(&text, page, &[style]).unwrap();
        assert_eq!(hanging.rows, cold.rows);
        assert_eq!(hanging.layout.paragraphs, cold.layout.paragraphs);
    }

    #[test]
    fn right_aligned_rows_end_at_the_right_indent_and_overlong_rows_keep_their_start() {
        let mut engine = session();
        let mut page = spec(2 * ROW_HEIGHT);
        let text = input(&["a\nb"]);
        let style = FlowStyle {
            left: 30,
            right: 40,
            first_line: 10,
            align: FlowAlign::Right,
            ..FlowStyle::default()
        };
        let wide = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        for (row, line) in wide.rows[0].iter().zip(wide.layout.paragraphs[0].iter()) {
            assert_eq!(row.x + line.width, page.width - page.right - style.right);
        }

        page.width = page.left + style.left + style.first_line + 1 + style.right + page.right;
        let narrow = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        assert_eq!(
            narrow.rows[0][0].x,
            page.left + style.left + style.first_line
        );
        assert_eq!(narrow.rows[0][1].x, page.left + style.left);
    }

    #[test]
    fn rejects_out_of_content_indents_and_nonpositive_line_boxes() {
        let mut engine = session();
        let page = spec(ROW_HEIGHT);
        let usable = page.width - page.left - page.right;
        let text = input(&["a"]);
        for (left, right, first_line, reason) in [
            (0, 0, -1, "stay within"),
            (-1, 0, 2, "stay within"),
            (0, -1, 0, "stay within"),
            (0, 0, usable, "first line"),
            (1, usable - 1, -1, "continuation line"),
            (i32::MAX, i32::MAX, i32::MAX, "first line"),
        ] {
            let style = FlowStyle {
                left,
                right,
                first_line,
                ..FlowStyle::default()
            };
            let error = engine.layout_flow_pages(&text, page, &[style]).unwrap_err();
            assert!(error.contains(reason), "{error}");
        }
    }

    #[test]
    fn moving_a_paragraph_box_without_resizing_it_reuses_wrapping_not_placement() {
        let mut engine = session();
        let page = spec(ROW_HEIGHT);
        let text = input(&["a", "b"]);
        let mut styles = [
            FlowStyle::default(),
            FlowStyle {
                left: 20,
                right: 10,
                first_line: -5,
                ..FlowStyle::default()
            },
        ];
        let before = engine.layout_flow_pages(&text, page, &styles).unwrap();
        styles[1].left += 5;
        styles[1].right -= 5;
        let after = engine.layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(after.layout.reuse.line_hits, 2);
        assert_eq!(after.layout.reuse.page_hits, 1);
        assert!(Arc::ptr_eq(&before.rows[0], &after.rows[0]));
        assert!(Arc::ptr_eq(
            &before.layout.paragraphs[1],
            &after.layout.paragraphs[1],
        ));
        assert_eq!(before.rows[1][0].x, page.left + 15);
        assert_eq!(after.rows[1][0].x, page.left + 20);
    }

    #[test]
    fn leading_is_between_rows_not_in_the_hit_box_or_at_the_end_of_a_page() {
        let mut engine = session();
        let first = engine
            .layout_pages(&input(&["A\nB", "C"]), spec(3 * ROW_HEIGHT))
            .unwrap();
        let metrics = Metrics {
            ascent: ASCENT.into(),
            descent: DESCENT.into(),
            gap: 9,
        };
        let page = spec(3 * ROW_HEIGHT + 18);
        let styles = [
            FlowStyle {
                after: 5,
                ..FlowStyle::default()
            },
            FlowStyle {
                before: 3,
                ..FlowStyle::default()
            },
        ];
        let composed = compose_page(
            &first.layout.paragraphs,
            Styles::Explicit(&styles),
            PageGeometry::try_from(page).unwrap(),
            metrics,
            (0, 0),
        )
        .unwrap();
        assert_eq!(composed.next, None);
        assert_eq!(composed.rows[1].top - composed.rows[0].bottom, 9);
        assert_eq!(composed.rows[2].top - composed.rows[1].bottom, 9);
        assert_eq!(composed.rows[2].bottom, page.height - page.bottom);
    }

    #[test]
    fn alignment_changes_only_physical_placement_and_preserves_neighboring_pages() {
        let mut engine = session();
        let mut page = spec(ROW_HEIGHT);
        let text = input(&["a", "b", "c", "d"]);
        let uniform = engine.layout_pages(&text, page).unwrap();
        let center_width = uniform.layout.paragraphs[1][0].width;
        page.width = page.left + page.right + center_width + 301;
        let uniform = engine.layout_pages(&text, page).unwrap();

        // Explicit styles override both uniform spacings, including when zero.
        page.space_before = 50;
        page.space_after = 51;
        let mut styles = [FlowStyle::default(); 4];
        let before = engine.layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(before.layout.reuse.page_hits, 4);
        assert_eq!(before.rows, uniform.rows);

        styles[1].align = FlowAlign::Center;
        styles[2].align = FlowAlign::Right;
        let after = engine.layout_flow_pages(&text, page, &styles).unwrap();
        let cold = session().layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(after.layout.reuse.shaped_hits, 4);
        assert_eq!(after.layout.reuse.line_hits, 4);
        assert_eq!(after.layout.reuse.page_hits, 2);
        assert_eq!(after.rows, cold.rows);
        assert_eq!(after.layout.pages, cold.layout.pages);
        for (old, new) in before
            .layout
            .paragraphs
            .iter()
            .zip(&after.layout.paragraphs)
        {
            assert!(Arc::ptr_eq(old, new));
        }
        for index in [0, 3] {
            assert!(Arc::ptr_eq(&before.rows[index], &after.rows[index]));
        }
        assert_eq!(after.rows[1][0].x, page.left + 150);
        assert_eq!(
            after.rows[2][0].x + after.layout.paragraphs[2][0].width,
            page.width - page.right
        );
        assert_eq!(after.hit_test(1, page.left + 150, page.top), Some((1, 0)));
        assert_eq!(after.hit_test(1, page.left, page.top), None);
        assert_eq!(before.hit_test(1, page.left, page.top), Some((1, 0)));
    }

    #[test]
    fn spacing_lookahead_changes_the_break_only_when_the_row_can_fit() {
        let mut engine = session();
        let page = spec(2 * ROW_HEIGHT + 7);
        let text = input(&["a", "b", "c", "d"]);
        let mut styles = [FlowStyle::default(); 4];
        styles[1].before = 9;
        let before = engine.layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(
            before
                .rows
                .iter()
                .map(|rows| rows.len())
                .collect::<Vec<_>>(),
            [1, 1, 2]
        );
        assert_eq!(before.rows[1][0].top, page.top + 9);

        styles[1].before = 8;
        let still_breaks = engine.layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(still_breaks.layout.reuse.page_hits, 2);
        assert!(Arc::ptr_eq(&before.rows[0], &still_breaks.rows[0]));
        assert_eq!(still_breaks.rows[1][0].top, page.top + 8);

        styles[1].before = 7;
        let after = engine.layout_flow_pages(&text, page, &styles).unwrap();
        let cold = session().layout_flow_pages(&text, page, &styles).unwrap();
        assert_eq!(after.rows, cold.rows);
        assert_eq!(after.layout.pages, cold.layout.pages);
        assert_eq!(after.layout.reuse.page_hits, 1);
        assert_eq!(after.rows[0][1].bottom, page.height - page.bottom);
        assert_eq!(after.rows[0][1].top - after.rows[0][0].bottom, 7);
        assert!(Arc::ptr_eq(&before.rows[2], &after.rows[1]));
        assert_eq!(before.rows[0].len(), 1);
        assert_eq!(before.rows[1][0].top, page.top + 9);
        assert_eq!(before.hit_test(1, page.left, page.top + 9), Some((1, 0)));
        assert_eq!(after.hit_test(1, page.left, page.top + 9), Some((2, 0)));
    }

    #[test]
    fn explicit_styles_require_one_per_paragraph_and_keep_overlong_rows_at_the_margin() {
        let mut engine = session();
        let page = PageSpec {
            width: 11,
            right: 0,
            ..spec(ROW_HEIGHT)
        };
        let text = input(&["a"]);
        assert!(
            engine
                .layout_flow_pages(&text, page, &[])
                .unwrap_err()
                .contains("one flow style")
        );
        let style = FlowStyle {
            align: FlowAlign::Right,
            ..FlowStyle::default()
        };
        let fit = engine.layout_flow_pages(&text, page, &[style]).unwrap();
        assert!(fit.layout.paragraphs[0][0].width > 1);
        assert_eq!(fit.rows[0][0].x, page.left);
    }

    #[test]
    fn rejects_unusable_area_and_layout_overflow() {
        let mut engine = session();
        let text = input(&["abc"]);
        assert!(
            engine
                .layout_pages(&text, spec(ROW_HEIGHT - 1))
                .unwrap_err()
                .contains("row does not fit")
        );
        let page = PageSpec {
            space_before: i32::MAX,
            ..spec(ROW_HEIGHT)
        };
        assert!(
            engine
                .layout_pages(&text, page)
                .unwrap_err()
                .contains("row does not fit")
        );
        let page = PageSpec {
            width: i32::MAX,
            left: i32::MAX,
            right: i32::MAX,
            ..spec(ROW_HEIGHT)
        };
        assert!(
            engine
                .layout_pages(&text, page)
                .unwrap_err()
                .contains("positive page content area")
        );
        let page = PageSpec {
            width: i32::MAX,
            left: i32::MAX - 1,
            right: 0,
            ..spec(ROW_HEIGHT)
        };
        assert!(
            engine
                .layout_pages(&text, page)
                .unwrap_err()
                .contains("physical x coordinate overflows")
        );
    }

    #[test]
    fn reuses_both_placement_buffers_across_repeat_and_page_shift() {
        let mut engine = session();
        let page = spec(2 * ROW_HEIGHT);
        let text = input(&["a\nb", "c\nd", "e\nf"]);
        let before = engine.layout_pages(&text, page).unwrap();
        assert_eq!(before.layout.reuse.page_hits, 0);
        let repeated = engine.layout_pages(&text, page).unwrap();
        assert_eq!(repeated.layout.reuse.page_hits, 3);
        assert!(!Arc::ptr_eq(&before.owner, &repeated.owner));
        assert!(Arc::ptr_eq(&before.owner, &before.clone().owner));
        let encoded = serde_json::to_value(&before).unwrap();
        assert!(encoded.get("owner").is_none());

        let edited = input(&["a\nb", "c\nd\nX\nY", "e\nf"]);
        let after = engine.layout_pages(&edited, page).unwrap();
        let cold = session().layout_pages(&edited, page).unwrap();
        assert_eq!(after.layout.reuse.page_hits, 2);
        assert_eq!(after.rows, cold.rows);
        assert_eq!(after.layout.pages, cold.layout.pages);
        for (old, new) in [(0, 0), (2, 3)] {
            assert!(Arc::ptr_eq(&repeated.rows[old], &before.rows[old]));
            assert!(Arc::ptr_eq(&after.rows[new], &before.rows[old]));
            assert!(Arc::ptr_eq(
                &after.layout.pages[new].lines,
                &before.layout.pages[old].lines,
            ));
        }
        assert!(!Arc::ptr_eq(&after.rows[1], &before.rows[1]));
    }

    #[test]
    fn revalidates_old_document_end_when_input_is_appended() {
        let mut engine = session();
        let page = spec(2 * ROW_HEIGHT);
        let empty = engine.layout_pages(&[], page).unwrap();
        assert!(empty.rows.is_empty());
        let before = engine.layout_pages(&input(&["a"]), page).unwrap();
        let after = engine.layout_pages(&input(&["a", "b"]), page).unwrap();
        assert_eq!(after.layout.reuse.page_hits, 0);
        assert_eq!(before.rows[0].len(), 1);
        assert_eq!(after.rows[0].len(), 2);
    }
}
