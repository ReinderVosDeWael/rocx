"""Inline rich-text content control declarations."""

from __future__ import annotations

import dataclasses
from typing import TYPE_CHECKING, Literal

from rocx.declarative.base import Component

if TYPE_CHECKING:
    from rocx.declarative.paragraph import InlineChildren
    from rocx.declarative.section import BlockChildren


@dataclasses.dataclass
class ContentControlMetadata:
    """Metadata for a rich-text row or cell content control."""

    alias: str | None = None
    tag: str | None = None
    lock: Literal["unlocked", "sdtLocked", "contentLocked", "sdtContentLocked"] | None = None
    id: int | None = None


@dataclasses.dataclass
class ContentControlListItem:
    display_text: str
    value: str


@dataclasses.dataclass
class InlineContentControl(Component):
    """Rich-text control for inline runs in a paragraph."""

    children: InlineChildren = dataclasses.field(default_factory=list)
    alias: str | None = None
    tag: str | None = None
    lock: Literal["unlocked", "sdtLocked", "contentLocked", "sdtContentLocked"] | None = None
    id: int | None = None
    text_type: Literal["rich", "plain", "dropdown"] = "rich"
    multi_line: bool | None = None
    items: list[ContentControlListItem] = dataclasses.field(default_factory=list)
    selected: str | None = None

@dataclasses.dataclass
class BlockContentControl(Component):
    """Rich-text control grouping paragraphs and tables within a story."""

    children: BlockChildren = dataclasses.field(default_factory=list)
    alias: str | None = None
    tag: str | None = None
    lock: Literal["unlocked", "sdtLocked", "contentLocked", "sdtContentLocked"] | None = None
    id: int | None = None
    text_type: Literal["rich", "plain", "dropdown"] = "rich"
    multi_line: bool | None = None
    items: list[ContentControlListItem] = dataclasses.field(default_factory=list)
    selected: str | None = None
