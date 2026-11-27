use rocx_layout::{Affinity, LayoutSession, PageSpec, TextPoint};

fn session() -> LayoutSession {
    LayoutSession::new(include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec(), 768).unwrap()
}

fn spec(width: i32, height: i32) -> PageSpec {
    PageSpec {
        width: width + 120,
        height: height + 220,
        left: 100,
        right: 20,
        top: 200,
        bottom: 20,
        space_before: 0,
        space_after: 0,
    }
}

fn p(paragraph: usize, offset: u32) -> TextPoint {
    TextPoint { paragraph, offset }
}

#[test]
fn one_source_boundary_has_a_caret_on_each_side_of_the_page_break() {
    let mut s = session();
    let text = vec!["aaaaa".to_string()];
    let big = s.layout_pages(&text, spec(20_000, 20_000)).unwrap();
    let width = big.layout.paragraphs[0][0].carets[2].x;
    let row = &big.rows[0][0];
    let snapshot = s
        .layout_pages(&text, spec(width, row.bottom - row.top))
        .unwrap();
    assert_eq!(snapshot.rows.len(), 3);
    let boundary = p(0, 2);
    let up = snapshot.caret_box(boundary, Affinity::Upstream).unwrap();
    let down = snapshot.caret_box(boundary, Affinity::Downstream).unwrap();
    assert_eq!((up.page, down.page), (0, 1));
    assert_eq!(down.x, 100);
    for caret in [up, down] {
        assert_eq!(
            snapshot.hit_test(caret.page, caret.x, caret.top),
            Some((0, 2))
        );
    }
    let forward = snapshot.selection_rects(p(0, 1), p(0, 5)).unwrap();
    assert_eq!(forward, snapshot.selection_rects(p(0, 5), p(0, 1)).unwrap());
    assert_eq!(
        forward.iter().map(|r| r.page).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(forward[0].right, up.x);
    assert_eq!(forward[1].left, down.x);
    assert!(snapshot.caret_box(p(0, 5), Affinity::Downstream).is_some());
}

#[test]
fn selection_excludes_breaks_and_rejects_an_interior_combining_offset() {
    let text = vec!["a\n\nb".into(), "e\u{301}".into()];
    let snap = session().layout_pages(&text, spec(20_000, 20_000)).unwrap();
    for (start, end) in [(p(0, 1), p(0, 3)), (p(0, 4), p(1, 0)), (p(0, 2), p(0, 2))] {
        assert_eq!(snap.selection_rects(start, end), Some(vec![]));
    }
    assert!(snap.selection_rects(p(0, 0), p(1, 1)).is_none());
    assert!(snap.caret_box(p(2, 0), Affinity::Downstream).is_none());
    assert!(snap.caret_box(p(0, 9), Affinity::Upstream).is_none());
    let rects = snap.selection_rects(p(0, 3), p(1, 2)).unwrap();
    assert_eq!(rects.len(), 2);
    assert!(rects.iter().all(|r| r.right > r.left && r.top < r.bottom));
}
