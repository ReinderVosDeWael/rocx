# Footnotes, endnotes and structured comments

The library generates ordinary footnotes, endnotes and structured comments. It
does not calculate note numbering, paginate, evaluate fields, or create threaded
comments.

## Declarations and deferred values

Rust adds `Note`, `NoteReference`, `NoteKind`, `NoteId`, `NoteTarget`, `Comment`,
`CommentMetadata` and read-only `ResolvedNote`/`ResolvedCommentBody` values.
`Document.notes` is a deferred collection of owned note declarations. A note
defers its target key and child blocks; a reference defers the same cloneable
`NoteTarget`. The normal `PropertyRef` mechanism shares keys or immutable child
content. Structural authoring lists/futures remain single-use, not cloneable.

```rust
use rocx::*;
let note = Note::footnote(NoteId::new(0), vec![Paragraph::new("Note body").into()]);
let reference = NoteReference::footnote(NoteId::new(0));
let mut document = Document::new(vec![Section::new(vec![
    Paragraph::from_children(vec![reference.into()]).into()
]).into()]);
document.notes = vec![note.into()].into();
```

Python adds `Note(kind, id, children)`, `NoteReference(kind, id)`, and the
`footnote`/`endnote` class helpers. `Document(notes=...)` and all note fields accept
ready/lazy/async/reference inputs through the unchanged interpreter. Conditional
definitions/references are omitted normally; a referenced omitted definition must
be provided by the template or the link is unresolved.

Reference numbers are intentionally absent from domain/plain-text projections.
The compiler emits `footnoteReference`/`endnoteReference` runs, and a corresponding
`footnoteRef`/`endnoteRef` mark at the start of each ordinary note body. A leading
marker paragraph is inserted when the first block is a table or the body is empty.
Marks use direct superscript formatting, without requiring a particular template
style. Word or another consumer determines display numbering and layout. Notes
currently use the final body's section usable width as the default drawing/table
width; this is not a footnote layout engine.

## Identity, special entries and owners

- Footnote and endnote IDs are separate namespaces. Authoring IDs are nonnegative
  32-bit values; **zero is valid for an ordinary note**.
- Missing/duplicate ordinary definitions, imported collisions, wrong part roots/
  content types and references to non-normal entries fail explicitly. Generated
  definitions do not overwrite or silently renumber imported ones. A new reference
  can target an imported ordinary ID without supplying another definition.
- Special entries are identified by `w:type`: `separator`,
  `continuationSeparator`, or `continuationNotice`. Their numeric IDs are not
  intrinsically reserved. Authored definitions only create normal entries.
- When writing note definitions, missing separator/continuation-separator entries
  are generated with unused negative IDs (normally -1 and -2) and corresponding
  settings references. Existing special entries are retained by type. Existing
  settings references must name special entries, not normal/missing notes.
- Imported decimal IDs are compared canonically, including signs, leading zeros,
  whitespace and values beyond machine-integer precision. Their lexical spelling
  is not rewritten merely for lookup. Public authoring values remain narrower.
- Owners are discovered through relationships. Conventional filenames are used
  only for new parts, with collision avoidance. Each note registers a separate
  logical story, even when notes share one physical part.
  Images/hyperlinks are registered against that physical owner, including relocated
  parts. Existing media deduplication and owner-local relationship IDs are reused.

New references are supported only in the main body, including its tables and
hyperlinks. References in notes themselves are forbidden by P1; references in
headers/footers/comments are outside this library's supported authoring contexts.
Notes support the existing paragraph/table/image/link/bookmark vocabulary. Range
endpoints cannot cross between different notes merely because their owner matches.
Comment generation remains body-anchored; nested comments or comments anchored
in note/header/footer/comment stories remain unsupported.

## Structured comments

Rust uses `RangeStart::structured_comment(id, Comment::new(children))` and a paired
`RangeEnd`. Comment child content and the `CommentMetadata` aggregate are deferred.
Python uses `RangeStart.structured_comment(id, Comment(children, author=...,
date=..., initials=...))`. Its fields support the same generic deferred inputs.
Supported content includes paragraphs, tables, images, links and bookmarks.

Author falls back to `Document.comment_author` when absent/empty. A missing date
uses the immutable snapshot timestamp; initials are optional. Explicit dates use
the checked UTC-seconds subset `YYYY-MM-DDTHH:MM:SSZ`, including calendar/leap-day
validation. Offset/fractional/leap-second date forms are not accepted by this
authoring convenience, even though the base schema is broader. Text comment
conveniences retain their previous author/time/initials behavior and share the
same anchor allocation/definition writer. No threaded-comment metadata is emitted.

## Templates, snapshots and evidence

`EditScope::SupportedStories` (Python `scope="supported_stories"`) visits related
note/comment content. Edits affect imported content, not newly appended content.
The fixed-marker/first-run replacement policy applies to these edits. Untouched note part
payloads retain bytes; appended/edited parts are reserialized namespace-safely.
This is checked base-vocabulary interpretation, not a general typed note importer
or complete validation of arbitrary extensions/MCE constructs.

Snapshots capture note/comment images during resolution, so rendering does not
read deleted source files or rerun producers. Examples are
[`notes.rs`](../examples/notes.rs) and [`notes.py`](../examples/notes.py).
Rust-to-python-docx OPC readback checks exact note IDs/content, owned media/links,
and structured comments; deliberate mutations must fail after successful loading.
Python-docx has no high-level note layout API here: note evidence uses its part
relationships plus independent XML parsing. Official XSD validation and an
independent note-graph checker cover missing/duplicate/special/nested references.
Neither readback nor schema validity proves pagination or visual equivalence.

References: ECMA-376 P1 (2016) §§11.3.4, 11.3.7 (parts), §17.11 and specifically
§§17.11.7, 17.11.10, 17.11.14 (reference/content constraints), §17.13.4 (comments),
§17.18.10 (decimal IDs), and normative Annex A.1; P2 (2021) §§6.4–6.5 (owners and
relationships). P1 distinguishes note roles by type and explicitly permits an
ordinary note with ID zero. Glossary notes and Microsoft threaded comment
extensions are unsupported.

### Document-wide numbering defaults

Set `DocumentDefaults.footnote_numbering` and
`DocumentDefaults.endnote_numbering` independently. Each `NoteNumbering` has
an optional nonnegative `start` and a restart mode of `continuous`,
`eachSect`, or `eachPage` in Python. Rust uses `NoteNumberRestart`.
Nested deferred values are supported in Python and Rust can defer the
defaults record. The codec merges the requests into the settings part's
`footnotePr` or `endnotePr`, preserving special-entry references. Note
identities and content remain independent of displayed numbering.
Sources: P1 (2016) §§17.11.4, 17.11.12, 17.11.19–20 and Annex A.1.

`NoteNumbering.format` accepts `decimal`, `lowerRoman`, `upperRoman`,
`lowerLetter` or `upperLetter` in Python and the corresponding
`NoteNumberFormat` variants in Rust. The compiler writes `numFmt` before
the start and restart settings. The consumer formats the resulting labels.

The document defaults also accept `footnote_position` (`pageBottom` or
`beneathText`) and `endnote_position` (`sectEnd` or `docEnd`) independently.
Rust uses `FootnotePosition` and `EndnotePosition`. These requests merge into
`pos` within the corresponding note settings, before format, start and
restart. These are document-wide defaults. See P1 (2016) §§17.11.4, 17.11.12
and Annex A.1 CT_FtnDocProps and CT_EdnDocProps.
