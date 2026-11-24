# Bookmarks, links and comment ranges

```python
from rocx import declarative as d

link = d.Hyperlink(d.BookmarkTarget("chapter"), [d.TextRun("Jump to chapter")])
heading = d.Paragraph(children=[
    d.RangeStart.bookmark("chapter"), d.TextRun("Chapter 1"), d.RangeEnd.bookmark("chapter"),
])
```

## Identity and pairing

- Match each `RangeStart` with a later `RangeEnd` of the same `RangeId`. The ID is a document-wide authoring key, not a character offset or XML ID. Bookmark helpers use `bookmark:<name>`; custom keys also work.
- Bookmark names are unique: 1–40 ASCII letters, digits or underscores; start with a letter or underscore. An empty bookmark is valid. A paired body-comment range must contain a leaf run, not just markers or an empty hyperlink.
- Pairs may overlap and cross paragraphs, sections or table cells **in the same story**. Put markers between paragraph children, not inside hyperlinks. Each header/footer definition, note and comment needs complete pairs of its own, even when stories share an XML part.
- Markers and target names can be deferred. A condition that skips only one endpoint leaves an invalid pair. Rust checks complete documents at resolution; Python checks pairing at render time.

## Internal links and imported collisions

Use `HyperlinkTarget::bookmark(name)` in Rust or `BookmarkTarget(name)` in Python. `"#name"` is not a bookmark reference. A target may appear later or in a related template story. Missing or ambiguous targets fail with `invalid_relationship`. Internal links emit no external relationship.

Use a new name when adding a bookmark to a template. To link to an existing bookmark, use its name without adding another pair of markers.

## Template edits

Replacement text goes into the first matched run. Markers stay in place, so replacing across a marker can change the text it encloses. Replacement cannot cross hyperlink, field, revision or control boundaries. Newly declared markers cannot pair with imported endpoints.

Runnable overlapping ranges: [Rust](../examples/ranges.rs) · [Python](../examples/ranges.py). See also [structured comments](NOTES.md#structured-comments) and [bookmark fields](FIELDS.md).
