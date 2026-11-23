"""Inspect and edit an imported DOCX using snapshot-scoped anchors.

Pass table insertions and header image edits to ``ImportedDocument.preview``
with ``table_insertions=`` and ``header_images=`` to stage them alongside
paragraph alignment edits in one transaction. ``EditPreview.header_changes``
reports the effects on shared and section-local headers.
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
    ImportedTableCell,
    ImportedTableRow,
    InlineAnchor,
    ParagraphAlignmentEdit,
    ParagraphAnchor,
    TableInsertion,
    TableAnchor,
    TableInsertionChange,
)

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
    "ImportedTableCell",
    "ImportedTableRow",
    "InlineAnchor",
    "ParagraphAlignmentEdit",
    "ParagraphAnchor",
    "TableInsertion",
    "TableAnchor",
    "TableInsertionChange",
]
