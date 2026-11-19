# Stories, bookmarks and range anchors

R3 supports paired bookmark and body-comment markers in Rust and Python. Ranges
can overlap and span paragraphs, sections and table cells in the same body story;
they are not nested containers. Notes and structured base comments are described
in [NOTES.md](NOTES.md); fields in [FIELDS.md](FIELDS.md). Revisions, threaded comments and a general typed
DOCX import model remain separate future features.

## Identity and pairing

- `RangeId` is a stable authoring identity, not a text offset or numeric XML ID.
  Each start identity is unique across the authored document. Its end must follow
  in the same logical story. An end without a start, duplicate start, unclosed
  start, or cross-story pair fails with a structural location. Validation paths
  describe the resolved tree; omitted children are not counted.
- `RangeStart` carries a `RangeKind::Bookmark(BookmarkName)` or
  `RangeKind::Comment(ResolvedComment)` payload; `RangeEnd` carries the same key.
  Endpoints do not use a stack, so crossing/overlapping pairs are legal.
- Rust's bookmark helpers derive the stable key from the name. Python's helpers
  use the same `bookmark:<name>` convention. Custom identities are also supported.
- Bookmark names are document-wide. Generated names use a Word-compatible
  application policy: 1–40 ASCII letters/digits/underscores, starting with a
  letter or underscore. This is narrower than the schema's `ST_String`.
- Empty bookmarks are allowed; explicit comment ranges must contain a leaf run.
  Marker-only and empty hyperlink containers do not count as runs. Comments are
  still restricted to the main body story. General range markers are supported
  at paragraph-inline boundaries, not inside hyperlink display children.

These are supported-authoring policies, not a complete list of schema rules.
P1 §17.13.4 permits single-point comment anchors in some cases; this API only
authors paired comment ranges. P1 §17.13.6.2 prescribes first-bookmark behavior
for duplicate names; this API instead rejects generated duplicates and ambiguous
internal references conservatively, without rewriting unrelated imported ranges.

Rust fields `RangeStart.id`, `.kind`, and `RangeEnd.id` use ordinary `Value<T>` and
`PropertyRef<T>` inputs. Python resolves all marker fields and `BookmarkTarget.name`
through its generic interpreter. Conditions omit markers normally; omitting just
one endpoint causes pairing validation to fail. Rust validates complete documents
during resolution; Python validates after direct native domain conversion at render
time. A standalone marker is not a complete story and cannot validate pairing.

## Logical story versus physical owner

`ooxml::StoryRegistry` explicitly registers logical stories with `StoryId`,
`StoryKind`, a typed OPC `PartName` owner, and `StoryContent` legal-content context.
Registering the same owner/kind again creates another identity: identity is not
inferred from a filename. The compiler has one body story across sections, reuses
the logical story of a reused header/footer part, and registers each comment
definition as a separate story even though comments share one XML part (text-only
or structured-block content). R4 also assigns separate identities to individual
footnotes/endnotes. Table cells retain their enclosing story.

Each authored header/footer definition must contain its own complete ranges;
ranges cannot be continued through separately supplied header definitions merely
because a template happens to map them to the same owner. Relationships belong
to the physical owner; range pairing belongs to the logical story.

## Internal links and imported collisions

Rust uses `HyperlinkTarget::bookmark(name)`; Python uses `BookmarkTarget(name)`.
The target is checked against generated and relationship-discovered imported
bookmarks, including forward references. Missing or ambiguous names fail with an
`invalid_relationship` diagnostic and target location. Internal links emit only
`w:anchor`, never a dummy external relationship. External URI strings retain R2
behavior; `"#name"` is not an implicit bookmark reference.

The compiler reserves existing bookmark names and numeric IDs in related supported
story parts, including relocated owners. Generated name collisions are rejected,
not silently renamed. Imported links, markers and unrelated payloads are not
rewritten. Bookmark IDs are allocated document-wide; comment IDs use their own
document-wide space, including imported definitions/references. Generated numeric
IDs have a 32-bit application limit, checked only when allocating a new ID. High
imported IDs can be retained or linked without allocating more IDs. This scan is
not exhaustive semantic validation or typed import of all existing ranges.
Reservation compares numeric decimal identities (including `-0`, `+0`, leading
zeros and surrounding whitespace), without rewriting imported lexical values.

Comment conveniences on runs/paragraphs now use the same wire-range allocation
and paired-anchor codec as explicit comment ranges. Existing author defaults,
body-only restrictions and comment reference runs remain unchanged.

## Template edits

Text replacement retains marker positions in the XML token stream. Inserted
replacement text belongs to the first matched run; later matched runs lose their
replaced text, without moving markers. Thus replacing text split across an anchor
can change the text enclosed by that anchor. No offset rebasing, anchor movement,
or inferred extension of ranges is performed. Replacements still cannot cross
hyperlink/field/revision/control boundaries. New declarations are resolved and
validated independently of template text changes; they do not pair with imported
numeric endpoints by coincidence.

See [`examples/ranges.rs`](../examples/ranges.rs) and
[`examples/ranges.py`](../examples/ranges.py). References: ECMA-376 P1 (2016)
§§17.13.4.3–17.13.4.5 (comment anchors), §§17.13.6.1–17.13.6.2 (bookmarks),
§17.16.22 (hyperlinks), and normative Annex A.1. Independent tests check exact
pairing and enclosed text, reject mutated endpoints, and validate representative
generated parts against official schemas. XSD success alone does not validate
pairing, navigation, layout or all imported range semantics.
