# Content controls

```python
from rocx import declarative as d

status = d.InlineContentControl([d.TextRun("Draft", bold=True)],
                                tag="status", text_type="plain")
template = d.DocumentTemplate("report.docx", content_control_replacements={"status": "Final"})
```

## Choose a container

| Container | Rich content |
| --- | --- |
| `InlineContentControl` | Text runs, tabs, breaks or images within one paragraph |
| `BlockContentControl` | Complete paragraphs and tables; a table cell retains its trailing paragraph. Resolved children are separate from direct section paragraphs. |
| Row/cell `content_control` | Exactly one row or cell. Grid spans, row flags and child order are preserved. Rich text only. |

Use optional `alias`, `tag`, `lock` and `id` (Rust `ContentControlProperties`; Python row/cell `ContentControlMetadata`). Locks: `unlocked`, `sdtLocked`, `contentLocked`, `sdtContentLocked`. IDs, if supplied, must be positive signed 32-bit integers unique across authored and imported controls. Metadata and children may be deferred; false conditions skip both. No nested controls, controls inside hyperlinks or field results, or data binding.

## Plain-text controls

Use `text_type="plain"` or Rust `ContentControlText::Plain { multi_line }`. Supply one text run (inside one paragraph for a block). No tabs, comments or revisions. Line breaks require multiline mode. Omission and explicit false both require a single line but differ in XML.

## Drop-down controls

Use `text_type="dropdown"` or Rust `ContentControlText::DropDown`. Supply ordered `ContentControlListItem` entries with nonempty display text, unique nonempty values and a selected value. The sole cached run must contain the selected item's display text (inside one paragraph for a block).

## Editing template controls by tag

`content_control_replacements` replaces the cached text of exactly one imported plain-text control per tag within the selected story scope. It preserves run formatting and control metadata. Missing or ambiguous tags, content locks, data binding, placeholders and nested protected structures fail. Tabs are unsupported; line breaks require a multiline target. Ordinary text replacement cannot edit controls.
