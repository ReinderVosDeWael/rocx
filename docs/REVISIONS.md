# Tracked text revisions

```python
from rocx import declarative as d

added = d.TextRun("Revised wording", revision=d.TextRevision("insertion", "Ada"))
removed = d.TextRun("Old wording", revision=d.TextRevision("deletion", "Ada"))
```

Rust uses `RevisionKind::Insertion` / `Deletion`. Supply a nonblank author and optional UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`). The revision record can be deferred; a false run condition skips it. IDs are allocated in render order without colliding with template IDs. Snapshot paragraph text includes insertions and omits deletions; individual runs retain formatting, text and metadata.

## Tracking future edits

`DocumentDefaults.track_revisions=True` requests tracking in the viewer; false turns it off. Omit to preserve the template setting. This does not change existing or newly authored revision markup.

## Accepting or rejecting imported text revisions

```python
template = d.DocumentTemplate("review.docx", text_revision_action="accept")
```

Use `"accept"` / `"reject"` or Rust `TemplateEdits.text_revision_action` with `TextRevisionAction::Accept` / `Reject`. Each imported insertion/deletion must be a direct paragraph child containing one text run outside protected structures. Accept keeps inserted text; reject restores deleted text. Retained formatting survives. Within the selected stories, ordinary and tagged replacements run first, then revision actions, then new declarations. Ordinary replacement alone never changes revisions. Formatting and block changes are unsupported.

## Paired text moves

Use `"move_source"` and `"move_destination"` with the same `move_name` (Rust `RevisionKind::MoveSource` / `MoveDestination` with a `BookmarkName`). Supply one source and one destination in the body, in either order, with identical nonempty text, author and UTC timestamp. No inline containers or run-level comments. Imported names are reserved; the paired ranges get distinct numeric IDs.

Snapshot display text includes only the destination. For imported direct body moves, each range must have adjacent start, run and end markers and matching content/metadata. Accept keeps the destination; reject keeps the source.

## Review a single change in an existing DOCX

Get a direct body run from `inline_structure` (`inlineStructure` in JavaScript).
Each call stages one change and returns an independent `EditPreview`. Reopen its
DOCX bytes to get anchors for the new wrappers. Ordinary edits remain ordinary,
even when the document requests viewer tracking.

| Operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Track text | `doc.preview_tracked_text(&run, TrackedText::Replace("new".into()), "Ada", None)?` | `doc.preview_tracked_text(run, "replace", "Ada", text="new")` | `doc.previewTrackedText({target: run, kind: "replace", author: "Ada", text: "new"})` |
| Track a move | `doc.preview_tracked_move(&run, &after, "Move1", "Ada", date)?` | `doc.preview_tracked_move(run, after, "Move1", "Ada", date)` | `doc.previewTrackedMove({target: run, after, name: "Move1", author: "Ada", date})` |
| Review a wrapper | `doc.preview_revision(&wrapper, TextRevisionAction::Accept, None)?` | `doc.preview_revision(wrapper, "accept")` | `doc.previewRevision({target: wrapper, action: "accept"})` |

`TrackedText::InsertAfter` / `"insertAfter"` inserts text after the target in
its formatting; `TrackedText::Delete` / `"delete"` takes no text. Replacement
produces a deletion and an insertion; review each wrapper independently.
An unchanged replacement produces no revision. Authors must be nonblank;
text must be nonempty single-line XML text without tabs. Dates use the UTC
format above; only moves require a date.

Moves target two distinct run anchors and use a fresh name and IDs. Supply
both wrapper anchors for review, in either order: `Some(&other)` in Rust,
`paired=other` in Python, or `paired: other` in JavaScript. Accept keeps the
destination and reject keeps the source. Both legs must have adjacent markers,
matching names, text and metadata, and unambiguous IDs. A referenced name,
complex field instruction, or simple field other than a bare `PAGE` makes a
name unsafe to create or remove.

These operations support direct body paragraphs and single-text runs with
recognized local formatting outside fields and protected ranges. They reject
unsafe nested/opaque content and ambiguous IDs. Imported paragraph `text` is
the **ordinary-text projection**: it omits all unreconciled wrappers, unlike
the declarative display text described above. Inspect wrapper children to read
pending text. Saving a preview leaves the original snapshot and its anchors usable.

### Propose a formatting change

| | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Track direct bold | `doc.preview_tracked_bold(&run, false, "Ada", date)?` | `doc.preview_tracked_bold(run, False, "Ada", date)` | `doc.previewTrackedBold({target: run, bold: false, author: "Ada", date})` |
| Inspect a reopened run | `run.property_revision.as_ref()` | `run.property_revision` | `run.propertyRevision` |

The inspector exposes the literal `id`, `author` and optional `date` from the
first direct `w:rPrChange`. Its presence does not imply review is supported.
The edit records the prior run properties inside `w:rPrChange`; it does not
change text. Reopen the preview, then call the single-target review operation
above with the **run anchor**. Accept keeps the new direct bold; reject restores
the prior value, including inherited formatting (no explicit bold).

This profile supports a direct body single-text run with absent, empty or
bold-only run properties. Repeating its explicit value is a no-op; setting
`false` on an inherited value is a change. Nested, duplicate, unknown or
already-revised properties are not writable. Review requires a unique change
ID, supported old and current properties, and valid author/date metadata.
Paragraph-mark, paragraph-property and table-row revisions are not run changes.

### Propose or review a whole paragraph

| | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Insert before `p` | `doc.preview_tracked_paragraph(&p, TrackedParagraph::InsertBefore(text), "Ada", date)?` | `doc.preview_tracked_paragraph(p, "insertBefore", "Ada", text=text, date=date)` | `doc.previewTrackedParagraph({target: p, kind: "insertBefore", text, author: "Ada", date})` |
| Delete `p` | `doc.preview_tracked_paragraph(&p, TrackedParagraph::Delete, "Ada", date)?` | `doc.preview_tracked_paragraph(p, "delete", "Ada", date=date)` | `doc.previewTrackedParagraph({target: p, kind: "delete", author: "Ada", date})` |
| Inspect mark | `paragraph.mark_revision` | `paragraph.mark_revision` | `paragraph.markRevision` |
| Review both mark and text | `doc.preview_paragraph_revision(&p, TextRevisionAction::Accept)?` | `doc.preview_paragraph_revision(p, "accept")` | `doc.previewParagraphRevision({target: p, action: "accept"})` |

Insertion creates a plain paragraph from nonempty single-line text. Deletion
requires a direct body paragraph containing only one supported text run and no
paragraph properties. Both operations track the paragraph terminator **and**
the run: the `w:pPr/w:rPr` mark and inline wrapper have distinct IDs and the
same author/date. The ordinary text projection omits the pending run.

Reopen the output and select the **paragraph anchor** to review the complete
change. Accept insertion or reject deletion to keep a normal paragraph and
its run formatting; reject insertion or accept deletion to remove the entire
paragraph. The following block must be an ordinary paragraph without section
properties, complex markup or a protected boundary. An insertion may precede
the final paragraph, but the final paragraph cannot itself be marked for deletion.
Table boundaries, opaque blocks and changes crossing sections are excluded.
Inspection reads mark metadata even in shapes that cannot be reviewed.
The paired review rejects mismatches, missing/ambiguous IDs, extra content,
and unsupported formatting. If a caller reviews the inline wrapper separately,
the remaining paragraph mark no longer qualifies for whole-paragraph review.

Properties other than the supported run bold profile, block moves, tracked
tables, partial paragraph-mark merges, rich/nested review and multi-operation
revision batches are outside this profile.
