"""Paragraph and text run components for declarative documents."""

from __future__ import annotations

import dataclasses
from collections.abc import MutableSequence
from typing import TYPE_CHECKING

from rv_docx.declarative import base
from rv_docx.declarative import formatting

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable, Coroutine

    from docx.enum import text as docx_text

    from rv_docx.declarative import image, hyperlink, range as ranges, notes, field

type InlineElement = TextRun | image.ImageRun | Tab | Break | hyperlink.Hyperlink | ranges.RangeStart | ranges.RangeEnd | notes.NoteReference | field.Field

type InlineChildren = MutableSequence[
    TextRun
    | image.ImageRun
    | Tab
    | Break
    | hyperlink.Hyperlink
    | ranges.RangeStart
    | ranges.RangeEnd
    | notes.NoteReference
    | field.Field
    | Coroutine[None, None, TextRun | image.ImageRun | Tab | Break | hyperlink.Hyperlink]
]


@dataclasses.dataclass
class TextRun(base.Component):
    """A run of text with formatting.

    Attributes:
        text: The text content. May be a zero-argument callable for lazy
            evaluation (useful with ``condition``).
        comment_text: Text content for a Word comment attached to this run.
        comment_author: Author name for the comment. Overrides the
            document-level default.
        bold: Apply bold formatting.
        italic: Apply italic formatting.
        underline: Apply underline formatting.
        font: Font name.
        size: Font size in points.
        color: Text color as (R, G, B) tuple (0-255).
        superscript: Apply superscript formatting.
        subscript: Apply subscript formatting.
        strike: Apply strikethrough formatting.
        all_caps: Apply all caps formatting.
        small_caps: Apply small caps formatting.
        language: Optional RunLanguage hints, without registry validation.
        bold_complex_script: Optional independent complex-script bold request.
        italic_complex_script: Optional independent complex-script italic request.
        size_complex_script: Optional complex-script size in points.
        rtl: Optional right-to-left run request; does not reverse stored text.
        complex_script: Optional request to use complex-script formatting.
            These fields accept generic deferred inputs.
        font_east_asia: Optional literal East Asian font name.
        font_complex_script: Optional literal complex-script font name.
        style: Optional character style name/ID; direct properties remain separate.
    """

    text: Awaitable[str] | Callable[[], Awaitable[str] | str] | str
    comment_text: Awaitable[str] | str | None = None
    comment_author: Awaitable[str] | str | None = None
    bold: bool | None = None
    italic: bool | None = None
    underline: bool | str | None = None
    font: str | None = None
    size: int | None = None
    color: tuple[int, int, int] | str | dict | None = None
    superscript: bool | None = None
    subscript: bool | None = None
    strike: bool | None = None
    all_caps: bool | None = None
    small_caps: bool | None = None
    language: formatting.RunLanguage | None = None
    bold_complex_script: bool | None = None
    italic_complex_script: bool | None = None
    size_complex_script: float | None = None
    rtl: bool | None = None
    complex_script: bool | None = None
    font_east_asia: str | None = None
    font_complex_script: str | None = None
    style: str | None = None


@dataclasses.dataclass
class Tab(base.Component):
    """A tab character."""


@dataclasses.dataclass
class Break(base.Component):
    """A line or page break.

    Attributes:
        type: Type of break ('line', 'page', 'column', 'textWrapping').
    """

    type: str = "line"


@dataclasses.dataclass
class Paragraph(base.Component):
    """A paragraph with optional formatting and child runs.

    Attributes:
        text: Shorthand for a paragraph with a single text run. May be a
            zero-argument callable for lazy evaluation (useful with
            ``condition``).
        comment_text: Text content for a Word comment attached to this paragraph.
        comment_author: Author name for the comment. Overrides the
            document-level default.
        children: List of TextRun, ImageRun, Tab, Break, Hyperlink, or coroutines
            that resolve to these types. May be a zero-argument callable for
            lazy evaluation (useful with ``condition``).
        heading: Heading level.
        style: Style name to apply.
        alignment: Paragraph alignment.
        spacing_before: Space before paragraph in points.
        spacing_after: Space after paragraph in points.
        line_spacing: Line spacing multiplier.
        line_height: Typed multiple/exact/minimum line height. Mutually exclusive
            with line_spacing; accepts the same generic deferred inputs.
        tab_stops: Local custom tab overrides in points; empty leaves inherited
            stops unchanged. Supports generic deferred inputs.
        bidi: Optional paragraph RTL layout request, independent of run RTL and
            stored text order. Supports generic deferred inputs.
        left_indent: Left indent in points.
        right_indent: Right indent in points.
        first_line_indent: First line indent in points.
        keep_together: Keep all lines of paragraph on same page.
        keep_with_next: Keep paragraph with next paragraph.
        page_break_before: Start paragraph on new page.
        widow_control: Enable widow/orphan control.
    """

    text: Awaitable[str] | Callable[[], Awaitable[str] | str] | str | None = None
    comment_text: Awaitable[str] | str | None = None
    comment_author: Awaitable[str] | str | None = None
    children: InlineChildren | Callable[[], InlineChildren] | None = None
    numbering: object | None = None
    heading: int | None = None
    style: str | None = None
    alignment: docx_text.WD_PARAGRAPH_ALIGNMENT | None = None
    spacing_before: int | None = None
    spacing_after: int | None = None
    line_spacing: float | None = None
    left_indent: int | None = None
    right_indent: int | None = None
    first_line_indent: int | None = None
    keep_together: bool | None = None
    keep_with_next: bool | None = None
    page_break_before: bool | None = None
    widow_control: bool | None = None
    line_height: formatting.LineHeight | None = None
    tab_stops: list[formatting.TabStop] = dataclasses.field(default_factory=list)
    bidi: bool | None = None

    def __post_init__(self) -> None:
        """Validate that either text or children is provided, not both.

        Raises:
            ValueError: If both text and children are provided, or neither is provided.
        """
        if self.text is not None and self.children is not None:
            msg = "Paragraph cannot have both 'text' and 'children'"
            raise ValueError(msg)
        if self.text is None and self.children is None:
            msg = "Paragraph must have either 'text' or 'children'"
            raise ValueError(msg)
