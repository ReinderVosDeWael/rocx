"""Declarative API for creating Word documents."""

from rocx.declarative.base import Component, Deferred, DeferredProperty, PropertyRef
from rocx.declarative.document import Document, DocumentTemplate
from rocx.declarative.defaults import DocumentDefaults, RunDefaults, ParagraphDefaults
from rocx.declarative.image import ImageRun
from rocx.declarative.hyperlink import Hyperlink
from rocx.declarative.field import Field, FieldInstruction
from rocx.declarative.formatting import LineHeight, TabStop, RunLanguage
from rocx.declarative.range import BookmarkTarget, RangeStart, RangeEnd
from rocx.declarative.notes import Note, NoteReference, Comment
from rocx.declarative.numbering import NumberingDefinition, NumberingLevel, NumberingReference, NumberingStartOverride
from rocx.declarative.paragraph import Break, Paragraph, Tab, TextRun
from rocx.declarative.section import (
    BlockChildren,
    Footer,
    Header,
    Section,
    SectionProperties,
)
from rocx.declarative.styles import (
    CharacterStyleDefinition,
    ParagraphStyleDefinition,
    TableSectionFormat,
    TableStyleDefinition,
)
from rocx.declarative.table import (
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
