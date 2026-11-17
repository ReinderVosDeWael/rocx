# Python bindings and CMI parity

The `rv-docx-rs` Python distribution exposes `rv_docx.declarative`. Its optional
PyO3 extension calls the existing Rust renderer. There is no runtime dependency
on CMI's package or renderer. `python-docx==1.2.0` supplies the enum types and
editable document returned by `to_docx()`, and reads the native output.

## Supported interface

All names in the pinned CMI declarative `__all__` are exported, including the
component base, document/template, sections/properties/header/footer, paragraphs,
text/images/tabs/breaks, tables/rows/cells/borders, and style definitions. Their
constructor parameter names, order, positional/keyword kinds, default values,
and dataclass fields are compared against the actual pinned CMI package.

Python callers use CMI dictionaries and tuples, rather than the Rust API's
record types and enums. python-docx alignment enum values are translated by the
extension. Conditions guard resolution before lazy or async fields run. Lists,
tuples, generators, dictionaries, awaitables, and components returned by async
functions are resolved recursively. Independent fields and siblings run
concurrently; shared components resolve once in a pass, and exceptions cancel
pending sibling work while retaining the original exception.

`await component` and `await component.resolve()` mutate resolved fields in place
and return the same object. Document rendering implicitly resolves the tree.
Successful resolved fields can be reused for another render. Coroutine values
are single-use; use lazy factories for hidden content that may later become
visible. Conditions are synchronous; async conditions raise `TypeError`.

`await Document.to_docx(template=None)` returns a `docx.document.Document` that
can be edited and saved using the standard python-docx API. Additional
conveniences are `await Document.to_bytes(template=None)`,
`await Document.save(path_or_stream, template=None)`, and `await document`.
The native renderer releases the GIL; OOXML rendering remains a synchronous CPU
step within the async document method, like the upstream packing step.

## Parity verification

```sh
uv sync --locked --group test
uv run --locked --group test pytest
```

The test-only CMI dependency is pinned to Git revision
`3449b942034129ad74a28aeb253361035806e385` (0.6.16). Production wheel metadata
requires only python-docx, not CMI, pytest, or an installed Rust toolchain.

- All 54 upstream declarative tests run against the bindings. Only their imports
  are redirected; a hash test verifies every original test file after reversing
  that change. The source hashes and revision are in
  `tests/python_bindings/upstream/ORIGIN.json`.
- API tests compare every export, constructor signature, defaults, and dataclass
  field order/keyword-only contract with the installed reference.
- Differential tests construct the same declarations using both implementations
  and compare actual text, runs/formatting, named styles, nested/merged tables,
  image payloads/sizes, comments, page geometry, headers/footers, and metadata.
  Templates created by CMI are edited by both engines at three insertion indices.
- Error comparisons cover unknown styles, invalid headings, duplicate table
  styles, mismatched column widths, missing images, and missing/invalid templates.
- Independent tests verify lazy/async resolution, recursion, shared components,
  cancellation, subclasses, file/stream output, and that the actual native Rust
  function is required and called.

These checks cover the declarative API, not CMI's separate imperative
`ExtendDocument`/`ExtendParagraph`/`ExtendTable` wrapper classes. The returned
python-docx document remains available for caller-side imperative editing.

## Recorded differences

The Rust API's deliberate differences are documented in `COMPATIBILITY.md`.
Python conditions run once per resolution pass, instead of the repeated calls in
the pinned packer. Dictionary values and components returned by coroutines are
fully resolved; cycles are rejected explicitly. The Python adapter preserves the
pinned packer's extra empty final section when the final declared section is
excluded, so the original upstream expectations pass. The Rust API retains its
behavior of omitting that trailing section.

The native engine implements some declared properties the pinned CMI renderer
ignores, such as columns, page numbering, title-page/break-type properties, and
image accessibility metadata. The Python `Document.numbering` dictionary is
accepted and stored without applying it, matching the pinned CMI implementation;
Rust's additional typed numbering definitions remain a Rust API feature.
Constructor annotations may refer to the bindings' own component classes; exact
annotation string representations, invalid dynamic assignments to arbitrary
attributes, and byte-identical ZIP/XML serialization are not parity guarantees.
Visual pagination still depends on the document reader and installed fonts.

## Build and provenance

```sh
uv build --wheel
```

maturin builds the optional PyO3 extension with the CPython 3.12 stable ABI.
The Python field declarations are adapted from the same pinned CMI source under
LGPL-2.1; the repository license is retained. Async resolution and document
wrapping are implemented here; rendering is the native Rust engine. Default Word
assets retain their separate python-docx MIT notice. Generated binaries, virtual
environments, caches, and `dist/` are excluded from Git.
