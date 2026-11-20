"""Declarative API for creating Word documents."""

from rv_docx.declarative.base import Component, Deferred, DeferredProperty, PropertyRef
from rv_docx.declarative.document import Document, DocumentTemplate
from rv_docx.declarative.defaults import DocumentDefaults, RunDefaults, ParagraphDefaults
from rv_docx.declarative.image import ImageRun
from rv_docx.declarative.hyperlink import Hyperlink
from rv_docx.declarative.field import Field, FieldInstruction
from rv_docx.declarative.formatting import LineHeight, TabStop, RunLanguage
from rv_docx.declarative.range import BookmarkTarget, RangeStart, RangeEnd
from rv_docx.declarative.notes import Note, NoteReference, Comment
from rv_docx.declarative.numbering import NumberingDefinition, NumberingLevel, NumberingReference, NumberingStartOverride
from rv_docx.declarative.paragraph import Break, Paragraph, Tab, TextRun
from rv_docx.declarative.section import (
    BlockChildren,
    Footer,
    Header,
    Section,
    SectionProperties,
)
from rv_docx.declarative.styles import (
    CharacterStyleDefinition,
    ParagraphStyleDefinition,
    TableSectionFormat,
    TableStyleDefinition,
)
from rv_docx.declarative.table import (
    CellMargins,
    RowHeight,
    CellBorder,
    Table,
    TableBorder,
    TableCell,
    TableRow,
    TableLook,
)

__all__ = [
    "BlockChildren",
    "Break",
    "CellBorder",
    "Component",
    "Deferred",
    "DeferredProperty",
    "PropertyRef",
    "Document",
    "DocumentTemplate",
    "DocumentDefaults",
    "RunDefaults",
    "ParagraphDefaults",
    "Footer",
    "Header",
    "ImageRun",
    "Hyperlink",
    "Field",
    "FieldInstruction",
    "LineHeight",
    "TabStop",
    "RowHeight",
    "CellMargins",
    "RunLanguage",
    "CharacterStyleDefinition",
    "NumberingStartOverride",
    "BookmarkTarget",
    "RangeStart",
    "RangeEnd",
    "Note",
    "NoteReference",
    "Comment",
    "NumberingDefinition",
    "NumberingLevel",
    "NumberingReference",
    "Paragraph",
    "ParagraphStyleDefinition",
    "Section",
    "SectionProperties",
    "Tab",
    "Table",
    "TableBorder",
    "TableCell",
    "TableRow",
    "TableLook",
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
