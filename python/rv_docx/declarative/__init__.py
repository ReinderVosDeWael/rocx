"""Declarative API for creating Word documents."""

from rv_docx.declarative.base import Component
from rv_docx.declarative.document import Document, DocumentTemplate
from rv_docx.declarative.image import ImageRun
from rv_docx.declarative.paragraph import Break, Paragraph, Tab, TextRun
from rv_docx.declarative.section import (
    BlockChildren,
    Footer,
    Header,
    Section,
    SectionProperties,
)
from rv_docx.declarative.styles import (
    ParagraphStyleDefinition,
    TableSectionFormat,
    TableStyleDefinition,
)
from rv_docx.declarative.table import (
    CellBorder,
    Table,
    TableBorder,
    TableCell,
    TableRow,
)

__all__ = [
    "BlockChildren",
    "Break",
    "CellBorder",
    "Component",
    "Document",
    "DocumentTemplate",
    "Footer",
    "Header",
    "ImageRun",
    "Paragraph",
    "ParagraphStyleDefinition",
    "Section",
    "SectionProperties",
    "Tab",
    "Table",
    "TableBorder",
    "TableCell",
    "TableRow",
    "TableSectionFormat",
    "TableStyleDefinition",
    "TextRun",
]

# Stable tags keep subclasses compatible with the native conversion layer.
for _name in __all__:
    _value = globals()[_name]
    if isinstance(_value, type):
        _value._native_kind = _name
del _name, _value
