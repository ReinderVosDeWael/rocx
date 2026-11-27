# Experimental browser layout

Checkout-only prototype, not an export of `rocx-docx`. Its Cargo workspace
builds the same editor Rust adapter plus layout exports into one web Wasm
module. The npm workspace and packed-source replay do not depend on it.

From the repository root, with `wasm32-unknown-unknown` and the matching
`wasm-bindgen-cli` 0.2.130 installed:

```sh
manifest=javascript/experimental-layout/Cargo.toml
scratch=$(mktemp -d /tmp/opencode/rocx-layout.XXXXXXXX)
trap 'rm -r -- "$scratch"' EXIT

cargo build --locked --release --manifest-path "$manifest" \
  --target wasm32-unknown-unknown --target-dir target
wasm-bindgen --target web --out-dir "$scratch/pkg" --out-name rocx \
  target/wasm32-unknown-unknown/release/rocx_js_experimental_layout.wasm
printf '{"type":"module"}\n' > "$scratch/package.json"

font=crates/rocx-layout/tests/fonts/DejaVuSans-subset.ttf
cargo run --locked --quiet --manifest-path "$manifest" --target-dir target \
  --example fixture -- "$font" > "$scratch/fixture.json"
node javascript/experimental-layout/test.mjs "$scratch/pkg/rocx.js" \
  "$font" "$scratch/fixture.json"

# Optional declaration check with TypeScript 5.9 or newer:
cp javascript/experimental-layout/types.mts "$scratch"
tsc --noEmit --strict --module nodenext --lib esnext,dom "$scratch/types.mts"
```

Serve `rocx.js`, `rocx_bg.wasm` and the supplied font over HTTP. The generated
`rocx.d.ts` declares the layout classes and page buffers. For example, from a
browser module or module worker:

```js
import init, { ExperimentalLayoutSession } from "./pkg/rocx.js";

await init();
const font = new Uint8Array(await (await fetch("./demo.ttf")).arrayBuffer());
const pt = (points) => points * 64;
const session = new ExperimentalLayoutSession(font, pt(12));
try {
  const snapshot = session.layoutPages(["First paragraph", ""], {
    width: pt(360), height: pt(480),
    left: pt(36), right: pt(36), top: pt(36), bottom: pt(36),
    spaceBefore: 0, spaceAfter: pt(6),
  });
  try {
    const firstPage = snapshot.page(0);
    if (firstPage?.rows.length) {
      const [, , x, top] = firstPage.rows;
      const hit = snapshot.hitTest(0, x, top);
      const caret = hit && snapshot.caretBox(hit, "downstream");
      console.log(firstPage, hit, caret, snapshot.reuse);
    }
  } finally {
    snapshot.free();
  }
} finally {
  session.free();
}
```

`layoutPages` uses font metrics for the height and baseline of each row. Add
each row's `x` and `baseline` to the glyph's line-relative `x` and `y` before
painting. `hitTest(page, x, y)` takes page-relative coordinates;
`caretBox(point, affinity)` resolves a scalar point to a row box, choosing
the upstream or downstream row at a shared wrap boundary.
`selectionRects(anchor, focus)` returns page-local rectangles in document
order for either selection direction. Coordinates use 1/64 pt; source
offsets count Unicode scalars, not UTF-16 code units or DOCX edit anchors.

`hitPosition(page, x, y)` also preserves the visual side of a soft wrap.
Pass its result to `moveCaret(position, direction)`, where `direction` is
`left`, `right`, `up`, `down`, `lineStart`, or `lineEnd`. Vertical moves
remember `preferredX` even across short rows. A hit or move with no
destination returns `undefined`. Positions carry a revision and snapshot ID;
`moveCaret` rejects positions from another snapshot instead of moving
them against different text. Only use positions within the Wasm instance
that produced them.

`plainSection(docxBytes)` returns `{ paragraphs, spec }` for use with
`layoutPages` on the supported single-section, single-column plain DOCX
profile. It rejects unsupported direct content rather than skipping it, but
does not resolve inherited styles or settings. Supply the font and size
when creating the session. The adapter produces layout input, not a
guarantee that a scalar range is editable.

Use `session.openPages(docxBytes)` instead when editing. The returned handle
owns the original source and has its own `revision` and `spec`. Its
`geometry()` method returns a physical snapshot for that same revision;
free both handles when finished, in either order. To insert at a page
point, call `opened.previewInsert(page, x, y, text)`. To edit a nonempty
scalar selection within one editable paragraph, call
`opened.previewReplace(anchor, focus, text)` with `{ paragraph, offset }`
endpoints; use `""` for deletion. Both methods return new DOCX bytes and
leave the original opening unchanged. They reject unpaired UTF-16
surrogates instead of replacing them. Choose a preview by reopening its
bytes for a fresh revision. A visible hit does not guarantee editability:
for example, text in a paragraph with a hard line break can be hit but not
edited by these methods.

`openPages` supports direct left, center, or right alignment and fixed
before/after paragraph spacing in the supported profile; `plainSection`
remains the plain-text projection.

The fixed-line grid remains available as `layout(paragraphs, width,
linesPerPage)` and `hitRow(page, row, x)`. Both kinds of snapshots can
outlive the session and share its revision sequence. `page(index)` copies
one page into owned, packed typed arrays: `lines`, `glyphs`, `carets` and,
for physical pages only, `rows`. Transfer them to a main thread if needed;
they remain valid after `free()`. No rendering backend is included.
