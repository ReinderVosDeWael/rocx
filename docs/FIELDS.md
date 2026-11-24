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

## Templates

Ordinary replacement inside a field result or across a field boundary fails with `unsupported_edit`. Protection for complex fields spans paragraphs in one story; an unclosed field protects the rest of that story, not other notes/comments. Unrelated edits preserve the field.

Runnable examples: [Rust](../examples/fields.rs) · [Python](../examples/fields.py).
