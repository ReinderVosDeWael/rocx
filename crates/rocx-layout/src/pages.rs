use std::sync::{Arc, Weak};

use crate::{Line, Page, PlacedLine};

#[derive(Debug, Default)]
pub(crate) struct PageCache {
    lines_per_page: usize,
    paragraphs: Vec<Weak<Vec<Line>>>,
    pages: Vec<Page>,
}

impl PageCache {
    pub(crate) fn paginate(
        &mut self,
        paragraphs: &[Arc<Vec<Line>>],
        lines_per_page: usize,
    ) -> Result<(Vec<Page>, usize), String> {
        if lines_per_page == 0 {
            return Err("expected positive lines per page".into());
        }

        let previous = if lines_per_page == self.lines_per_page {
            std::mem::take(&mut self.pages)
        } else {
            Vec::new()
        };
        let mut previous = previous.into_iter().peekable();
        let mut pages = Vec::new();
        let mut reused = 0;
        let mut end = PlacedLine {
            paragraph: 0,
            line: 0,
        };
        while let Some(start) = next_line(paragraphs, end) {
            let mut remaining = lines_per_page;
            end = start.clone();
            loop {
                let count = remaining.min(paragraphs[end.paragraph].len() - end.line);
                end.line += count;
                remaining -= count;
                if remaining == 0 {
                    break;
                }
                let Some(next) = next_line(paragraphs, end.clone()) else {
                    break;
                };
                end = next;
            }

            // Coordinates increase strictly. A page can move in the output,
            // but its cached placements must still address the same inputs.
            let start_key = coordinate(&start);
            while previous
                .peek()
                .is_some_and(|page| coordinate(&page.lines[0]) < start_key)
            {
                previous.next();
            }
            let count = lines_per_page - remaining;
            let range = start.paragraph..=end.paragraph;
            let current = &paragraphs[range.clone()];
            let cached = previous.next_if(|page| {
                coordinate(&page.lines[0]) == start_key
                    && page.lines.len() == count
                    && self.paragraphs.get(range).is_some_and(|old| {
                        // Weak keeps the allocation address reserved, not the
                        // geometry alive. A matching strong Arc proves liveness.
                        old.iter()
                            .zip(current)
                            .all(|(a, b)| a.as_ptr() == Arc::as_ptr(b))
                    })
            });
            if let Some(page) = cached {
                pages.push(page);
                reused += 1;
            } else {
                let mut lines = Vec::with_capacity(count);
                for (offset, input) in current.iter().enumerate() {
                    let paragraph = start.paragraph + offset;
                    let first = if offset == 0 { start.line } else { 0 };
                    let last = if paragraph == end.paragraph {
                        end.line
                    } else {
                        input.len()
                    };
                    lines.extend((first..last).map(|line| PlacedLine { paragraph, line }));
                }
                pages.push(Page {
                    lines: lines.into(),
                });
            }
        }

        // Keep only the previous document's identities and share its placements
        // with the returned snapshot without retaining any paragraph geometry.
        self.lines_per_page = lines_per_page;
        self.paragraphs = paragraphs.iter().map(Arc::downgrade).collect();
        self.pages = pages.clone();
        Ok((pages, reused))
    }
}

fn coordinate(line: &PlacedLine) -> (usize, usize) {
    (line.paragraph, line.line)
}

fn next_line(paragraphs: &[Arc<Vec<Line>>], mut line: PlacedLine) -> Option<PlacedLine> {
    while let Some(input) = paragraphs.get(line.paragraph) {
        if line.line < input.len() {
            return Some(line);
        }
        line.paragraph += 1;
        line.line = 0;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(count: u32) -> Arc<Vec<Line>> {
        Arc::new(
            (0..count)
                .map(|start| Line {
                    start,
                    end: start + 1,
                    width: 64,
                    glyphs: Vec::new(),
                    carets: Vec::new(),
                })
                .collect(),
        )
    }

    fn page(lines: &[(usize, usize)]) -> Page {
        Page {
            lines: lines
                .iter()
                .map(|&(paragraph, line)| PlacedLine { paragraph, line })
                .collect(),
        }
    }

    #[test]
    fn places_wraps_across_paragraphs_and_pages() {
        let mut cache = PageCache::default();
        let input = [paragraph(0), paragraph(3), paragraph(0), paragraph(2)];
        let expected = vec![
            page(&[(1, 0), (1, 1)]),
            page(&[(1, 2), (3, 0)]),
            page(&[(3, 1)]),
        ];
        assert_eq!(cache.paginate(&input, 2).unwrap(), (expected.clone(), 0));
        assert_eq!(cache.paginate(&input, 2).unwrap(), (expected, 3));
        assert_eq!(cache.paginate(&[], 2).unwrap(), (vec![], 0));
        assert_eq!(cache.paginate(&[paragraph(0)], 2).unwrap(), (vec![], 0));
    }

    #[test]
    fn reuses_unchanged_pages_after_edit_and_page_shift() {
        let mut cache = PageCache::default();
        let mut input = [paragraph(2), paragraph(2), paragraph(3)];
        let (before, _) = cache.paginate(&input, 2).unwrap();

        input[1] = paragraph(4);
        let (cold, _) = PageCache::default().paginate(&input, 2).unwrap();
        // First page is unchanged; the last two move by one page.
        let (after, reused) = cache.paginate(&input, 2).unwrap();
        assert_eq!((&after, reused), (&cold, 3));
        for (old, new) in [(0, 0), (2, 3), (3, 4)] {
            assert!(Arc::ptr_eq(&before[old].lines, &after[new].lines));
        }
        assert!(!Arc::ptr_eq(&before[1].lines, &after[1].lines));

        input[2] = paragraph(3);
        let (cold, _) = PageCache::default().paginate(&input, 2).unwrap();
        // Identical placements are not a hit if they reference changed input.
        assert_eq!(cache.paginate(&input, 2).unwrap(), (cold, 3));
    }

    #[test]
    fn duplicate_insertion_and_removal_keep_current_coordinates() {
        let mut cache = PageCache::default();
        let duplicate = paragraph(1);
        let tail = paragraph(3);
        let mut input = vec![duplicate.clone(), duplicate.clone(), tail];
        let (before, _) = cache.paginate(&input, 2).unwrap();

        input.insert(0, duplicate);
        let (cold, _) = PageCache::default().paginate(&input, 2).unwrap();
        assert_eq!(cold[1], page(&[(2, 0), (3, 0)]));
        assert_eq!(cache.paginate(&input, 2).unwrap(), (cold, 1));

        input.remove(1);
        assert_eq!(cache.paginate(&input, 2).unwrap(), (before, 1));
    }

    #[test]
    fn rejects_hits_when_an_intervening_paragraph_changes_line_count() {
        let mut cache = PageCache::default();
        let mut input = [paragraph(1), paragraph(1), paragraph(2)];
        cache.paginate(&input, 3).unwrap();

        input[1] = paragraph(0);
        assert_eq!(
            cache.paginate(&input, 3).unwrap(),
            (vec![page(&[(0, 0), (2, 0), (2, 1)])], 0)
        );
    }

    #[test]
    fn rejects_zero_capacity_and_invalidates_on_capacity_change() {
        let mut cache = PageCache::default();
        let input = [paragraph(2)];
        let (pages, _) = cache.paginate(&input, 2).unwrap();
        assert!(cache.paginate(&input, 0).is_err());
        assert_eq!(cache.paginate(&input, 2).unwrap(), (pages.clone(), 1));
        assert_eq!(cache.paginate(&input, usize::MAX).unwrap(), (pages, 0));
    }

    #[test]
    fn misses_when_previous_paragraph_geometry_expires() {
        let mut cache = PageCache::default();
        let input = paragraph(2);
        let weak = Arc::downgrade(&input);
        let (before, _) = cache.paginate(&[input], 2).unwrap();
        assert!(weak.upgrade().is_none());

        let (after, reused) = cache.paginate(&[paragraph(2)], 2).unwrap();
        assert_eq!((&after, reused), (&before, 0));
        assert!(!Arc::ptr_eq(&before[0].lines, &after[0].lines));
    }

    #[test]
    fn only_retains_the_previous_successful_document() {
        let mut cache = PageCache::default();
        let input = [paragraph(2)];
        let (pages, _) = cache.paginate(&input, 2).unwrap();
        let weak = Arc::downgrade(&pages[0].lines);
        drop(pages);
        assert!(weak.upgrade().is_some());

        cache.paginate(&[paragraph(1)], 2).unwrap();
        assert!(weak.upgrade().is_none());
    }
}
