use rocx::editor::ImportedDocument;
use rocx_layout::{
    Affinity, CaretMove, LayoutSession, PageSpec, PhysicalSnapshot, TextPoint, VisualPosition,
};
use serde_json::{Value, json};

fn main() {
    let font_path = std::env::args_os().nth(1).expect("expected font path");
    let font = std::fs::read(font_path).expect("read font");
    let (font_size, width, lines_per_page) = (768, 2200, 2);
    let paragraphs = ["A😀e\u{301} office", "\nA\n", "fixed tail"].map(String::from);
    let edit = ["AB😀e\u{301} office", "\nA\n", "fixed tail"].map(String::from);
    let mut session = LayoutSession::new(font.clone(), font_size).expect("open layout session");
    let revisions: Vec<_> = [&paragraphs, &paragraphs, &edit]
        .into_iter()
        .map(|input| {
            let snapshot = session
                .layout(input, width, lines_per_page)
                .expect("layout");
            let hits: Vec<Value> = snapshot
                .pages
                .iter()
                .enumerate()
                .flat_map(|(page, placements)| {
                    let snapshot = &snapshot;
                    placements
                        .lines
                        .iter()
                        .enumerate()
                        .flat_map(move |(row, line)| {
                            let width = snapshot.paragraphs[line.paragraph][line.line].width;
                            [-1, 0, width / 2, width, width + 1]
                                .map(|x| json!([page, row, x, snapshot.hit_row(page, row, x)]))
                        })
                })
                .collect();
            json!({ "input": input, "snapshot": snapshot, "hits": hits })
        })
        .collect();
    let physical = physical_revisions(font.clone(), font_size, width, &paragraphs, &edit);
    let doc = ImportedDocument::open(include_bytes!(
        "../../../crates/rocx-layout/tests/fixtures/split-runs.docx"
    ))
    .unwrap();
    let (input, spec) = rocx_layout::plain_section(&doc).expect("plain section");
    let imported = LayoutSession::new(font.clone(), font_size)
        .unwrap()
        .layout_pages(&input, spec)
        .unwrap();
    let opened = opened_fixture(font.clone(), font_size);
    let indented = LayoutSession::new(font.clone(), font_size)
        .unwrap()
        .open_pages(include_bytes!(
            "../../../crates/rocx-layout/tests/fixtures/physical-indents.docx"
        ))
        .unwrap();
    let mut session = LayoutSession::new(font, font_size).unwrap();
    let formatted = session
        .open_pages(include_bytes!(
            "../../../crates/rocx-layout/tests/fixtures/physical-formatted.docx"
        ))
        .expect("open formatted fixture");
    println!(
        "{}",
        json!({
            "fontSize": font_size, "width": width, "linesPerPage": lines_per_page,
            "revisions": revisions, "physical": physical,
            "imported": { "paragraphs": input, "spec": page_spec(spec), "snapshot": physical_result(&imported) },
            "opened": opened,
            "formatted": {"spec": page_spec(formatted.spec()), "snapshot": physical_result(formatted.geometry())},
            "indented": {"spec": page_spec(indented.spec()), "snapshot":physical_result(indented.geometry())},
            "navigation": vertical_fixture(&mut session),
        })
    );
}

fn vertical_fixture(session: &mut LayoutSession) -> Value {
    let input = ["hello", "a", "hello"].map(String::from);
    let spec = PageSpec {
        width: 5000,
        height: 5000,
        left: 100,
        right: 100,
        top: 100,
        bottom: 100,
        space_before: 0,
        space_after: 0,
    };
    let snapshot = session.layout_pages(&input, spec).unwrap();
    let row = &snapshot.rows[0][0];
    let x = row.x + snapshot.layout.paragraphs[0][0].width;
    let navigation = navigation_path(
        &snapshot,
        0,
        x,
        row.top,
        &[
            ("down", CaretMove::Down),
            ("down", CaretMove::Down),
            ("up", CaretMove::Up),
            ("up", CaretMove::Up),
            ("lineStart", CaretMove::LineStart),
            ("right", CaretMove::Right),
            ("lineEnd", CaretMove::LineEnd),
            ("up", CaretMove::Up),
        ],
    );
    json!({"input": input, "spec": page_spec(spec), "path": navigation})
}

fn navigation_path(
    snapshot: &PhysicalSnapshot,
    page: usize,
    x: i32,
    y: i32,
    directions: &[(&str, CaretMove)],
) -> Value {
    let mut position = snapshot.hit_position(page, x, y);
    let initial = position.as_ref().map(position_json);
    let moves: Vec<_> = directions
        .iter()
        .map(|&(spelling, direction)| {
            position = position
                .as_ref()
                .and_then(|current| snapshot.move_caret(current, direction));
            json!([spelling, position.as_ref().map(position_json)])
        })
        .collect();
    json!({"at": [page, x, y], "initial": initial, "moves": moves})
}

fn position_json(position: &VisualPosition) -> Value {
    let affinity = match position.affinity {
        Affinity::Upstream => "upstream",
        Affinity::Downstream => "downstream",
    };
    let mut result = json!({"point": position.point, "affinity": affinity});
    if let Some(x) = position.preferred_x {
        result["preferredX"] = json!(x);
    }
    result
}

fn opened_fixture(font: Vec<u8>, font_size: i32) -> Value {
    let mut session = LayoutSession::new(font, font_size).unwrap();
    let opened = session
        .open_pages(include_bytes!(
            "../../../crates/rocx-layout/tests/fixtures/physical-plain.docx"
        ))
        .expect("open physical plain fixture");
    let geometry = opened.geometry();
    let text_point = |paragraph, offset| TextPoint { paragraph, offset };
    let insert_at = geometry
        .caret_box(text_point(3, 3), Affinity::Upstream)
        .unwrap();
    let hard_break = geometry
        .caret_box(text_point(2, 4), Affinity::Upstream)
        .unwrap();
    let (anchor, focus) = (text_point(3, 2), text_point(3, 1)); // Reverse selection of 😀.
    let insert_text = " hello 😀";
    let replace_text = "o";
    let insert = opened
        .preview_insert(insert_at.page, insert_at.x, insert_at.top, insert_text)
        .expect("native insert preview");
    let replace = opened
        .preview_replace(anchor, focus, replace_text)
        .expect("native replace preview");
    let delete = opened
        .preview_replace(anchor, focus, "")
        .expect("native deletion preview");
    for (bytes, expected) in [
        (&insert, "a😀a hello 😀"),
        (&replace, "aoa"),
        (&delete, "aa"),
    ] {
        let (actual, _) =
            rocx_layout::plain_section(&ImportedDocument::open(bytes).unwrap()).unwrap();
        assert_eq!(actual[3], expected);
    }
    let refused = opened
        .preview_insert(hard_break.page, hard_break.x, hard_break.top, "!")
        .expect_err("hard break must not become an editable text paragraph");
    let refused_replace = opened
        .preview_replace(text_point(2, 4), text_point(2, 5), "!")
        .expect_err("hard break must not accept range edits");
    json!({
        "spec": page_spec(opened.spec()),
        "snapshot": physical_result(geometry),
        "insert": {"at": insert_at, "text": insert_text, "bytes": insert},
        "replace": {"anchor": anchor, "focus": focus, "text": replace_text, "bytes": replace},
        "delete": delete,
        "refused": {"at": hard_break, "insertError": refused, "replaceError": refused_replace},
    })
}

fn physical_revisions(
    font: Vec<u8>,
    font_size: i32,
    width: i32,
    paragraphs: &[String],
    edit: &[String],
) -> Vec<Value> {
    let mut exact = PageSpec {
        width: width + 291,
        height: 20_000,
        left: 121,
        right: 170,
        top: 153,
        bottom: 220,
        space_before: 17,
        space_after: 29,
    };
    let mut probe = LayoutSession::new(font.clone(), font_size).unwrap();
    let roomy = probe.layout_pages(paragraphs, exact).unwrap();
    exact.height = roomy.rows[0][1].bottom + exact.bottom;
    let tight = PageSpec {
        bottom: exact.bottom + 1,
        ..exact
    };
    let mut session = LayoutSession::new(font, font_size).unwrap();
    [
        (paragraphs, exact),
        (paragraphs, exact),
        (paragraphs, tight),
        (paragraphs, tight),
        (edit, tight),
    ]
    .map(|(input, spec)| {
        let snapshot = session.layout_pages(input, spec).expect("physical layout");
        let row = &snapshot.rows[0][0];
        let width = snapshot.layout.paragraphs[row.paragraph][row.line].width;
        let navigation = navigation_path(
            &snapshot,
            0,
            row.x + width,
            row.top,
            &[("right", CaretMove::Right), ("left", CaretMove::Left)],
        );
        json!({
            "input": input, "spec": page_spec(spec), "snapshot": physical_result(&snapshot),
            "navigation": navigation,
        })
    })
    .into()
}

fn page_spec(spec: PageSpec) -> Value {
    json!({
        "width": spec.width, "height": spec.height, "left": spec.left, "right": spec.right,
        "top": spec.top, "bottom": spec.bottom, "spaceBefore": spec.space_before,
        "spaceAfter": spec.space_after,
    })
}

fn physical_result(snapshot: &PhysicalSnapshot) -> Value {
    let hits: Vec<_> = snapshot
        .rows
        .iter()
        .enumerate()
        .flat_map(|(page, rows)| {
            rows.iter().flat_map(move |row| {
                let width = snapshot.layout.paragraphs[row.paragraph][row.line].width;
                [
                    (row.x - 1, row.baseline),
                    (row.x, row.top - 1),
                    (row.x, row.top),
                    (row.x + width / 2, row.baseline),
                    (row.x + width, row.bottom - 1),
                    (row.x + width + 1, row.baseline),
                    (row.x, row.bottom),
                ]
                .map(|(x, y)| json!([page, x, y, snapshot.hit_test(page, x, y)]))
            })
        })
        .collect();
    let mut points = Vec::new();
    for (paragraph, lines) in snapshot.layout.paragraphs.iter().enumerate() {
        for line in lines.iter() {
            points.extend(line.carets.iter().map(|caret| TextPoint {
                paragraph,
                offset: caret.offset,
            }));
        }
    }
    let first = points[0];
    let last = *points.last().unwrap();
    let invalid = TextPoint {
        paragraph: snapshot.layout.paragraphs.len(),
        offset: 0,
    };
    points.push(invalid);
    let carets: Vec<_> = points
        .into_iter()
        .flat_map(|point| {
            [
                ("upstream", Affinity::Upstream),
                ("downstream", Affinity::Downstream),
            ]
            .map(|(name, affinity)| {
                json!([point_json(point), name, snapshot.caret_box(point, affinity)])
            })
        })
        .collect();
    let selections = [
        (first, last),
        (last, first),
        (first, first),
        (first, invalid),
    ]
    .map(|(anchor, focus)| {
        json!([
            point_json(anchor),
            point_json(focus),
            snapshot.selection_rects(anchor, focus)
        ])
    });
    json!({
        "layout": snapshot.layout, "rows": snapshot.rows, "hits": hits,
        "carets": carets, "selections": selections,
    })
}

fn point_json(point: TextPoint) -> Value {
    json!({"paragraph": point.paragraph, "offset": point.offset})
}
