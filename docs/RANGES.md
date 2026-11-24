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

## Imported body ranges

Use the same numeric ID to insert, move or remove a pair. Supply a bookmark
name for a bookmark, or omit the name to target an existing comment
definition. Endpoints are inclusive direct body run/hyperlink anchors. They
may cross paragraphs and overlap other bookmark/comment ranges.

Inspect `doc.body_range_markers()?` (Rust), `doc.body_range_markers` (Python)
or `doc.bodyRangeMarkers()` (JavaScript) for direct marker kinds, original
body and inline indices, raw IDs, and bookmark names.

| Rust | Python | JavaScript |
| --- | --- | --- |
| `doc.set_paired_range(id, Some("Excerpt"), Some((&first, &last)))` | `doc.set_paired_range(id, "Excerpt", (first, last))` | `doc.setPairedRange({id, bookmark: "Excerpt", endpoints: [first, last]})` |
| `doc.set_paired_range(id, None, Some((&first, &last)))` | `doc.set_paired_range(id, endpoints=(first, last))` | `doc.setPairedRange({id, endpoints: [first, last]})` |
| `doc.set_paired_range(id, name, None)` | `doc.set_paired_range(id, name)` | `doc.setPairedRange({id, bookmark: name})` |

These methods return new DOCX bytes. Reopen after each edit. Moving preserves
the original markers and a supported reference run; removing a comment pair
also removes its reference, leaving the comment definition unchanged. Duplicate
IDs, partial pairs, non-body endpoints, reversed anchors, and protected or
opaque marker payloads are rejected. Bookmark removal is conservative in the
presence of field instructions or a hyperlink to the bookmark.
Creating a pair also guards numeric IDs of supported revisions and move-range
names across the package, so adding a range cannot invalidate later review.
