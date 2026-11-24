"""Inspect and edit an imported DOCX using snapshot-scoped anchors.

Direct paragraphs in body table cells expose ``inline_items`` and
``inline_structure``. Use ``ImportedDocument.inline_at`` to resolve a run,
hyperlink, or revision anchor in the same snapshot. Supported direct-body run anchors may
also be used for text replacement or whole-run removal.

``ImportedDocument.preview`` stages paragraph alignments together with any of
``table_insertions=``, ``paragraph_insertions=``, ``cell_alignments=``,
``header_images=``, ``text_ranges=``, ``text_insertions=``, ``run_text=``, ``run_removals=``,
``block_edits=``, ``inline_edits=``, ``table_structure=``, ``style_edits=``,
``resources=``, and ``controls=`` in one transaction.
All targets refer to the original snapshot.

``doc.preview_layout([LayoutEdit.section(doc.sections[0].anchor, "page_width",
15840)])`` previews direct layout fields in a separate atomic batch. Read the
``layout`` dictionary on direct body paragraphs and sections; use ``None`` to
clear a field. Section layout anchors work on table-first and empty sections.

Track a direct body run with ``preview_tracked_text`` or move one with
``preview_tracked_move``. Reopen the preview and use ``preview_revision`` to
accept or reject one wrapper (both wrappers for moves). Paragraph text omits
unreviewed wrappers; ``inline_structure`` retains their text and metadata.

Use ``doc.style_anchor(id)`` to address a style by exact ID. ``StyleEdit.add``
accepts paragraph, character, table and numbering declarations. A numbering
style binds an existing concrete list. ``StyleEdit.field`` sets or clears a
direct scalar field; ``StyleEdit.region`` sets or clears a conditional table
region. ``replace`` and ``remove`` require unused, recognized custom definitions.
Each ``ImportedStyle.property_xml`` entry contains one direct property group
or table region, with its namespace declarations.

``ImportedDocument.document_defaults`` returns the directly present run,
paragraph and settings defaults. ``ResourceEdit.document_default(field, value)``
sets one field, or clears it when ``value`` is ``None``. For example, ``False``
on ``"bold"`` is distinct from clearing it. Tab stops are triples of signed
twip position, alignment, and optional leader.

``ImportedDocument.stories`` inspects effective default, first, and even header
and footer references without simulating page selection. Every effective slot
exposes an image-edit anchor; ``ImportedDocument.default_headers``
also exposes the default-header targets. Use ``edit_story_text(owner_part, edits)``
for single-text runs in a physical header/footer or note/comment owner.
Use ``edit_entry_inlines(owner_part, edits)`` for resolved inline insertion,
replacement, or direct run bold in ordinary note/comment definitions.
``edit_note_reference(owner_part, target, kind, ...)`` creates, attaches,
detaches, retargets, renumbers or deletes direct-body note references.
``edit_entry(owner_part, id, kind, ...)`` updates simple paragraphs, base
comment metadata or removes an unused ordinary note.
``part_tables(owner_part=None)`` discovers nested tables. Pass its anchors to
``edit_part_tables(owner_part, edits)`` for edits in that physical owner.
"""

from ._native import (
    AbstractStart,
    ImportedNumberingInstance,
    ResourceEdit,
    StyleAnchor,
    StyleEdit,
    ControlAnchor,
    ControlEdit,
    ImportedControl,
    BlockEdit,
    LayoutEdit,
    SectionAnchor,
    AlignmentChange,
    BodyOpaque,
    BodyContentControl,
    BodyParagraph,
    BodyTable,
    LocatedTable,
    TableStructureEdit,
    CellAlignmentEdit,
    CellAlignmentChange,
    EditPreview,
    HeaderAnchor,
    HeaderImageChange,
    HeaderImageEdit,
    HeaderImagePreview,
    ImportedDocument,
    ImportedEntry,
    ImportedStyle,
    ImportedStyles,
    TextRangeEdit,
    TextInsertion,
    RunTextEdit,
    RunRemovalChange,
    RunTextChange,
    ImportedCellParagraph,
    ImportedHeader,
    ImportedHeaderParagraph,
    ImportedHyperlink,
    ImportedInline,
    ImportedRevision,
    ImportedRun,
    ImportedSection,
    ImportedStory,
    ImportedTableCell,
    ImportedTableRow,
    InlineAnchor,
    ParagraphAlignmentEdit,
    InlineEdit,
    ParagraphInsertion,
    ParagraphAnchor,
    TableInsertion,
    TableAnchor,
    TableInsertionChange,
)

# Both story views expose the same paragraph class.
ImportedStoryParagraph = ImportedHeaderParagraph

__all__ = [
    "AbstractStart",
    "ImportedNumberingInstance",
    "ResourceEdit",
    "StyleAnchor",
    "StyleEdit",
    "ControlAnchor",
    "ControlEdit",
    "ImportedControl",
    "BlockEdit",
    "LayoutEdit",
    "SectionAnchor",
    "AlignmentChange",
    "BodyOpaque",
    "BodyContentControl",
    "BodyParagraph",
    "BodyTable",
    "LocatedTable",
    "TableStructureEdit",
    "CellAlignmentEdit",
    "CellAlignmentChange",
    "EditPreview",
    "HeaderAnchor",
    "HeaderImageChange",
    "HeaderImageEdit",
    "HeaderImagePreview",
    "ImportedDocument",
    "ImportedEntry",
    "ImportedStyle",
    "ImportedStyles",
    "TextRangeEdit",
    "TextInsertion",
    "RunTextEdit",
    "RunRemovalChange",
    "RunTextChange",
    "ImportedCellParagraph",
    "ImportedHeader",
    "ImportedHeaderParagraph",
    "ImportedHyperlink",
    "ImportedInline",
    "ImportedRevision",
    "ImportedRun",
    "ImportedSection",
    "ImportedStory",
    "ImportedStoryParagraph",
    "ImportedTableCell",
    "ImportedTableRow",
    "InlineAnchor",
    "ParagraphAlignmentEdit",
    "InlineEdit",
    "ParagraphInsertion",
    "ParagraphAnchor",
    "TableInsertion",
    "TableAnchor",
    "TableInsertionChange",
]
