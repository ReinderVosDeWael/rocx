`images/` contains tiny synthetic red 24×12 images generated using Pillow
(PNG/JPEG/BMP/TIFF at 144 DPI; GIF at its default 72 DPI). No user data is
included. `sample_zero_dpi.png` checks the documented 72-DPI fallback for
zero-valued PNG density metadata.

## python-docx editor fixture

`imported-editor.docx` is saved by python-docx, not by rocx or desktop Word.
It contains a formatted body paragraph with an empty run, bookmark endpoints,
and a two-run external link; a separate table insertion anchor; and two sections
with an inherited default header containing a link.
The saved package lets tests use the original input without regenerating it.

To regenerate it from the repository root:

```sh
uv run --with python-docx==1.2.0 --no-project python tests/fixtures/generate_imported_editor.py
```
