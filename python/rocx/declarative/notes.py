"""Ordinary note stories and base-schema structured comment content."""
from __future__ import annotations
import dataclasses
from rocx.declarative import base


@dataclasses.dataclass
class Note(base.Component):
    """A footnote/endnote definition, with independently deferred fields.

    Attributes:
        kind: 'footnote' or 'endnote'; each has its own ID namespace.
        id: Nonnegative 32-bit ordinary-note identity, including zero.
        children: Block content; may be deferred. Special separators are not ordinary notes.
    """
    kind: str
    id: int
    children: object

    @classmethod
    def footnote(cls, id: int, children: object) -> Note:
        """Create an ordinary footnote definition."""
        return cls("footnote", id, children)

    @classmethod
    def endnote(cls, id: int, children: object) -> Note:
        """Create an ordinary endnote definition."""
        return cls("endnote", id, children)


@dataclasses.dataclass
class NoteReference(base.Component):
    """An ordinary-note reference in the body, never expanded by the renderer.

    Attributes:
        kind: 'footnote' or 'endnote', optionally deferred.
        id: Referenced ordinary-note ID, optionally deferred.
    """
    kind: str
    id: int

    @classmethod
    def footnote(cls, id: int) -> NoteReference:
        """Reference an ordinary footnote."""
        return cls("footnote", id)

    @classmethod
    def endnote(cls, id: int) -> NoteReference:
        """Reference an ordinary endnote."""
        return cls("endnote", id)


@dataclasses.dataclass
class Comment(base.Component):
    """Structured content for a body-anchored base-schema comment range.

    Attributes:
        children: Paragraph/table content; may be deferred.
        author: Optional author; absent/empty falls back to document comment_author.
        date: Optional YYYY-MM-DDTHH:MM:SSZ timestamp; absent uses snapshot time.
        initials: Optional initials; no threaded-comment extension metadata is generated.
    """
    children: object = dataclasses.field(default_factory=list)
    author: str | None = None
    date: str | None = None
    initials: str | None = None
