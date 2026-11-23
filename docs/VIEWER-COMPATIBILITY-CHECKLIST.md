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
| LibreOffice 25.8.7.3 580(Build:3), Ubuntu 25.10 x86_64, kernel 6.17.0-41-generic | Three fixtures displayed in GUI under Xvfb | Quickstart, compact fixture, edited catalog pages 1–2 | Only captured pages and features inspected; no physical desktop | GUI and separate headless observations below |
| Desktop Word, platform/version not observed | Not tested | Not tested | No desktop Word observation available | None |

- No desktop Word observation or redistributable Word-authored fixture has been
  established for this checklist. Those remain explicit gaps. The later Xvfb
  inspection below covers only specific LibreOffice pages and features.
- Independent python-docx 1.2.0 readback opened the repository editor fixture
  (4 top-level paragraphs, 0 tables) and pre-existing Rust and Python catalog
  artifacts (279 top-level paragraphs, 29 tables each). The two pre-existing
  catalog files both had SHA-256
  `24325a9a3f997433ec0b91dbe5212f51c71878b4ba8cae8dbe3d6b16462f026c`;
  the editor fixture had SHA-256
  `919563e963cfc6f131007ce10359b4309379880c37162fe168fcba22e0cf0f2a`.
  These are readback observations of those exact files, not GUI observations
  or proof of the current source's generated output.
- Independent readback checked the generated quickstart text and the compact
  table, bullet paragraphs and inline image; ZIP integrity checks passed.

## LibreOffice GUI probe under Xvfb, 2026-11-23

LibreOffice Writer `25.8.7.3 580(Build:3)` ran on Ubuntu 25.10 x86_64,
kernel `6.17.0-41-generic`, with the generic X11 VCL backend and a fresh user
profile. Xvfb used a 1440 × 1000, 24-bit screen and `-nolisten tcp`; there was
no physical desktop or window manager. `fc-match Calibri` returned Carlito and
`fc-match Cambria` returned Caladea; these fontconfig matches do not prove the
font used for every run. No manual field update, save, or export was made.
Inputs were opened directly; their SHA-256 hashes, listed in the headless table
below, were unchanged afterward.

Screenshots were inspected, not inferred from PDF text. Each showed a Writer
document with no repair dialog visible at capture; snapshots do not establish
that every possible prompt was absent throughout startup.

| Fixture | Observed GUI content | Scope and limitations |
| --- | --- | --- |
| rocx quickstart | Page 1 of 1: both paragraphs legible, including `café` and a globe emoji. | Captured at 154% zoom. No other content tested. |
| python-docx compact synthetic fixture | Page 1 of 1: blue heading, `café` and globe, bordered 2 × 2 table with four labels, two bulleted items, and a red rectangle below its label. | Captured at 100%. This is not a Word-authored fixture. |
| rocx edited catalog | Page 1: first-header label, underlined link, small red image, and readable opening text. Page 2: even-header label, centered imported-editor paragraph, two-cell inserted table with bold first-cell label, and blue heading samples. Page 1 footer shows `PAGE 1 / NUMPAGES 25`; Writer status shows 25 pages. | Captured at 95%; only portions of pages 1–2 inspected. No claim about remaining styles, objects, pagination, or other pages. Field values were displayed but no explicit field update was performed. The template was rocx-generated, not Word-authored. |

An earlier simultaneous launch using a different LibreOffice profile produced
a **Document in Use** dialog for `quickstart.docx`, stating that it was locked
for editing by the same user on a different system. This was a competing local
instance, not a repair dialog; that instance was terminated. The screenshots
above were taken from the generic-backend instance. After capture, the Xvfb
process reached its 120-second execution timeout; Writer then reported an X
I/O error and fatal signal. This shutdown is not a document compatibility
result. No desktop Word behavior or Word-authored source was tested.

To repeat with Xvfb, use a fresh LibreOffice profile and open copies of the
inputs from the headless probe below, one at a time. For example, with
`xvfb-run`, `xwininfo` and `ffmpeg` installed, launch an authenticated X11
session and print its environment:

```sh
gui=$(mktemp -d /tmp/opencode/rocx-viewer-gui-XXXXXX)
cp /path/to/quickstart.docx "$gui/"
xvfb-run -a -s '-screen 0 1440x1000x24 -nolisten tcp' sh -c '
    printf "DISPLAY=%s XAUTHORITY=%s\n" "$DISPLAY" "$XAUTHORITY"
    exec env SAL_USE_VCLPLUGIN=gen libreoffice "-env:UserInstallation=file://$1/profile" \
        --norestore --nologo "$1/quickstart.docx"
' sh "$gui"
```

In a second terminal, export the printed `DISPLAY` and `XAUTHORITY`. Run
`xwininfo -root -tree` to identify Writer's windows and capture the rendered
screen with `ffmpeg -f x11grab -video_size 1440x1000 -i "$DISPLAY" -frames:v 1 capture.png`.
Inspect the screenshot and any prompts. Page 2 of the edited catalog was
reached with Page Down.

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
