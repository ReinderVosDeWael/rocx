# rocx-layout (experimental)

This checkout-only crate lays out plain paragraphs on physical pages, either
from caller-supplied page metrics or a supported single-section DOCX.
A `LayoutSession` owns the font, size and reuse state; keep it alive
across edits. A snapshot owns the geometry for one layout. A host can still
paint the previous snapshot while preparing the next one.

```rust
use rocx_layout::{LayoutSession, Affinity, TextPoint};

fn insert_at_start(font_bytes: Vec<u8>, docx: &[u8]) -> Result<Vec<u8>, String> {
    let mut session = LayoutSession::new(font_bytes, 12 * 64)?;
    let pages = session.open_pages(docx)?;
    let caret = pages.geometry()
        .caret_box(TextPoint { paragraph: 0, offset: 0 }, Affinity::Downstream)
        .ok_or("no initial caret")?;
    pages.preview_insert(caret.page, caret.x, caret.top, "Hello ")
}
```

Widths, page coordinates and font size are in 1/64 pt. In the fixed-grid
`layout` interface, `lines_per_page` is a host-supplied
capacity, not a DOCX section height. A `PlacedLine` selects a paragraph and
line from the snapshot. It carries no editable document anchor. Glyph positions
are local to the line; use the supplied font to paint them. Hits outside the
line return no result. Row numbers start at zero.

## Physical pages and edits

`layout_pages(paragraphs, spec)` accepts a `PageSpec` with page dimensions,
four margins and uniform paragraph spacing. The available width determines
wrapping. Each physical row has a paragraph and line index, an x origin, and
a top, baseline and bottom. Font ascent and descent determine row height;
nonnegative font leading and inter-paragraph spacing consume space between
rows, outside the hit box. At a paragraph boundary the gap is the maximum
of font leading, the preceding after-spacing and the following before-spacing.
A row ending exactly at the bottom content edge fits.
Changing only page height or vertical margins reuses shaped and wrapped text.

`hit_test(page, x, y)` resolves a page-local point. `caret_box` maps a scalar
position back to its visual box, choosing the earlier or later line at a soft
wrap. `selection_rects` returns the visible spans in document order for a
forward or reversed selection. Empty lines and paragraph marks have no painted
width. Interior combining-sequence offsets are invalid caret stops.

`hit_position` retains the visual affinity of a click. Pass its result to
`move_caret` to move left or right by visual stop, up or down by row, or to
either edge of the row. Up/down retain a preferred page-local x even when
crossing a shorter line; a horizontal movement resets it. Positions belong
to the physical snapshot that issued them. Use `visual_position` to bind
known scalar coordinates to a new snapshot after an edit.

`plain_section` projects direct ordinary runs and explicit page dimensions
and margins, assuming a uniform caller-supplied font and size. It does not
resolve DOCX styles, themes, or effective document settings. Tables, multiple
sections, run formatting, numbering, and unsupported flow input are outside
the uniform projection. `open_pages` additionally accepts direct left, center
and right paragraph alignment, nonnegative before/after spacing, left/right
insets, and first-line or hanging indents in twips. Indents must leave positive
first-line and continuation widths and stay inside the horizontal content area.
Only the first visual line in a paragraph uses the first-line adjustment;
it does not restart at a new page or inline break.
An alignment change moves rows without changing glyphs or line breaks.
`open_pages` retains the source so `preview_insert` and
`preview_replace` submit edits to the exact document that supplied the hit.
The result is new DOCX bytes; reopen those bytes in the same session to
reuse geometry. A drawable row can still be ineligible for an edit. A range
replacement or deletion must stay within one eligible paragraph.

| Operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Open bound DOCX | `open_pages` | `open_pages` | `openPages` |
| Physical layout | `layout_pages` | `layout_pages` | `layoutPages` |
| Hit / caret / selection | `hit_test`, `caret_box`, `selection_rects` | same spellings | `hitTest`, `caretBox`, `selectionRects` |
| Visual navigation | `hit_position`, `move_caret` | same spellings | `hitPosition`, `moveCaret` |
| Source-preserving preview | `preview_insert`, `preview_replace` | same spellings | `previewInsert`, `previewReplace` |

The initial profile shapes Latin text, supported combining accents and selected
punctuation and emoji with face 0 of the supplied font. Unsupported characters
and missing glyphs are errors with paragraph and scalar locations. LF starts
a new line; empty lines have a caret. Spaces keep their advances. An overlong
word splits only at safe shaping boundaries, so an indivisible span may be
wider than the requested line. Glyph `y` increases downwards from the baseline.

Identical paragraphs share geometry even at different document positions.
The session retains up to four recently used width pairs per cached paragraph
(first visual line, continuation lines), subject to its byte budget. Changing
width can still hit the shaping cache. `LayoutSnapshot.reuse` counts hits for this
call, including duplicates within the call. The default paragraph budget
is approximately 16 MiB with at most 8,192 entries. Page checkpoints, font
bytes, the live input and snapshots are separate from that budget. Page
validation scans the document; a page hit reuses its placements.

| Cache operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Supply budget in bytes | `LayoutSession::with_cache_budget` | `LayoutSession(..., cache_budget=n)` | `ExperimentalLayoutSession.withCacheBudget` |
| Inspect occupancy | `session.cache_stats()` | `session.cache_stats` | `session.cacheStats` |
| Drop cached work | `session.clear_caches()` | `session.clear_caches()` | `session.clearCaches()` |

Clearing releases cached shapes, wraps and page checkpoints. Live snapshots
remain drawable and hittable. Cache byte counts are estimated charges, not
process memory measurements; native and Wasm allocation sizes can differ.

## Run the consumers

- **Interactive browser demo:** from the checkout run
  `python3 tools/serve_layout_demo.py`, then open `http://127.0.0.1:8000/`.
  Install the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli`
  0.2.130 first. See the [demo guide](../../javascript/experimental-layout/demo/README.md).
- **JavaScript:** see the [experimental Wasm consumer](../../javascript/experimental-layout/README.md)
  for build and parity-test commands. `ExperimentalLayoutSession.layout(...)`
  returns a snapshot with `page(index)`, `hitRow`, `reuse` and `revision`.
  Page buffers are transferable and can outlive the Wasm snapshot.
- **Rust:** `cargo test -p rocx-layout`. `cargo run -p rocx-layout --example
  pages -- FONT.ttf` writes complete page geometry for the two-paragraph
  parity sample. The edit-loop integration test uses a `python-docx` fixture,
  submits a hit as a `rocx` insertion and compares warm and cold reflow.
  To measure first layout, a one-character edit near the start and one near the
  end of 1,000 distinct paragraphs, run `cargo run --release -p rocx-layout
  --example reflow -- FONT.ttf`. It also checks both edits against cold results.
  Append `--physical` for physical page reflow; use `--example docx_reflow
  -- FONT.ttf INPUT.docx` to time opening, source-preserving preview, and
  reopening separately on an actual DOCX.
  To draw page two of the independent fixture from positioned font outlines:
  ```sh
  cargo run -q -p rocx-layout --example paint_page -- \
    crates/rocx-layout/tests/fonts/DejaVuSans-subset.ttf \
    crates/rocx-layout/tests/fixtures/physical-plain.docx 1 > page-two.svg
  ```
- **Python:** locally install this directory in a Python 3.12+ environment
  with `pip install ./crates/rocx-layout`, then run
  `python crates/rocx-layout/tests/python_consumer.py`. The checkout-only
  `rocx_layout.LayoutSession(font_bytes, size).layout(paragraphs, width, rows)`
  returns a snapshot with `page_count`, `page_json(index)`, `hit_row` and
  `reuse`. Run the consumer with `--native` and `python-docx` installed to
  compare geometry and edited package bytes to fresh Rust results.

See the [layout plan](../../docs/LAYOUT-PLAN.md) for qualification and later
input profiles.
