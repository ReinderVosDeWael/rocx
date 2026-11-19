"""External hyperlinks with independent, deferred display children."""
from __future__ import annotations

import dataclasses
from collections.abc import Awaitable, Callable, Sequence
from typing import TYPE_CHECKING

from rv_docx.declarative import base

if TYPE_CHECKING:
    from rv_docx.declarative import paragraph, range as ranges


@dataclasses.dataclass
class Hyperlink(base.Component):
    """An external absolute URI/IRI or explicit internal BookmarkTarget.

    Attributes:
        target: Absolute external URI/IRI or BookmarkTarget, optionally deferred.
        children: TextRun/ImageRun/Tab/Break display children or deferred sequence.
            Nested hyperlinks are deliberately unsupported (not an XSD restriction).
        tooltip: Optional hover text, resolved independently of the target.
        history: Optional Word visited-history flag; does not fetch the target.
    """

    target: (
        str | ranges.BookmarkTarget
        | Awaitable[str | ranges.BookmarkTarget]
        | Callable[[], str | ranges.BookmarkTarget | Awaitable[str | ranges.BookmarkTarget]]
    )
    children: (
        Sequence[paragraph.InlineElement]
        | Awaitable[Sequence[paragraph.InlineElement]]
        | Callable[[], Sequence[paragraph.InlineElement] | Awaitable[Sequence[paragraph.InlineElement]]]
    )
    tooltip: str | Awaitable[str] | Callable[[], str | Awaitable[str]] | None = None
    history: bool | Awaitable[bool] | Callable[[], bool | Awaitable[bool]] | None = None
