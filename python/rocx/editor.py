"""Inspect and edit an imported DOCX using snapshot-scoped anchors.

Direct paragraphs in body table cells expose ``inline_items`` and
``inline_structure``. Use ``ImportedDocument.inline_at`` to resolve a run,
hyperlink, or revision anchor in the same snapshot. Supported direct-body run anchors may
also be used for text replacement or whole-run removal.

``ImportedDocument.preview`` stages paragraph alignments together with any of
``table_insertions=``, ``paragraph_insertions=``, ``cell_alignments=``,
``header_images=``, ``text_ranges=``, ``run_text=``, and ``run_removals=`` in one transaction.
All targets refer to the original snapshot.

``ImportedDocument.stories`` inspects effective default, first, and even header
and footer references without simulating page selection. Default and first-page
header stories expose image-edit anchors; ``ImportedDocument.default_headers``
also exposes the default-header targets.
"""

from ._native import (
    BlockEdit,
    AlignmentChange,
    BodyOpaque,
    BodyContentControl,
    BodyParagraph,
    BodyTable,
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
    "BlockEdit",
    "AlignmentChange",
    "BodyOpaque",
    "BodyContentControl",
    "BodyParagraph",
    "BodyTable",
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
