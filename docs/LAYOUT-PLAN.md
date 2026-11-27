# rocx-layout: from a DOCX paragraph to an editable page

**In development · [#95](https://github.com/ReinderVosDeWael/rocx/issues/95).**
`crates/rocx-layout` is a headless Rust workspace member. The native examples
and [browser worker demo](../javascript/experimental-layout/demo/README.md)
draw positioned outlines. The same physical row geometry drives the pointer,
caret, and selection queries.

The [kernel](../crates/rocx-layout/README.md) supports a fixed row grid and
physical pages. Physical layout uses a single section's explicit dimensions
and margins, one supplied font and size, and left-to-right text. Both paths
share shaping and wrapping caches. Geometry for physical pages also includes
baselines, half-open row boxes, source caret lookup with wrap affinity and
selection rectangles and visual caret navigation. Bound import handles
direct paragraph alignment, overlapping before/after spacing and horizontal
indents. First-line and continuation widths have separate cache keys from
the paragraph's page placement. The bound DOCX path retains the exact source behind
the displayed snapshot for insertion, replacement and deletion previews.

The prototype uses rustybuzz for glyph selection, advances and safe-break
flags, with unicode-segmentation for caret boundaries. It keeps design-unit
positions until materializing a line, so wrapping rounds relative to the
line's own origin. The pinned DejaVu subset exercises kerning, ligatures,
combining text and an astral glyph. In the native `reflow` example, 1,000
distinct paragraphs produced 63 pages. Single-character edits near either
end reused 999 shaped/wrapped paragraphs and 62 pages; both results matched
cold geometry. This measures the kernel only. The broader text-stack comparison
and end-to-end typing-latency benchmark remain open.

The same sample with `reflow FONT.ttf --physical` produces 67 pages using
font-derived row heights, margins and paragraph spacing. One-character edits
at either end reused 999 paragraph layouts and 66 physical page placements;
all row positions and paragraph geometry matched cold layout.

`docx_reflow` times a different operation: opening a DOCX, previewing a
source edit and encoding bytes, then reopening and laying out the selected
preview. A 1,000-paragraph `python-docx` sample reused 999 paragraph layouts
and 21/22 physical pages. The preview and warm reopen together took about
94 ms on one local run, versus less than 1 ms for kernel-only edits in the
synthetic sample. These are observations, not latency guarantees. A future
interactive edit loop should avoid encoding a package per keystroke.

## Where the boundary goes

| Owner | Contract |
| --- | --- |
| `rocx` | Own the immutable DOCX snapshot, typed inspection, edit eligibility, transactions and package preservation. |
| `rocx-layout` | Project supported content into flow order, resolve display properties, shape and wrap text, paginate, and return geometry with source positions. Depend on `rocx` in this direction only. |
| Editor or viewer | Supply fonts, paint positioned glyphs, convert screen coordinates to page coordinates, manage selection, input methods, undo and document versions; submit edits to `rocx`. |

Start from `ImportedDocument`, its ordered `BodyBlock`s and structured inlines,
not concatenated paragraph text: a break or an opaque object can disappear
in that projection. `src/editor/layout.rs` inspects document properties;
page and line geometry are a different product. Inventory the public section,
paragraph, run and style reads against the first fixtures. If a required value
is inaccessible, add the narrow typed read to `rocx` rather than parsing XML
again or depending on the MCP transport. An authored-document adapter can
follow when there is a consumer; it should feed the same flow algorithm.

## Incremental layout kernel

A long-lived `LayoutSession` owns bounded caches across document revisions;
each layout result is immutable. The kernel lays a block into an available
area and returns a fragment plus a continuation if the block did not fit.
Page layout repeatedly calls this operation. Keep glyph and line geometry
relative to its fragment. A page places a fragment by reference and offset.

| Cached product | Inputs determining reuse |
| --- | --- |
| Shaped paragraph | Text, resolved shaping styles and boundaries, direction, script/language, selected font bytes/face/size/features and shaping policy. Cache cluster offsets relative to the paragraph. |
| Line layout | Shaped paragraph, available width, paragraph metrics and wrapping policy. Store line breaks, finalized glyph positions, caret stops and heights. Finalize or reshape boundary-sensitive glyphs for the chosen breaks. |
| Page fragment | Remaining flow input consumed by the page (including any lookahead used to decide a break), incoming continuation, page constraints and pagination policy. Store placements and outgoing continuation. |

Compare canonical inputs for equality; a hash accelerates lookup. Keep
source anchors in a per-revision side table, never in shared geometry. Map
paragraph-local offsets to current anchors even for cache hits on duplicate
paragraphs. Use validated old-to-new block correspondence for checkpoints;
if correspondence is ambiguous, restart earlier. Reuse a whole suffix only
when the remaining flow inputs match and the incoming continuation matches
at a corresponding boundary. V1 continuation identifies the next paragraph
and line, active section geometry and pending paragraph spacing. Extend the
state when adding keep rules, repeated stories or footnotes.

Set a byte budget per session and evict least-recently-used cache entries.
Live snapshots retain referenced fragments independently. Count cold work,
hits, pages recomposed and retained bytes. A cold run uses the same kernel
with reuse disabled; it is the oracle for incremental results.

## Native and browser execution

Use one Rust kernel and session implementation, linked directly on desktop
and compiled into the editor's Wasm module for the web. Keep document and
layout handles in the same instance so a hit can target a live editor snapshot.
Rust can borrow page data; expose versioned session/snapshot handles and
owned, packed per-page buffers to JavaScript. Request visible pages in batches.
Include the layout revision in queries and responses so an older worker reply
cannot paint over a newer edit. Release obsolete snapshots explicitly.

The browser host may run the session in a dedicated worker without Wasm
threads or shared memory. Use the same font bytes, fallback order, shaping
policy and layout units on either host. Paint the engine's glyph placements;
Canvas `fillText` is not a positioned-glyph backend. Qualify a browser
painter on the first slice using font outlines or a glyph atlas. Keep JS
UTF-16 input/IME conversion at the boundary, separate from scalar source
offsets and visual caret stops. Run the click/edit/reflow fixture in an
actual browser worker and in native Rust; compare geometry and source queries.

## First supported profile

One section, one column, horizontal left-to-right body paragraphs, explicit
page size and margins, one supplied font face, a known font size, left alignment,
empty paragraphs, ordinary spaces, explicit line breaks and automatic page
breaks. Include mixed run boundaries within a word: a run is not a line-breaking
opportunity. Freeze default line height, paragraph spacing and overlong-word
behavior in the first fixture specification. Start with fixed-point physical
coordinates (for example 1/64 pt); convert twips at the adapter and apply zoom
only when painting. Font bytes and face index are inputs, not machine settings.

Return pages containing line bounds, baselines, positioned glyphs, font
identities and source spans. Report unsupported input with a source location.
For v1, refuse authoritative geometry for a document outside the profile:
silently dropping a table would give every subsequent paragraph a false
position. Distinguish missing fonts from unsupported document features.

## The editor contract

- A point query takes a page and local coordinates and returns a caret stop,
  or no hit outside text. A stop records the original paragraph anchor,
  Unicode scalar offset and upstream/downstream affinity (one logical boundary
  may appear at the end of a line and the start of the next).
- Grapheme boundaries govern user-visible stops; shaping clusters govern glyph
  placement. Neither glyph indices nor UTF-8 or JavaScript UTF-16 indices are
  `rocx` offsets. Carry an explicit mapping through shaping. A selection query
  returns rectangles per visual line, including for wrapped text.
- Geometry belongs to a particular input snapshot and font configuration.
  Reject stale or foreign positions. Only offer an edit for a source location
  accepted by the corresponding `rocx` operation: being drawable is not an
  edit permission. After an edit, open the resulting snapshot, lay it out
  again and obtain fresh anchors. Persistent identities are not a prerequisite.

## Delivery order and exit tests

1. **Qualify the text stack.** Compare a composed Rust shaper + Unicode break
   libraries with an integrated text stack such as Parley. Use one redistributable
   pinned font and the same inputs: ligature, combining mark, astral character,
   long word and a word split across runs. Choose the smallest approach that
   exposes cluster mappings and positioned glyphs, builds on Rust 1.88 and the
   repository's Wasm target, and works without system font discovery. Record
   measured results and license before adopting dependencies.
2. **Paginate and draw.** Generate a two-page fixture independently with
   `python-docx`; include a line that exactly fits and one that spills.
   Assert source coverage, expected break offsets, page count and baselines
   with pinned fonts. Use a debug painter that consumes the returned glyph
   positions without measuring text again. Verify native and Wasm geometry;
   keep a rendered sample for human inspection. Word or LibreOffice samples
   can reveal differences; they do not define portable pixel-exactness.
3. **Close the edit loop.** Hit both sides of a wrap, select across lines,
   and insert text in an eligible plain paragraph using `TextInsertion`
   (`src/editor/ranges.rs`). Reopen and verify the intended source text,
   changed wrapping and rejection of the old position. Test an empty paragraph,
   a combining sequence with no internal caret stop, and an unsupported table.
   Ship this slice before adding another document feature. At public API
   promotion, expose the same geometry, diagnostics and position semantics in
   Rust, Python and JavaScript and update bindings, catalogs and parity checks.
4. **Qualify incremental reflow.** Build the caches alongside their producers
   in steps 1–3. Compare warm and cold geometry and source queries after edits
   near the start and end, width and font changes, duplicate-block insertion,
   and forced eviction. Check reuse counters as well as output equality.
   Benchmark typing latency, pages recomposed and live/cache memory separately.

After the edit loop, prioritize the first user's documents: inherited styles
and font fallback, bidi and complex scripts, paragraph keep rules, lists,
tables with row fragmentation, and repeated headers/footers each need their
own fixtures. Decide between wider DOCX fidelity and lower typing latency
using those documents and the measurements from step 4.
