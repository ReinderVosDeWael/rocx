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
Use ``preview_tracked_bold`` for a direct bold-only property change and
inspect ``run.property_revision`` on reopen. ``preview_tracked_paragraph``
marks a whole eligible paragraph plus its text; inspect ``mark_revision``
and review the pair with ``preview_paragraph_revision``.

Use ``doc.style_anchor(id)`` to address a style by exact ID. ``StyleEdit.add``
accepts paragraph, character, table and numbering declarations. A numbering
style binds an existing concrete list. ``StyleEdit.field`` sets or clears a
direct scalar field; ``StyleEdit.region`` sets or clears a conditional table
region. Dotted fields such as ``spacing.after`` and ``lang.eastAsia`` patch
one stored attribute. Adding styles can create a missing styles part.
``replace`` and ``remove`` require unused, recognized custom definitions;
``retarget(source, replacement)`` migrates provable same-kind uses first.
Each ``ImportedStyle.property_xml`` entry contains one direct property group
or table region, with its namespace declarations.

``ImportedDocument.document_defaults`` returns the directly present run,
paragraph and settings defaults. ``ResourceEdit.document_default(field, value)``
sets one field, or clears it when ``value`` is ``None``. For example, ``False``
on ``"bold"`` is distinct from clearing it. Tab stops are triples of signed
twip position, alignment, and optional leader.

``ImportedDocument.numbering_catalog`` returns direct abstract/concrete entries
with complete modeled levels, nested label/paragraph formats, and stored style
IDs. The dictionary uses the same snake-case keys as CLI inspection. Missing
properties are omitted, opaque level content is flagged, and inherited levels
are not expanded.

``ImportedDocument.stories`` inspects effective default, first, and even header
and footer references without simulating page selection. Every effective slot
exposes an image-edit anchor; ``ImportedDocument.default_headers``
also exposes the default-header targets. Use ``edit_story_text(owner_part, edits)``
for single-text runs in a physical header/footer or note/comment owner.
Use ``edit_entry_inlines(owner_part, edits)`` for resolved inline insertion,
replacement, or direct formatting in ordinary note/comment definitions.
``edit_run_format(edits, owner_part=part)`` patches runs and hyperlink child
runs in one named header, footer, or entry part. Omit ``owner_part`` for direct
body paragraphs and paragraphs of directly inspected body table cells.
``edit_note_reference(owner_part, target, kind, ...)`` creates, attaches,
detaches, retargets, renumbers or deletes direct-body note references.
``edit_entry(owner_part, id, kind, ...)`` updates simple paragraphs, base
comment metadata or removes an unused ordinary note.
``part_tables(owner_part=None)`` discovers nested tables. Pass its anchors to
``edit_part_tables(owner_part, edits)`` for edits in that physical owner.
"""

from dataclasses import dataclass, field
from typing import Literal
from .declarative.content_control import ContentControlListItem, ContentControlMetadata
from ._native import (
    AbstractStart,
    ImportedNumberingInstance,
    ResourceEdit,
    StyleAnchor,
    StoryParagraphAnchor,
    StyleEdit,
    ControlAnchor,
    ControlSite,
    WrapperEdit,
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
    ImportedPropertyRevision,
    ImportedRun,
    ImportedFraction,
    DrawingOccurrence,
    ImportedSection,
    ImportedStory,
    ImportedTableCell,
    ImportedTableRow,
    InlineAnchor,
    ParagraphAlignmentEdit,
    InlineEdit,
    FieldEdit, PairedRangeEdit,
    ImportedField,
    BodyRangeMarker,
    ParagraphInsertion,
    ParagraphAnchor,
    TableInsertion,
    TableAnchor,
    TableInsertionChange,
)

from ._numbering_view import (
    ImportedConcreteNumbering,
    ImportedNumberingAbstract,
    ImportedNumberingCatalog,
    ImportedNumberingLevel,
)

# Both story views expose the same paragraph class.
ImportedStoryParagraph = ImportedHeaderParagraph


@dataclass
class ControlProperties(ContentControlMetadata):
    """Complete replacement definition; omitted optional properties are cleared."""

    text_type: Literal["rich", "plain", "dropdown"] = "rich"
    multi_line: bool | None = None
    items: list[ContentControlListItem] = field(default_factory=list)
    selected: str | None = None


__all__ = [
    "AbstractStart",
    "ImportedNumberingInstance",
    "ImportedConcreteNumbering",
    "ImportedNumberingAbstract",
    "ImportedNumberingCatalog",
    "ImportedNumberingLevel",
    "ResourceEdit",
    "StyleAnchor",
    "StoryParagraphAnchor",
    "StyleEdit",
    "ControlAnchor",
    "ControlSite",
    "WrapperEdit",
    "ControlProperties",
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
    "ImportedPropertyRevision",
    "ImportedRun",
    "ImportedFraction",
    "DrawingOccurrence",
    "ImportedSection",
    "ImportedStory",
    "ImportedStoryParagraph",
    "ImportedTableCell",
    "ImportedTableRow",
    "InlineAnchor",
    "ParagraphAlignmentEdit",
    "InlineEdit",
    "FieldEdit", "PairedRangeEdit",
    "ImportedField",
    "BodyRangeMarker",
    "ParagraphInsertion",
    "ParagraphAnchor",
    "TableInsertion",
    "TableAnchor",
    "TableInsertionChange",
]
