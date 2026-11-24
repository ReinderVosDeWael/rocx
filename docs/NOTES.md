# Footnotes, endnotes and structured comments

```python
from rocx import declarative as d

note = d.Note.footnote(0, [d.Paragraph(text="Source: annual report")])
body = d.Paragraph(children=[d.TextRun("Revenue increased."), d.NoteReference.footnote(0)])
document = d.Document([d.Section(children=[body])], notes=[note])
```

Rust uses `Note::footnote(NoteId::new(0), children)` and `NoteReference::footnote(NoteId::new(0))`. Both APIs also have endnote helpers. Declarations, targets and children can be deferred. Share target keys or resolved content, not owned Rust child futures.

## Referencing a note

- Footnotes and endnotes have separate unsigned 32-bit ID spaces. **Zero is valid**. Each reference needs one ordinary authored or imported definition; collisions, missing definitions and references to special entries fail. A conditionally omitted definition needs a template target.
- Reference notes from body paragraphs, including tables and hyperlinks; not from other notes, headers, footers or comments. A note can contain paragraphs, tables, images, links and bookmarks. Its ranges cannot cross into another note.
- The viewer supplies displayed numbers; plain-text projections omit them. A superscript reference mark begins each ordinary note (in a new leading paragraph if needed). Default drawing/table width uses the final body section's usable width.
- rocx supplies missing separators. Reference an ordinary note, not its separator or continuation notice.

## Structured comments

```python
comment = d.Comment([d.Paragraph(text="Check this figure.")], author="Ada", initials="AL")
p = d.Paragraph(children=[
    d.RangeStart.structured_comment("review", comment), d.TextRun("12%"),
    d.RangeEnd("review"),
])
```

In Rust use `RangeStart::structured_comment(id, Comment::new(children))` and `RangeEnd`. Anchor comments only in the main body. Paragraphs, tables, images, links and bookmarks are supported; nested and threaded comments are not. Omitted/empty author uses `Document.comment_author`; omitted date uses the snapshot timestamp. An explicit date must be a valid UTC `YYYY-MM-DDTHH:MM:SSZ` (no fractions, offsets or leap seconds). Initials are optional.

## Document-wide numbering defaults

| `DocumentDefaults` property | Values |
| --- | --- |
| `footnote_numbering`, `endnote_numbering` | Independent `NoteNumbering`: optional nonnegative `start`, `restart` (`continuous`, `eachSect`, `eachPage`), `format` (`decimal`, `lowerRoman`, `upperRoman`, `lowerLetter`, `upperLetter`) |
| `footnote_position` | `pageBottom`, `beneathText` |
| `endnote_position` | `sectEnd`, `docEnd` |

These change display requests, not note IDs, bodies or separator references. Rust uses `NoteNumberRestart`, `NoteNumberFormat`, `FootnotePosition` and `EndnotePosition`.

## Templates

`EditScope::SupportedStories` / `scope="supported_stories"` includes related notes and comments. [Replacement rules](RANGES.md#template-edits) apply to imported content before new content is appended. Untouched payloads retain their bytes. Resolved snapshots capture images; rendering does not reread source files.

Runnable examples: [Rust](../examples/notes.rs) · [Python](../examples/notes.py).
