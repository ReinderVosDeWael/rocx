use std::mem::size_of;

use rustybuzz::{Direction, Face, UnicodeBuffer};
use unicode_segmentation::UnicodeSegmentation;

use crate::{Caret, Glyph, Line, Widths};

#[derive(Debug)]
pub(crate) struct Shaped {
    hard_lines: Vec<HardLine>,
    scale: Scale,
}

impl Shaped {
    pub(crate) fn heap_bytes(&self) -> usize {
        self.hard_lines.capacity() * size_of::<HardLine>()
            + self
                .hard_lines
                .iter()
                .map(HardLine::heap_bytes)
                .sum::<usize>()
    }
}

#[derive(Debug)]
struct HardLine {
    glyphs: Vec<PositionedGlyph>,
    carets: Vec<PositionedCaret>,
    breaks: Vec<Break>,
}

impl HardLine {
    fn heap_bytes(&self) -> usize {
        self.glyphs.capacity() * size_of::<PositionedGlyph>()
            + self.carets.capacity() * size_of::<PositionedCaret>()
            + self.breaks.capacity() * size_of::<Break>()
    }
}

// Keep design-unit coordinates so rounding starts at the origin of each
// wrapped line, independent of where it started in the paragraph.
#[derive(Debug)]
struct PositionedGlyph {
    id: u32,
    pen: i64,
    x: i32,
    y: i32,
    advance: i32,
    cluster: u32,
}

#[derive(Debug)]
struct PositionedCaret {
    offset: u32,
    x: i64,
}

#[derive(Debug)]
struct Break {
    glyph: usize,
    caret: usize,
    preferred: bool,
}

#[derive(Debug)]
struct Scale {
    em: i32,
    size: i32,
}

impl Scale {
    fn to_layout(&self, units: i128) -> i128 {
        let em = i128::from(self.em);
        let scaled = units * i128::from(self.size);
        // Round nearest, ties away from zero.
        (scaled + scaled.signum() * (em / 2)) / em
    }
}

pub(crate) fn shape(text: &str, font: &[u8], size: i32) -> Result<Shaped, String> {
    if size <= 0 {
        return Err("font size must be positive".into());
    }
    let face = Face::from_slice(font, 0).ok_or("invalid font face at index 0")?;
    for (offset, ch) in text.chars().enumerate() {
        if !supported(ch) {
            return Err(format!(
                "unsupported text character U+{:04X} at scalar offset {offset}",
                u32::from(ch)
            ));
        }
    }

    // split preserves leading, consecutive, and trailing empty hard lines.
    let mut hard_lines = Vec::new();
    let mut start = 0;
    let mut parts = text.split('\n').peekable();
    while let Some(part) = parts.next() {
        let (line, end) = shape_hard_line(part, start, &face)?;
        hard_lines.push(line);
        if parts.peek().is_some() {
            start = next_scalar(end)?;
        }
    }
    Ok(Shaped {
        hard_lines,
        scale: Scale {
            em: face.units_per_em(),
            size,
        },
    })
}

fn supported(ch: char) -> bool {
    // Deliberately not a claim of general LTR support: Latin, selected common
    // punctuation, combining accents and standalone pictographs. In particular
    // exclude bidi controls, tabs, non-LF separators, SHY, CGJ, ZWJ and VS.
    matches!(ch,
        '\n' | ' '..='~' | '\u{00a0}'..='\u{00ac}' | '\u{00ae}'..='\u{024f}'
        | '\u{0300}'..='\u{034e}' | '\u{0350}'..='\u{036f}'
        | '\u{1e00}'..='\u{1eff}' | '\u{2010}'..='\u{2015}'
        | '\u{2018}'..='\u{201f}' | '\u{2022}' | '\u{2026}'
        | '\u{1f600}'..='\u{1f64f}'
    )
}

fn next_scalar(offset: u32) -> Result<u32, String> {
    offset.checked_add(1).ok_or("too many text scalars".into())
}

struct Grapheme {
    start: u32,
    space: bool,
    no_break: bool,
}

fn shape_hard_line(text: &str, start: u32, face: &Face<'_>) -> Result<(HardLine, u32), String> {
    let mut buffer = UnicodeBuffer::new();
    let mut graphemes = Vec::new();
    let mut end = start;
    for grapheme in text.graphemes(true) {
        graphemes.push(Grapheme {
            start: end,
            space: grapheme == " ",
            no_break: grapheme.starts_with(['\u{00a0}', '\u{2011}']),
        });
        let cluster = end;
        for ch in grapheme.chars() {
            buffer.add(ch, cluster);
            end = next_scalar(end)?;
        }
    }

    let mut line = HardLine {
        glyphs: Vec::new(),
        carets: Vec::new(),
        breaks: vec![Break {
            glyph: 0,
            caret: 0,
            preferred: false,
        }],
    };
    if text.is_empty() {
        line.carets.push(PositionedCaret {
            offset: start,
            x: 0,
        });
        return Ok((line, end));
    }
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_script(rustybuzz::script::LATIN);
    let output = rustybuzz::shape(face, &[], buffer);
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    if infos.first().map(|info| info.cluster) != Some(start) {
        return Err(format!("invalid shaping clusters at scalar offset {start}"));
    }

    let mut pen = 0i64;
    let mut remaining_graphemes = graphemes.as_slice();
    let mut previous: Option<&Grapheme> = None;
    let groups = infos.chunk_by(|a, b| a.cluster == b.cluster);
    for group in groups {
        let offset = group[0].cluster;
        let next_glyph = line.glyphs.len() + group.len();
        let limit = infos.get(next_glyph).map_or(end, |info| info.cluster);
        let count = remaining_graphemes
            .iter()
            .take_while(|g| g.start < limit)
            .count();
        if count == 0
            || remaining_graphemes[0].start != offset
            || (limit != end && remaining_graphemes.get(count).map(|g| g.start) != Some(limit))
        {
            return Err(format!(
                "invalid shaping clusters at scalar offset {offset}"
            ));
        }
        let (cluster_graphemes, rest) = remaining_graphemes.split_at(count);
        remaining_graphemes = rest;
        if let Some(previous) = previous
            && !previous.no_break
            && !cluster_graphemes[0].no_break
            && group.iter().all(|info| !info.unsafe_to_break())
        {
            line.breaks.push(Break {
                glyph: line.glyphs.len(),
                caret: line.carets.len(),
                preferred: previous.space,
            });
        }
        let cluster_x = pen;
        for (info, pos) in group.iter().zip(&positions[line.glyphs.len()..next_glyph]) {
            if info.glyph_id == 0 {
                return Err(format!("missing font glyph at scalar offset {offset}"));
            }
            if pos.x_advance < 0 || pos.y_advance != 0 {
                return Err(format!(
                    "unsupported glyph advance at scalar offset {offset}"
                ));
            }
            line.glyphs.push(PositionedGlyph {
                id: info.glyph_id,
                pen,
                x: pos.x_offset,
                y: pos.y_offset,
                advance: pos.x_advance,
                cluster: offset,
            });
            pen = pen
                .checked_add(i64::from(pos.x_advance))
                .ok_or("shaping coordinates overflow")?;
        }
        for (i, grapheme) in cluster_graphemes.iter().enumerate() {
            // Interpolate only between graphemes in a shaping cluster (e.g.
            // an ffi ligature), never between scalars of one grapheme.
            let advance = i128::from(pen - cluster_x) * i as i128 / count as i128;
            line.carets.push(PositionedCaret {
                offset: grapheme.start,
                x: cluster_x + advance as i64,
            });
        }
        previous = cluster_graphemes.last();
    }
    line.breaks.push(Break {
        glyph: line.glyphs.len(),
        caret: line.carets.len(),
        preferred: true,
    });
    line.carets.push(PositionedCaret {
        offset: end,
        x: pen,
    });
    Ok((line, end))
}

pub(crate) fn wrap_widths(shaped: &Shaped, widths: Widths) -> Result<Vec<Line>, String> {
    if widths.first <= 0 || widths.rest <= 0 {
        return Err("wrap width must be positive".into());
    }
    let mut lines = Vec::new();
    for line in &shaped.hard_lines {
        let mut start = 0;
        loop {
            let from = &line.breaks[start];
            let remaining = &line.breaks[start + 1..];
            if remaining.is_empty() {
                lines.push(materialize(line, from, from, &shaped.scale)?);
                break;
            }

            // Prefer ordinary space breaks, with spaces retained in geometry.
            // For an overlong word, fill to a safe cluster boundary; when
            // nothing fits, overflow the smallest indivisible nonempty span.
            // The first width belongs to the first visual row, even if empty.
            let width = if lines.is_empty() {
                widths.first
            } else {
                widths.rest
            };
            let origin = line.carets[from.caret].x;
            let fitting = remaining.partition_point(|b| {
                shaped
                    .scale
                    .to_layout(i128::from(line.carets[b.caret].x - origin))
                    <= i128::from(width)
            });
            let relative_end = remaining[..fitting]
                .iter()
                .rposition(|b| b.preferred)
                .unwrap_or_else(|| fitting.saturating_sub(1));
            lines.push(materialize(
                line,
                from,
                &remaining[relative_end],
                &shaped.scale,
            )?);
            start += relative_end + 1;
            if start + 1 == line.breaks.len() {
                break;
            }
        }
    }
    Ok(lines)
}

fn coordinate(value: i128) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| "line coordinates overflow".into())
}

fn materialize(line: &HardLine, start: &Break, end: &Break, scale: &Scale) -> Result<Line, String> {
    let origin = &line.carets[start.caret];
    let limit = &line.carets[end.caret];
    let glyphs = line.glyphs[start.glyph..end.glyph]
        .iter()
        .map(|glyph| {
            let pen = i128::from(glyph.pen - origin.x);
            let advance = scale.to_layout(pen + i128::from(glyph.advance)) - scale.to_layout(pen);
            Ok(Glyph {
                id: glyph.id,
                x: coordinate(scale.to_layout(pen + i128::from(glyph.x)))?,
                // rustybuzz uses y-up; line coordinates use y-down from baseline.
                y: coordinate(scale.to_layout(-i128::from(glyph.y)))?,
                advance: coordinate(advance)?,
                cluster: glyph.cluster,
            })
        })
        .collect::<Result<_, String>>()?;
    let carets = line.carets[start.caret..=end.caret]
        .iter()
        .map(|caret| {
            Ok(Caret {
                offset: caret.offset,
                x: coordinate(scale.to_layout(i128::from(caret.x - origin.x)))?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Line {
        start: origin.offset,
        end: limit.offset,
        width: coordinate(scale.to_layout(i128::from(limit.x - origin.x)))?,
        glyphs,
        carets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../tests/fonts/DejaVuSans-subset.ttf");
    const SIZE: i32 = 12 * 64;

    fn wrap(shaped: &Shaped, width: i32) -> Result<Vec<Line>, String> {
        wrap_widths(
            shaped,
            Widths {
                first: width,
                rest: width,
            },
        )
    }

    fn layout(text: &str, width: i32) -> Vec<Line> {
        wrap(&shape(text, FONT, SIZE).unwrap(), width).unwrap()
    }

    #[test]
    fn ligature_mark_and_astral_keep_scalar_carets() {
        let lines = layout("ffi x\u{0323}😀", i32::MAX);
        let line = &lines[0];
        let offsets: Vec<_> = line.carets.iter().map(|c| c.offset).collect();
        let clusters: Vec<_> = line.glyphs.iter().map(|g| g.cluster).collect();
        assert_eq!((line.start, line.end), (0, 7));
        assert_eq!(offsets, [0, 1, 2, 3, 4, 6, 7]);
        assert_eq!(clusters, [0, 3, 4, 4, 6]);
        assert_eq!(line.glyphs[3].advance, 0);
        assert_ne!(line.glyphs[3].x, line.carets[5].x);
        assert!(line.carets.windows(2).all(|pair| pair[0].x < pair[1].x));
        assert_eq!(line.carets.last().unwrap().x, line.width);
        assert_eq!(
            line.glyphs.iter().map(|g| g.advance).sum::<i32>(),
            line.width
        );
    }

    #[test]
    fn exact_width_prefers_spaces_and_rebases_to_local_geometry() {
        let exact = layout("one two ", i32::MAX)[0].width;
        let lines = layout("one two three", exact);
        assert_eq!(lines.len(), 2);
        assert_eq!(
            (lines[0].start, lines[0].end, lines[0].width),
            (0, 8, exact)
        );
        assert_eq!((lines[1].start, lines[1].end), (8, 13));
        assert_eq!(lines[1].carets[0], Caret { offset: 8, x: 0 });
        assert_eq!(layout("one two three", exact - 1)[0].end, 4);

        let mut expected = layout("three", i32::MAX).remove(0);
        expected.start += 8;
        expected.end += 8;
        for glyph in &mut expected.glyphs {
            glyph.cluster += 8;
        }
        for caret in &mut expected.carets {
            caret.offset += 8;
        }
        assert_eq!(lines[1], expected);
    }

    #[test]
    fn first_line_indent_fits_at_equality_and_spills_one_unit_below() {
        let two = layout("WW", i32::MAX)[0].width;
        let three = layout("WWW", i32::MAX)[0].width;
        let shaped = shape("WWWWW", FONT, SIZE).unwrap();
        let exact = wrap_widths(
            &shaped,
            Widths {
                first: two,
                rest: three,
            },
        )
        .unwrap();
        assert_eq!(exact.len(), 2);
        assert_eq!((exact[0].start, exact[0].end, exact[0].width), (0, 2, two));
        assert_eq!(
            (exact[1].start, exact[1].end, exact[1].width),
            (2, 5, three)
        );

        for (first, rest, expected_ends) in
            [(two - 1, three, [1, 4, 5]), (two, three - 1, [2, 4, 5])]
        {
            let lines = wrap_widths(&shaped, Widths { first, rest }).unwrap();
            assert_eq!(
                lines.iter().map(|line| line.end).collect::<Vec<_>>(),
                expected_ends
            );
        }
    }

    #[test]
    fn hanging_indent_keeps_rest_width_after_soft_and_hard_breaks() {
        let one = layout("W", i32::MAX)[0].width;
        let two = layout("WW", i32::MAX)[0].width;
        let shaped = shape("WWW\nWW", FONT, SIZE).unwrap();
        let lines = wrap_widths(
            &shaped,
            Widths {
                first: two,
                rest: one,
            },
        )
        .unwrap();
        let spans: Vec<_> = lines.iter().map(|l| (l.start, l.end, l.width)).collect();
        assert_eq!(spans, [(0, 2, two), (2, 3, one), (4, 5, one), (5, 6, one)]);
    }

    #[test]
    fn first_empty_hard_line_consumes_first_width() {
        let one = layout("W", i32::MAX)[0].width;
        let shaped = shape("\nWW", FONT, SIZE).unwrap();
        let lines = wrap_widths(
            &shaped,
            Widths {
                first: i32::MAX,
                rest: one,
            },
        )
        .unwrap();
        let spans: Vec<_> = lines.iter().map(|l| (l.start, l.end, l.width)).collect();
        assert_eq!(spans, [(0, 0, 0), (1, 2, one), (2, 3, one)]);
    }

    #[test]
    fn overlong_word_makes_progress_without_unsafe_splits() {
        let lines = layout("ffiWWW", 1);
        let ranges: Vec<_> = lines.iter().map(|line| (line.start, line.end)).collect();
        assert_eq!(ranges, [(0, 3), (3, 4), (4, 5), (5, 6)]);
        assert_eq!(lines[0].glyphs.len(), 1);
        assert!(lines.iter().all(|line| line.width > 1));
        let kerned = layout("AV", i32::MAX);
        assert!(kerned[0].width < layout("A", i32::MAX)[0].width + layout("V", i32::MAX)[0].width);
        assert_eq!(layout("AV", 1), kerned);
        assert_eq!(layout("W\u{00a0}W", 1).len(), 1);
        assert_eq!(layout("W\u{2011}W", 1), layout("W\u{2011}W", i32::MAX));
    }

    #[test]
    fn empty_hard_lines_still_have_carets() {
        let lines = layout("\né\n\n", i32::MAX);
        let ranges: Vec<_> = lines.iter().map(|line| (line.start, line.end)).collect();
        assert_eq!(ranges, [(0, 0), (1, 2), (3, 3), (4, 4)]);
        for i in [0, 2, 3] {
            let line = &lines[i];
            assert_eq!(line.width, 0);
            assert!(line.glyphs.is_empty());
            assert_eq!(
                line.carets,
                [Caret {
                    offset: line.start,
                    x: 0
                }]
            );
        }
        assert_eq!(layout("", 1), &lines[..1]);
    }

    #[test]
    fn rejects_invalid_or_unsupported_inputs() {
        assert!(shape("", b"", SIZE).unwrap_err().contains("invalid font"));
        assert!(shape("", FONT, 0).unwrap_err().contains("positive"));
        for unsupported in ['א', '\t', '\r', '\u{202e}', '\u{200d}', '\u{fe0f}'] {
            let error = shape(&format!("é{unsupported}"), FONT, SIZE).unwrap_err();
            assert!(error.contains("unsupported text character"));
            assert!(error.contains("scalar offset 1"));
        }
        assert!(
            shape("🙂", FONT, SIZE)
                .unwrap_err()
                .contains("missing font glyph at scalar offset 0")
        );
        assert!(wrap(&shape("A", FONT, SIZE).unwrap(), 0).is_err());
        let empty = shape("", FONT, SIZE).unwrap();
        for (first, rest) in [(0, 1), (1, -1)] {
            assert!(wrap_widths(&empty, Widths { first, rest }).is_err());
        }
        assert!(
            wrap(&shape("W\u{00a0}W", FONT, i32::MAX).unwrap(), 1)
                .unwrap_err()
                .contains("coordinates overflow")
        );
    }

    #[test]
    fn heap_estimate_includes_hard_lines_and_nested_geometry() {
        let empty = shape("", FONT, SIZE).unwrap().heap_bytes();
        assert!(empty > 0);
        assert!(shape("\n\n", FONT, SIZE).unwrap().heap_bytes() > empty);
        assert!(shape("one two three", FONT, SIZE).unwrap().heap_bytes() > empty);
    }
}
