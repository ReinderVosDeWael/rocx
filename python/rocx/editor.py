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

Track a direct body run with ``preview_tracked_text`` or move one with
``preview_tracked_move``. Reopen the preview and use ``preview_revision`` to
accept or reject one wrapper (both wrappers for moves). Paragraph text omits
unreviewed wrappers; ``inline_structure`` retains their text and metadata.
Use ``preview_tracked_bold`` for a direct bold-only property change and
inspect ``run.property_revision`` on reopen. ``preview_tracked_paragraph``
marks a whole eligible paragraph plus its text; inspect ``mark_revision``
and review the pair with ``preview_paragraph_revision``.

Use ``doc.style_anchor(id)`` to address a style by exact ID. ``StyleEdit.add``
accepts paragraph, character and table declarations. ``StyleEdit.patch`` sets
direct bold/italic flags; ``replace`` and ``remove`` require unused, simple custom styles.

``ImportedDocument.stories`` inspects effective default, first, and even header
and footer references without simulating page selection. Every effective slot
exposes an image-edit anchor; ``ImportedDocument.default_headers``
also exposes the default-header targets. Use ``edit_story_text(owner_part, edits)``
for single-text runs in a physical header/footer or note/comment owner.
Use ``edit_entry_inlines(owner_part, edits)`` for resolved inline insertion,
replacement, or direct run bold in ordinary note/comment definitions.
"""

from dataclasses import dataclass, field
from typing import Literal
from .declarative.content_control import ContentControlListItem, ContentControlMetadata
from ._native import (
    AbstractStart,
    ResourceEdit,
    StyleAnchor,
    StyleEdit,
    ControlAnchor,
    ControlSite,
    ControlEdit,
    ImportedControl,
    BlockEdit,
    AlignmentChange,
    BodyOpaque,
    BodyContentControl,
    BodyParagraph,
    BodyTable,
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
    ImportedSection,
    ImportedStory,
    ImportedTableCell,
    ImportedTableRow,
    InlineAnchor,
    ParagraphAlignmentEdit,
    InlineEdit,
    ImportedField,
    BodyRangeMarker,
    ParagraphInsertion,
    ParagraphAnchor,
    TableInsertion,
    TableAnchor,
    TableInsertionChange,
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
    "ResourceEdit",
    "StyleAnchor",
    "StyleEdit",
    "ControlAnchor",
    "ControlSite",
    "ControlProperties",
    "ControlEdit",
    "ImportedControl",
    "BlockEdit",
    "AlignmentChange",
    "BodyOpaque",
    "BodyContentControl",
    "BodyParagraph",
    "BodyTable",
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
    "ImportedSection",
    "ImportedStory",
    "ImportedStoryParagraph",
    "ImportedTableCell",
    "ImportedTableRow",
    "InlineAnchor",
    "ParagraphAlignmentEdit",
    "InlineEdit",
    "ImportedField",
    "BodyRangeMarker",
    "ParagraphInsertion",
    "ParagraphAnchor",
    "TableInsertion",
    "TableAnchor",
    "TableInsertionChange",
]
