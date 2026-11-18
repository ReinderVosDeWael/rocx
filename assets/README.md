`default.docx` is an empty document saved by python-docx 1.2.0, containing the
default styles, numbering, font table, theme, page settings, and related OPC
parts. It contains no user content. It is used as static data; runtime rendering
and template editing are implemented directly in Rust.

The original python-docx MIT license is included as `PYTHON-DOCX-LICENSE`.

The settings part is minimally corrected to include `w:zoom/@w:percent="100"`,
required by `CT_Zoom` in the ECMA-376 fifth-edition Transitional schema. The
original `bestFit` mode is retained. This makes the bundled starting document
schema-valid without rewriting arbitrary imported templates. Other part payloads
and the original MIT notice are retained.
