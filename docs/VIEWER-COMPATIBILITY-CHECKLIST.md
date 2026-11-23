# LibreOffice compatibility matrix

LibreOffice Writer displayed the four synthetic DOCX files below. Results
apply only to the listed pages, viewer build, and platform; they are not
a visual fidelity guarantee. Microsoft Word was not tested, and no input
was authored in Word.

## Fixture matrix — 2026-11-23

Writer `25.8.7.3 580(Build:3)` on Ubuntu 25.10 x86_64 (kernel
`6.17.0-41-generic`). GUI runs used the generic X11 VCL backend, a fresh
profile per file and a 1440 × 1000 × 24 Xvfb screen without a physical
desktop or window manager. Fontconfig matched Calibri to Carlito and Cambria
to Caladea; this does not identify the font used for every run.

All four files opened to readable document views with no repair dialog visible
in the inspected screenshots. Those captures do not establish that every
possible startup prompt was absent. Files were opened read-only; no explicit
field refresh, link activation or save was performed. Headless PDF conversion
used a separate profile with `DISPLAY` and `WAYLAND_DISPLAY` unset.

| Fixture and provenance | Writer GUI: readability and layout | Separate headless PDF check |
| --- | --- | --- |
| `quickstart.docx` — rocx Rust `plain` example | Page 1: both paragraphs legible, including `café` and the globe emoji. | 1 page; both paragraphs and Unicode present in extracted text. |
| `compact.docx` — independent synthetic python-docx 1.2.0 fixture | Page 1: blue heading, `café` and globe emoji, bordered 2 × 2 table with all four labels, two bulleted items, and a red rectangle below its label. | 1 page; cell labels, bullets and Unicode present in extracted text; `pdfimages` reports a 24 × 12 RGB image. |
| `all_features.docx` — rocx-generated catalog, before the imported-editor batch | Page 2: the labeled editor paragraph is left-aligned and followed by the heading samples, with no inserted table. | 25 pages; labeled paragraph and `03 Styles` present. |
| `all_features.edited.docx` — rocx-generated catalog built from its own template, then edited by rocx | Page 1: `Template REPLACED`, first header with an underlined link and red image, and footer `PAGE 1 / NUMPAGES 25`. Page 2: even header, centered editor paragraph, two-column inserted table with bold first-cell label, and blue headings. Page 6: `03 Styles`, bold blue paragraph style sample, an italic character style sample, and a readable updated-template style label. Page 7: default header with an underlined link and red image, and a shaded merged cell in the first table. | 25 pages; first/even/default header text, footer, editor paragraph, inserted table cell text and `03 Styles` present. |

These are the inputs generated at rocx commit `d5afcff`; both the originals
and the GUI copies retained their input bytes:

```text
54af8daa7b1c954a4091811220af75753671c8e99df43d3fb50ea82f671d855e  quickstart.docx
b448890cc991f80b753773165d661f75e83c6b7f0d1e38846fc5156cd08ee521  compact.docx
9a11f212275be7a0602b87b83938c4b6e9736766bfbc3198b7c08160a5a6e523  all_features.docx
c6d53fcb1eec613288af72ba01c11a87a3f1c94ccf05c9b5e1bfafa293d9ade9  all_features.edited.docx
```

**Limits.** Only the listed views and features were inspected. The updated
template style label on page 6 did not visibly demonstrate the bold/italic
combination described by its text; readback confirmed the definitions, not
Writer's effective formatting. The GUI footer values describe this run, not
stable pagination or field behavior. Other pages and advanced features (such as
charts, floating objects, revisions and controls), the dedicated shared/local
header edit cases, the independently authored editor fixture after editing,
other viewers/platforms, and save/reopen behavior were not tested here.

**Nonvisual cross-checks.** python-docx 1.2.0 confirmed quickstart paragraph
text, the compact table contents, bullet styles and inline image, and the
catalog pair's left-to-center alignment change, inserted table with bold
label, header text and template style definitions. ZIP integrity checks passed.
A separate byte comparison of the ZIP part payloads found that the editor
batch changed only `word/document.xml`; sibling payloads, including style,
header and relationship parts, matched exactly. Neither package readback nor
headless conversion establishes visual fidelity.

An earlier run at `7d8ce6b` covered the quickstart, independent compact fixture
and edited catalog pages 1–2 on the same Writer build. A competing local Writer
instance produced a **Document in Use** lock dialog, not a repair dialog.
An X I/O error when that run's Xvfb process timed out was a display teardown,
not a document compatibility result. The current run used separate read-only
copies and closed Writer before ending the display.

## Reproduce the check

### Generate and identify inputs

From the repository root, with Rust, Cargo and python-docx 1.2.0 available.
If needed, activate a Python environment containing python-docx, or use
`uv run --no-project --with python-docx==1.2.0 python` instead of `python`
for the compact fixture.

```sh
run=$(mktemp -d "${TMPDIR:-/tmp}/rocx-viewer-XXXXXX")
mkdir "$run/input" "$run/pdf"
cargo run --locked --example plain -- "$run/input/quickstart.docx"
python tools/generate_viewer_compatibility_fixture.py "$run/input/compact.docx"
cargo run --locked --example all_features -- "$run/input/all_features.docx"
(cd "$run/input" && sha256sum quickstart.docx compact.docx \
    all_features.docx all_features.edited.docx) > "$run/input.sha256"
cat "$run/input.sha256"
```

`all_features.template.docx` is the small intermediate generation template;
compare `all_features.docx` with `all_features.edited.docx` to see the separate
body edit. Hash the actual files inspected rather than expecting the compact
fixture's hash to be stable. The [catalog guide](../examples/ALL_FEATURES.md)
lists the feature labels and documents the full Rust/Python DOCX byte-parity
check. That check is separate from the viewer check.

### Inspect interactively

1. Open a copy in LibreOffice Writer, one file at a time, and record any
   repair, conversion or security prompt. Record app/build, OS, display, fonts,
   date and file hash. Keep originals untouched.
2. Compare the views with the matrix: quickstart and compact on page 1; compare
   the catalog pair at `Imported editor:`; inspect `03 Styles` and the labeled
   first/even/default headers in the edited catalog. Use the labels to find views
   if pagination differs. Report legibility and layout separately, including
   clipping, overlap, missing objects or unexpected page breaks if observed.
3. Record what was not tested. Note any field updates or settings changes and
   save or export only a separate copy. If attaching screenshots, identify the
   input hash, viewer build, page and feature. Do not use private documents.

For Linux without a desktop, start Writer on an authenticated Xvfb display
using `xvfb-run` and `xauth`. Repeat for each input with a new profile and
copy; close Writer before terminating Xvfb. This is still the Writer GUI,
not `--headless` mode.

```sh
file=all_features.edited.docx  # repeat with the other names in the matrix
gui=$(mktemp -d "$run/gui-XXXXXX")
cp "$run/input/$file" "$gui/"
xvfb-run -a -s '-screen 0 1440x1000x24 -nolisten tcp' sh -c '
    printf "export DISPLAY=%s XAUTHORITY=%s\n" "$DISPLAY" "$XAUTHORITY"
    exec env SAL_USE_VCLPLUGIN=gen libreoffice \
        "-env:UserInstallation=file://$1/profile" --view --norestore --nologo "$1/$2"
' sh "$gui" "$file"
```

In another terminal, set `DISPLAY` and `XAUTHORITY` to the printed values.
Identify the Writer window with `xwininfo -root -tree`. After the page has
rendered, capture the X11 screen and inspect the image:

```sh
ffmpeg -loglevel error -f x11grab -video_size 1440x1000 \
    -i "$DISPLAY" -frames:v 1 -threads 1 capture.png
```

Use Page Down or Writer's Find command on the Xvfb display to navigate and
capture the specific features. For example, if `xdotool` is installed, use
`xdotool windowfocus "$writer_window" key Next`, where `writer_window` is
the window ID from `xwininfo`. Wait for the page count to settle before
recording `NUMPAGES`; a window appearing does not mean Writer has finished
layout. After closing the viewer, use `cmp "$run/input/$file" "$gui/$file"`
to check the copy for changes.

### Optional headless cross-check

Use LibreOffice and the Poppler utilities. PDF extraction checks text,
reported page count and image inventory, not visual layout or GUI warnings.
Use a new output directory if repeating the export.

```sh
(
    unset DISPLAY WAYLAND_DISPLAY
    libreoffice "-env:UserInstallation=file://$run/lo-headless" --headless \
        --convert-to pdf --outdir "$run/pdf" \
        "$run/input/quickstart.docx" "$run/input/compact.docx" \
        "$run/input/all_features.docx" "$run/input/all_features.edited.docx"
)
for pdf in "$run/pdf/"*.pdf; do
    pdfinfo "$pdf"
    pdftotext -layout "$pdf" "${pdf%.pdf}.txt"
done
pdfimages -list "$run/pdf/compact.pdf"
(cd "$run/input" && sha256sum -c "$run/input.sha256")
```

### Observation record

```text
Date, source commit, fixture name/provenance/SHA-256:
Viewer version/build, OS, display, fonts:
GUI open (prompts/repair):
Readability (page, label, result):
Layout (page, label, result):
Not tested:
Field updates, settings, save/export actions:
Evidence (GUI captures and separate nonvisual checks):
```
