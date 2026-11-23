"""Inspect and edit an imported DOCX using snapshot-scoped anchors.

Direct paragraphs in body table cells expose ``inline_items`` and
``inline_structure``. Use ``ImportedDocument.inline_at`` to resolve a run or
hyperlink anchor in the same snapshot. These anchors are read-only.

Pass table insertions and header image edits to ``ImportedDocument.preview``
with ``table_insertions=`` and ``header_images=`` to stage them alongside
paragraph alignment edits in one transaction. ``EditPreview.header_changes``
reports the effects on shared and section-local headers.

``ImportedDocument.stories`` inspects effective default, first, and even header
and footer references without simulating page selection. Existing default-header
image edits still use anchors from ``ImportedDocument.default_headers``.
"""

from ._native import (
    AlignmentChange,
    BodyOpaque,
    BodyParagraph,
    BodyTable,
    EditPreview,
    HeaderAnchor,
    HeaderImageChange,
    HeaderImageEdit,
    HeaderImagePreview,
    ImportedDocument,
    ImportedCellParagraph,
    ImportedHeader,
    ImportedHeaderParagraph,
    ImportedHyperlink,
    ImportedInline,
    ImportedRun,
    ImportedSection,
    ImportedStory,
    ImportedTableCell,
    ImportedTableRow,
    InlineAnchor,
    ParagraphAlignmentEdit,
    ParagraphAnchor,
    TableInsertion,
    TableAnchor,
    TableInsertionChange,
)

# Both story views expose the same paragraph class.
ImportedStoryParagraph = ImportedHeaderParagraph

__all__ = [
    "AlignmentChange",
    "BodyOpaque",
    "BodyParagraph",
    "BodyTable",
    "EditPreview",
    "HeaderAnchor",
    "HeaderImageChange",
    "HeaderImageEdit",
    "HeaderImagePreview",
    "ImportedDocument",
    "ImportedCellParagraph",
    "ImportedHeader",
    "ImportedHeaderParagraph",
    "ImportedHyperlink",
    "ImportedInline",
    "ImportedRun",
    "ImportedSection",
    "ImportedStory",
    "ImportedStoryParagraph",
    "ImportedTableCell",
    "ImportedTableRow",
    "InlineAnchor",
    "ParagraphAlignmentEdit",
    "ParagraphAnchor",
    "TableInsertion",
    "TableAnchor",
    "TableInsertionChange",
]
