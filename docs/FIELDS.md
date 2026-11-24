# Fields and cached results

```python
from rocx import declarative as d

page = d.Field(d.FieldInstruction.page(), [d.TextRun("1")], dirty=True)
```

`"1"` is the **cached result**. rocx writes the field; the viewer decides whether to update it. rocx does not calculate page numbers, build a TOC, execute instructions or fetch references.

| Instruction | Rust `FieldInstruction` | Python `FieldInstruction` |
| --- | --- | --- |
| Current / total pages | `Page`, `NumPages` | `.page()`, `.num_pages()` |
| Bookmark content | `Reference(name)` | `.reference(name)` |
| TOC for headings 1–3 | `TableOfContents` | `.table_of_contents()` |
| Other instructions or switches | `raw(code)?` | `.raw(code)` |

Typed references require an unambiguous [bookmark](RANGES.md), authored or imported, even in nested results. Raw code must be nonempty XML-valid text. Whitespace is preserved; syntax and targets are **not** validated. Use trusted instructions.

## Cached content

Rust: `Field::new(instruction, children).with_properties(FieldProperties { form, dirty, locked })`. Python: `Field(instruction, children=[], form="simple", dirty=None, locked=None)`. Inputs can be deferred; a false condition skips the field.

- Use `form="simple"` (default) or `form="complex"`. Both expose the cached result as paragraph text.
- Cache formatted runs, tabs, breaks, images, hyperlinks or nested fields, or leave the result empty. Nest only in **results**. Fields are paragraph-local; no multi-block TOC results, fields inside hyperlinks or explicit range markers inside results. A range may enclose a whole field.
- `dirty=True` requests a refresh; `locked=True` requests no recalculation. False differs from omission. No global update setting is added.
- Fields can appear in headers, footers, notes and structured comments; normal story restrictions on children still apply.

## Editing an imported field or link

Use a paragraph anchor and the **original physical inline index** of a `fldSimple`
or complex field's begin run in a direct body paragraph. For example, inspect
the second physical inline with `doc.field_at(&p.anchor, 1)?` in Rust,
`doc.field_at(p.anchor, 1)` in Python, or
`doc.fieldAt({target: p.anchor, inlineIndex: 1})` in JavaScript. The record gives
the instruction, whether the field is complex, physical inline count, and raw
dirty/lock attributes. Cached text is never calculated.

| Operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Replace instruction; keep cached display | `patch_field(&p.anchor, i, FieldPatch::Instruction(s))` | `patch_field(p.anchor, i, "instruction", s)` | `patchField({target, inlineIndex: i, property: "instruction", value: s})` |
| Set/clear a flag | `FieldPatch::Dirty(Some(true))`, `FieldPatch::Locked(None)` | `"dirty", True` or `"locked", None` | `property: "dirty", value: true` or `property: "locked"` |
| Replace the whole field and cache, or remove it | `replace_field(&p.anchor, i, Some(&resolved_inline))`; `None` removes | `replace_field(p.anchor, i, resolved_field)`; omit the field to remove | `replaceField({target, inlineIndex: i, field})`; omit `field` to remove |
| Insert a resolved field or hyperlink | `InlineAction::InsertAfter` | `InlineEdit.insert_after` | `inlineEdits` kind `insertAfter` |
| Set/clear a direct hyperlink property | `InlineAction::HyperlinkTooltip`, `HyperlinkHistory` | `InlineEdit.hyperlink_tooltip`, `.hyperlink_history` | `hyperlinkTooltip`, `hyperlinkHistory` with optional `value` |
| Remove link and display children | `InlineAction::RemoveHyperlink` | `InlineEdit.remove_hyperlink` | `removeHyperlink` |

Field methods return new DOCX bytes; reopen before obtaining another anchor.
The supported source is a balanced paragraph-local field with ordinary text,
break/tab, hyperlink or nested supported field results. Cross-paragraph fields,
form data, opaque caches, formatted complex-field delimiters, and targets
inside a surrounding field or range are rejected. `replace_field` can switch
forms. A typed REF or internal hyperlink in an inserted inline must resolve to
one directly paired body bookmark. A raw instruction is not parsed.
`InlineAction::Replace` / `InlineEdit.replace` / `replace` can replace a whole
supported hyperlink's children and target. New external relationships belong
to the main document; old relationships are retained for other consumers.

## Templates

Ordinary replacement inside a field result or across a field boundary fails with `unsupported_edit`. Protection for complex fields spans paragraphs in one story; an unclosed field protects the rest of that story, not other notes/comments. Unrelated edits preserve the field.

Runnable examples: [Rust](../examples/fields.rs) · [Python](../examples/fields.py).
