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
