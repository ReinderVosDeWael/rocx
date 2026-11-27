"""Run with an installed checkout build of rocx-layout (see the crate README)."""
import json
import argparse
import os
import subprocess
import tempfile
from pathlib import Path
from io import BytesIO
from zipfile import ZipFile

from rocx_layout import LayoutSession, PageSpec, plain_section

TESTS = Path(__file__).resolve().parent
FONT = TESTS / "fonts/DejaVuSans-subset.ttf"


def native_output(target, example, *args):
    return subprocess.check_output(
        ["cargo", "run", "--quiet", "--locked", "-p", "rocx-layout", "--example", example, "--", FONT, *args],
        cwd=TESTS.parents[2],
        env={**os.environ, "CARGO_TARGET_DIR": target},
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", action="store_true", help="Compare full native geometry and DOCX preview bytes")
    parser.add_argument("grid_json", nargs="?", type=Path, help="Compare a pre-generated fixed-grid result")
    args = parser.parse_args()
    if args.native:
        # Independent installed-consumer qualification. Different rustc
        # versions can otherwise reuse the root cdylib's unhashed rocx.rlib
        # across nested workspace builds (#96). One scratch for all oracles.
        preferred = Path("/tmp/opencode")
        with tempfile.TemporaryDirectory(prefix="rocx-layout-oracles.", dir=preferred if preferred.is_dir() else None) as target:
            check(args, target)
    else:
        check(args, None)


def check(args, target):
    session = LayoutSession(FONT.read_bytes(), 12 * 64)
    paragraphs = ["hello", "hello"]
    first = session.layout(paragraphs, 20_000, 1)
    assert first.page_count == 2
    assert first.reuse == (1, 1, 0)
    if args.native or args.grid_json:
        native = json.loads(native_output(target, "pages") if args.native else args.grid_json.read_bytes())
        assert [json.loads(first.page_json(i)) for i in range(first.page_count)] == native
    [(placement, line)] = json.loads(first.page_json(1))
    assert placement == {"paragraph": 1, "line": 0}
    assert line["glyphs"] and first.hit_row(1, 0, line["width"]) == (1, 5)
    second = session.layout(paragraphs, 20_000, 1)
    assert second.reuse == (2, 2, 2)
    edited = session.layout(["hello", "hello hello"], line["width"], 1)
    assert edited.page_count > 2 and first.page_count == 2
    assert first.hit_row(1, 0, -1) is None

    def spec(height, bottom):
        return PageSpec(width=20_000, height=height, left=100, top=200, bottom=bottom)

    initial = session.layout_pages(paragraphs, spec(20_000, 100))
    rows = json.loads(initial.page_json(0))
    height = rows[-1][0]["bottom"] + 100
    exact = session.layout_pages(paragraphs, spec(height, 100))
    assert exact.page_count == 1
    spill = session.layout_pages(paragraphs, spec(height, 101))
    assert spill.page_count == 2 and spill.reuse[:2] == (2, 2)
    [(row, line)] = json.loads(spill.page_json(1))
    assert row["top"] == 200 and row["x"] == 100
    assert spill.hit_test(1, 100 + line["width"], row["baseline"]) == (1, 5)
    end = spill.caret_box(1, 5, downstream=True)
    assert (end.page, end.row, end.x) == (1, 0, 100 + line["width"])
    assert [r.page for r in spill.selection_rects((1, 5), (0, 1))] == [0, 1]
    old_position = exact.visual_position(0, 1)
    assert spill.move_caret(old_position, "down") is None
    hit = spill.hit_position(0, 100, 200)
    assert (hit.paragraph, hit.offset) == (0, 0)
    moved = spill.move_caret(spill.move_caret(hit, "line_end"), "down")
    assert (moved.paragraph, moved.offset, moved.preferred_x) == (1, 5, 100 + line["width"])
    assert 0 < session.cache_stats.estimated_bytes <= session.cache_stats.budget_bytes
    session.clear_caches()
    assert session.cache_stats.estimated_bytes == 0 and session.cache_stats.wrap_variants == 0
    assert spill.hit_test(1, 100, row["baseline"]) == (1, 0)
    disabled = LayoutSession(FONT.read_bytes(), 768, cache_budget=0)
    disabled.layout(["hello"], 20_000, 2)
    assert disabled.cache_stats.paragraphs == 0
    if args.native:
        docx = TESTS / "fixtures/physical-plain.docx"
        source = docx.read_bytes()
        texts, metrics = plain_section(source)
        imported = session.layout_pages(texts, metrics)
        native = json.loads(native_output(target, "pages", docx))
        assert [json.loads(imported.page_json(i)) for i in range(imported.page_count)] == native
        bound = session.open_pages(source)
        frozen = bound.geometry
        a = frozen.caret_box(3, 2)
        inserted = bound.preview_insert(a.page, a.x, a.top, "!")
        assert plain_section(inserted)[0][-1] == "a😀!a"
        replaced = bound.preview_replace((3, 2), (3, 1), "X")
        assert plain_section(replaced)[0][-1] == "aXa"
        assert plain_section(bound.preview_replace((3, 1), (3, 2), ""))[0][-1] == "aa"
        edited = session.open_pages(replaced)
        assert edited.geometry.reuse[1] >= 3 and frozen.hit_test(a.page, a.x, a.top) == (3, 2)
        from docx import Document

        assert replaced == native_output(target, "edit_page", docx, "3", "2", "1", "X")
        assert [p.text for p in Document(BytesIO(replaced)).paragraphs] == ["hello", "", "one\ntwo", "aXa"]
        with ZipFile(BytesIO(source)) as before, ZipFile(BytesIO(replaced)) as after:
            assert set(before.namelist()) == set(after.namelist())
            assert [n for n in before.namelist() if before.read(n) != after.read(n)] == ["word/document.xml"]
        for fixture in ["physical-formatted.docx", "physical-indents.docx"]:
            path = TESTS / "fixtures" / fixture
            formatted = session.open_pages(path.read_bytes()).geometry
            pages = [json.loads(formatted.page_json(i)) for i in range(formatted.page_count)]
            assert pages == json.loads(native_output(target, "pages", path))
            assert len({row["x"] for page in pages for row, _ in page}) > 1


if __name__ == "__main__":
    main()
