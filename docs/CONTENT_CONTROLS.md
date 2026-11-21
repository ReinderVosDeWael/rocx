# Content controls

Use `InlineContentControl` for a group of text runs, tabs, breaks or images
inside a paragraph. Rust takes optional `ContentControlProperties`. Python
accepts optional `alias`, `tag` and `lock` keyword arguments. The lock tokens
are `unlocked`, `sdtLocked`, `contentLocked` and `sdtContentLocked`. Rust
uses the corresponding `ContentControlLock` variants.

The compiler writes `sdt/sdtPr` and `sdt/sdtContent` using the inline
rich-text content model. Children and metadata can be deferred. A false
condition skips both. Nested controls, field-result controls, hyperlink
children that are controls, and data binding remain outside this subset.

Template replacement retains the existing structured-boundary guard for
imported controls. The visible catalog includes an inline control with
a descriptive alias, tag, content lock and bold text.

Source: P1 (2016) §17.5.2 and Annex A.1 CT_SdtRun, CT_SdtPr and ST_Lock.

Use `BlockContentControl` to group complete paragraphs and tables in a
story. It shares the alias, tag and lock properties with inline controls
and uses the block content model `CT_SdtBlock`. Directly nested block
controls are outside this subset. In a table cell, the compiler still
ensures a trailing paragraph. Resolved block controls expose their child
blocks separately from the section's direct paragraph list.

An optional control `id` assigns an explicit positive 32-bit signed identity.
Inline and block controls share the same ID namespace. The compiler rejects
collisions across authored controls and with IDs already present in the
template. Omission leaves the ID unset. Alias and tag remain independent.
Source: P1 (2016) Annex A.1 CT_SdtPr.

## Plain-text controls

Set `text_type="plain"` in Python or use
`ContentControlText::Plain { multi_line }` in Rust. Inline plain-text controls
accept a single text run. Block plain-text controls accept one paragraph with
a single text run. This subset excludes tabs, comments and tracked revisions
inside plain-text controls. With multiline enabled, line breaks are encoded
within the run. Omitted and explicit false multiline requests both require
single-line content and remain distinct in XML. Source: P1 (2016) §17.5.2
and Annex A.1 CT_SdtText.

## Drop-down controls

Use `text_type="dropdown"` in Python or `ContentControlText::DropDown` in
Rust. Provide a list of `ContentControlListItem` records with nonempty
display text and unique nonempty values, plus a selected value. The cached
text run must show the selected item's display text. Inline controls take one
run. Block controls take one paragraph containing one run. The codec emits
`dropDownList`, ordered `listItem` elements and `lastValue`. Source: P1
(2016) §17.5.2 and Annex A.1 CT_SdtDropDownList and CT_SdtListItem.

## Editing template controls by tag

Use `TemplateEdits.content_control_replacements` in Rust or
`DocumentTemplate(..., content_control_replacements={...})` in Python.
Each tag must identify exactly one imported plain-text control in the
selected story scope. The edit replaces its cached text while preserving
the run properties and control metadata. Bodies and selected related
stories use the existing template scope. Missing and ambiguous tags fail.

This operation targets a control with one existing text run, optionally
inside one paragraph. It rejects content-locked, data-bound, placeholder
and nested protected structures. Tabs are unsupported and line breaks
require the target's multiline setting. The standard text-replacement
operation retains its separate structured-boundary checks.

## Row and cell controls

Set `content_control` on a row or cell. Rust uses the corresponding row or
cell properties with `ContentControlProperties`. Python takes
`ContentControlMetadata` with optional alias, tag, lock and ID. These controls
wrap exactly one row or exactly one cell in its table context. Grid spans,
row flags, child order and the trailing cell paragraph are preserved.
This subset supports rich text in row and cell contexts. Source: P1 (2016)
§17.5.2 and Annex A.1 CT_SdtRow and CT_SdtCell.
