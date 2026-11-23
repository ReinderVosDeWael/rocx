# Viewer compatibility checklist

This is a repeatable manual inspection protocol, not a compatibility guarantee.
Record results only for the exact file, application, version and platform
actually inspected. Opening a document successfully does not establish correct
layout. Headless conversion, schema validation and python-docx readback are not
GUI observations.

## Fixture set

Generate the rocx quickstart (two paragraphs, including Unicode):

```sh
cargo run --locked --example plain -- /tmp/opencode/viewer-quickstart.docx
```

Generate a compact, synthetic **python-docx-authored** sample with a labeled
table, image and bullet list using the existing repository image:

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/generate_viewer_compatibility_fixture.py /tmp/opencode/viewer-compact.docx
```

This independent sample is neither rocx-authored nor Word-authored. Inspect
the red image, four labeled table cells and both list items. Its output can
vary between runs; hash the actual file under test.

From the repository root, generate the paired catalogs and edit artifacts using
the existing checks (requires the provisioned development environment and a
current native Python extension):

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/check_examples.py --output-dir /tmp/opencode/viewer-compatibility-7
```

See [catalog generation and inspection guide](../examples/ALL_FEATURES.md) for
artifact names, visible labels, intended features and viewer caveats. The script
checks exact Rust/Python DOCX bytes, not merely normalized XML. Retain originals
outside the output directory before regenerating; record each file's SHA-256,
for example `sha256sum /path/to/file.docx`. Test the final catalog and its edited
counterpart separately: the edited catalog exercises styles and headers as well
as an imported body edit, but is not an edit of a Word-authored template.
The generated header-source, header-shared and
header-local files provide a focused shared-header comparison.

The repository's [imported editor fixture](../tests/fixtures/README.md#python-docx-editor-fixture)
is an independent python-docx-authored input. It is **not** Word-authored. The
catalogs are rocx-authored, including when the editor modifies a catalog.
Neither demonstrates interoperability with a desktop Word-authored source.
If a redistributable Word-authored fixture becomes available, document its exact
origin, authoring application/version, license and hash before including it;
do not use private or unvetted documents.

The issue #6 branded report is an optional additional visual fixture once
available; its absence does not block this checklist. Record its provenance and
hash independently and use its own feature labels as inspection targets.

## Procedure

1. Keep an untouched original; open a copy locally in the interactive viewer.
   Record OS and version, application name and exact version/build, date, file
   name, SHA-256 of the original, display environment and relevant installed
   fonts. Record any repair, conversion or security prompts, even if opening
   fails. Never overwrite the original.
2. Record separately: **open** (including warnings), **readability** (legible
   text and missing/garbled content), and **layout** (specific visible behavior).
   Use `not tested` where appropriate. Don't infer results for uninspected
   features, applications, versions or operating systems.
3. For the catalog, inspect labeled paragraphs and Unicode; heading and list
   hierarchy; table borders, merges and pagination; images and text wrapping;
   charts, equations and shapes; headers/footers and section breaks; and
   comments, notes, links, fields, revisions and controls where the viewer
   exposes them. Note clipping, overlap, font substitution, missing objects,
   unexpected page breaks and repair notices, with page and label. Compare
   edited versus original alignment/table and shared versus local headers.
   Use the catalog guide as the requested-content reference, not as proof of
   a particular viewer rendering.
4. Record initial cached fields and whether fields were updated; save or export
   only a separate copy, noting any actions. Viewer-calculated pagination,
   labels and fields, font availability and application settings can differ.
   A saved copy is a new artifact, not the original byte-parity baseline.
5. Record failures and limitations as well as successes. Attach screenshots only
   when permitted and identify the file hash, app, page and feature. Avoid
   publishing sensitive documents or screenshots.

## Observation record template

Copy once per file/application/platform:

```text
Date:
Fixture, provenance, SHA-256:
OS/version, display environment, relevant fonts:
Application and exact version/build:
Open (prompts/repair):
Readability (feature/page and result):
Layout (feature/page and result):
Not tested:
Field updates, settings, save/export actions:
Evidence / reproduction:
```

## Current observations and gaps

| Application / platform | Open / readability | Key features | Limitations | Targeted findings |
| --- | --- | --- | --- | --- |
| LibreOffice 25.8.7.3 580(Build:3), Ubuntu 25.10 x86_64, kernel 6.17.0-41-generic | GUI not tested | GUI not tested | No interactive display available in this session | Headless PDF conversion and limited extracted-content checks below |
| Desktop Word, platform/version not observed | Not tested | Not tested | No desktop Word observation available | None |

- On Ubuntu 25.10 x86_64, `libreoffice --version` reported
  `LibreOffice 25.8.7.3 580(Build:3)`; `uname -a` reported kernel
  `6.17.0-41-generic`. Both `DISPLAY` and `WAYLAND_DISPLAY` were empty.
  No interactive open, readability or layout was observed. The installed
  executable and its version alone are not a viewer-compatibility result.
- No desktop Word observation or redistributable Word-authored fixture has been
  established for this checklist. Those remain explicit gaps, as does interactive
  LibreOffice inspection.
- Independent python-docx 1.2.0 readback opened the repository editor fixture
  (4 top-level paragraphs, 0 tables) and pre-existing Rust and Python catalog
  artifacts (279 top-level paragraphs, 29 tables each). The two pre-existing
  catalog files both had SHA-256
  `24325a9a3f997433ec0b91dbe5212f51c71878b4ba8cae8dbe3d6b16462f026c`;
  the editor fixture had SHA-256
  `919563e963cfc6f131007ce10359b4309379880c37162fe168fcba22e0cf0f2a`.
  These are readback observations of those exact files, not GUI observations
  or proof of the current source's generated output.
- The quickstart and compact fixture commands above ran locally. Independent
  readback checked the quickstart text and the compact table, bullet paragraphs
  and inline image; ZIP integrity checks passed. Neither was opened in a GUI.

## Headless LibreOffice conversion probe, 2026-11-23

This is **not** an interactive viewer result: no GUI, repair prompt, visual
readability, image appearance, formatting, or layout was observed. Conversion
success does not prove that an interactive open has no repair warnings. Word
desktop and Word-authored inputs were not tested. LibreOffice's field updates
and PDF export are not evidence of Word's behavior.

On Ubuntu 25.10 x86_64, kernel 6.17.0-41-generic, LibreOffice
`25.8.7.3 580(Build:3)` ran with empty `DISPLAY` and `WAYLAND_DISPLAY`.
`Xvfb`, `xvfb-run`, `Xorg`, `weston`, `Xvnc`, and `xpra` were not found;
`Xwayland` was present but no Wayland display or compositor was available.
The input files were generated from rocx commit `7d8ce6b`, then converted
with a separate LibreOffice user profile. SHA-256 values were unchanged
after export.

| Input and provenance | SHA-256 | Headless result |
| --- | --- | --- |
| rocx Rust quickstart | `54af8daa7b1c954a4091811220af75753671c8e99df43d3fb50ea82f671d855e` | One-page PDF; extracted text contains both paragraphs, `café` and `🌍`. |
| python-docx compact synthetic fixture | `d3432cfb8bfac5e49203373ef4276edcb233f7573c1ed0fd30ccca4275234d11` | One-page PDF; extracted text contains all four table labels, both list items, `café` and `🌍`; `pdfimages -list` reports one 24 × 12 RGB image on page 1. Image color and placement were not visually checked. |
| rocx Rust edited catalog (generated template, not Word-authored) | `2272664cc4c39fa115779009731df2896d2ae92cb70190eca7b82550a170970b` | 25-page PDF; extracted text contains first-header and default-footer labels, the imported-editor paragraph and inserted table labels, and the `03 Styles` heading. This does not verify the style appearance, table geometry, or alignment. |

Reproduce from the repository root with Rust, python-docx, LibreOffice and
Poppler utilities installed. Use a fresh directory so prior exports cannot be
mistaken for results; PDF text extraction is a content check, not a visual check.
The compact fixture may have a different hash on another run.

```sh
run=$(mktemp -d /tmp/opencode/rocx-viewer-XXXXXX)
mkdir "$run/input" "$run/pdf"
cargo run --locked --example plain -- "$run/input/quickstart.docx"
uv run --locked --group test python tools/generate_viewer_compatibility_fixture.py "$run/input/compact.docx"
cargo run --locked --example all_features -- "$run/input/all_features.docx"
sha256sum "$run/input/"{quickstart,compact,all_features.edited}.docx
libreoffice "-env:UserInstallation=file://$run/lo-profile" --headless \
    --convert-to pdf --outdir "$run/pdf" \
    "$run/input/"{quickstart,compact,all_features.edited}.docx
for file in "$run/pdf/"*.pdf; do
    pdfinfo "$file"
    pdftotext -layout "$file" -
done
pdfimages -list "$run/pdf/compact.pdf"
sha256sum "$run/input/"{quickstart,compact,all_features.edited}.docx
```
