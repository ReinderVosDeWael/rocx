use rocx_layout::{Affinity::*, CaretMove::*, LayoutSession, PageSpec, TextPoint};

fn session() -> LayoutSession {
    LayoutSession::new(include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec(), 768).unwrap()
}
fn spec(width: i32) -> PageSpec {
    PageSpec {
        width: width + 20,
        height: 940,
        left: 10,
        right: 10,
        top: 10,
        bottom: 10,
        space_before: 0,
        space_after: 0,
    }
}
fn p(paragraph: usize, offset: u32) -> TextPoint {
    TextPoint { paragraph, offset }
}

#[test]
fn sticky_x_survives_an_empty_paragraph_and_page_boundary() {
    let paragraphs = vec!["abcdef".into(), "".into(), "abcdef".into()];
    let mut s = session();
    let snap = s.layout_pages(&paragraphs, spec(10_000)).unwrap();
    let first = snap.visual_position(p(0, 5), Upstream, None).unwrap();
    let expected = snap.caret_box(first.point, first.affinity).unwrap().x;
    let empty = snap.move_caret(&first, Down).unwrap();
    assert_eq!((empty.point, empty.preferred_x), (p(1, 0), Some(expected)));
    let last = snap.move_caret(&empty, Down).unwrap();
    assert_eq!(last.point, p(2, 5));
    assert!(snap.move_caret(&last, Down).is_none());
    let prev = snap.move_caret(&last, Up).unwrap();
    assert_eq!(snap.move_caret(&prev, Up).unwrap().point, first.point);
    let left = snap.move_caret(&last, Left).unwrap();
    assert_eq!((left.point, left.preferred_x), (p(2, 4), None));
    let other = s.layout_pages(&paragraphs, spec(10_000)).unwrap();
    assert!(other.move_caret(&first, Right).is_none());
}

#[test]
fn soft_wrap_has_two_reversible_visual_stops() {
    let mut s = session();
    let text = vec!["aaaa".into()];
    let big = s.layout_pages(&text, spec(10_000)).unwrap();
    let width = big.layout.paragraphs[0][0].carets[2].x;
    let snap = s.layout_pages(&text, spec(width)).unwrap();
    let leading = snap.hit_position(1, 10, 10).unwrap();
    assert_eq!((leading.point, leading.affinity), (p(0, 2), Downstream));
    let trailing = snap.move_caret(&leading, Left).unwrap();
    assert_eq!((trailing.point, trailing.affinity), (p(0, 2), Upstream));
    let other = snap.move_caret(&trailing, Right).unwrap();
    assert_eq!(
        (other.point, other.affinity),
        (leading.point, leading.affinity)
    );
    assert_eq!(snap.move_caret(&leading, LineEnd).unwrap().point, p(0, 4));
    assert_eq!(
        snap.move_caret(&trailing, LineStart).unwrap().point,
        p(0, 0)
    );
}

#[test]
fn moves_by_visible_stops_instead_of_bytes_or_code_units() {
    let snap = session()
        .layout_pages(&["e\u{301}😀a\nb".into()], spec(10_000))
        .unwrap();
    let mut pos = snap.visual_position(p(0, 0), Upstream, None).unwrap();
    assert!(snap.move_caret(&pos, Left).is_none());
    for offset in [2, 3, 4, 5, 6] {
        pos = snap.move_caret(&pos, Right).unwrap();
        assert_eq!(pos.point, p(0, offset));
    }
    assert!(snap.move_caret(&pos, Right).is_none());
    assert!(snap.visual_position(p(0, 1), Downstream, None).is_none());
}
