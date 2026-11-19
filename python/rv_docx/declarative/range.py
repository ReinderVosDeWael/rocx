"""Stable paired markers for overlapping bookmark and body-comment ranges."""
from __future__ import annotations

import dataclasses
from rv_docx.declarative import base


@dataclasses.dataclass
class BookmarkTarget:
    """An internal hyperlink target, resolved by document-wide bookmark name.

    Attributes:
        name: A Word-compatible bookmark name; may be deferred.
    """

    name: str


@dataclasses.dataclass
class RangeStart(base.Component):
    """A range's start; ranges may overlap and span paragraphs in one story.

    Attributes:
        id: Stable authoring key, distinct from allocated XML IDs.
        kind: 'bookmark' or 'comment'.
        name: Bookmark name for a bookmark range.
        text: Comment text for a comment range.
        author: Optional comment author.
    """

    id: str
    kind: str
    name: str | None = None
    text: str | None = None
    author: str | None = None
    content: object | None = None

    @classmethod
    def bookmark(cls, name: str) -> RangeStart:
        """Create a named bookmark start; pair with RangeEnd.bookmark(name)."""
        return cls(f"bookmark:{name}", "bookmark", name=name)

    @classmethod
    def comment(cls, id: str, text: str, author: str | None = None) -> RangeStart:
        """Create a body-comment start with an explicit stable identity."""
        return cls(id, "comment", text=text, author=author)

    @classmethod
    def structured_comment(cls, id: str, content: object) -> RangeStart:
        """Pair a structured Comment's content with RangeEnd(id)."""
        return cls(id, "structured_comment", content=content)


@dataclasses.dataclass
class RangeEnd(base.Component):
    """The end of a preceding start with the same stable id in the same story.

    Attributes:
        id: The paired RangeStart's authoring key; may be deferred.
    """

    id: str

    @classmethod
    def bookmark(cls, name: str) -> RangeEnd:
        """Pair with RangeStart.bookmark(name)."""
        return cls(f"bookmark:{name}")
