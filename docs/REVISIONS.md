# Tracked text revisions

Set a text run's `revision` to a `TextRevision` with an insertion or deletion
kind, a nonblank author and an optional UTC timestamp in
`YYYY-MM-DDTHH:MM:SSZ` form. Rust uses `RevisionKind::Insertion` and
`RevisionKind::Deletion`. Python uses `"insertion"` and `"deletion"`.
Rust can defer the revision record. Python can defer the record or its fields.
A false run condition skips the revision producer.

The compiler wraps the formatted run in `w:ins` or `w:del`. Deleted text
uses `w:delText` with whitespace preserved. IDs are allocated deterministically
in render order, skipping numeric IDs reserved by the template. Rust snapshot
paragraph text includes inserted text and omits deleted text. Individual
resolved text runs retain their original contents and metadata.

Tracked changes for non-text components and paragraph or block changes
remain separate work. Ordinary template text replacement protects revision markup.

Source: P1 (2016) §17.13.5 and Annex A.1 CT_RunTrackChange and CT_TrackChange.


## Tracking future edits

Set `DocumentDefaults.track_revisions` to write `settings/trackRevisions`.
True requests tracking of future edits, and false writes an explicit off
value. Omission preserves the imported setting. The flag is independent of
authored and imported revision markup. Source: P1 (2016) §17.15.1.

## Accepting or rejecting imported text revisions

Set `TemplateEdits.text_revision_action` to `TextRevisionAction::Accept` or
`Reject` in Rust, or use `DocumentTemplate(..., text_revision_action="accept")`
or `"reject"` in Python. The operation applies to imported direct paragraph
insertions and deletions in the selected story scope. Each revision must
contain one text run outside protected structures. Accepting keeps the
inserted run and removes the deleted run. Rejecting keeps the deleted run,
restores `delText` as ordinary text, and removes the inserted run. Retained
run formatting stays intact. The compiler performs ordinary and tag-based
replacements first, then transforms revisions before rendering new declarations. Formatting change markup remains outside this text transformation. Source: P1 (2016) §17.13.5.

## Paired text moves

Use `RevisionKind::MoveSource` and `MoveDestination` with the same
`BookmarkName` in Rust. Python uses `"move_source"` and `"move_destination"`
with a shared `move_name`. An authored move requires exactly one source and
one destination in the body, with identical nonempty text, author and UTC
timestamp. The runs can appear in either order. Inline containers and
run-level comments are outside this subset.

The compiler writes move source and destination wrappers with paired range
markers. The two ranges share the move name and have distinct numeric IDs.
Imported names are reserved. The snapshot display projection includes the
destination and excludes the source. The text revision action also accepts or rejects paired direct body moves.
It reads adjacent start, run and end markers and checks both ranges for matching
content and metadata. Accept keeps the destination run and removes the source.
Reject keeps the source and removes the destination. Source: P1 (2016) §§17.13.5.22–25, 17.13.5.27–28 and
Annex A.1 CT_MoveBookmark.
