"""Style definition components for declarative documents."""

from __future__ import annotations

import dataclasses
from typing import TYPE_CHECKING

from rocx.declarative import formatting

if TYPE_CHECKING:
    from docx.enum import text as docx_text


@dataclasses.dataclass
class NumberingStyleDefinition:
    """Name an authored list definition through a concrete numbering ID."""

    name: str
    numbering_id: int


@dataclasses.dataclass
class CharacterStyleDefinition:
    """Named character style using the same properties as TextRun formatting.

    Attributes:
        name: Style name; existing character styles are updated in place.
        base_style: Character parent name/ID, creation-only; forward links work.
        linked_style: Optional paragraph-style name/ID to pair with. Writes only
            this style's link, not a reciprocal link or inherited formatting.
        font: Optional Latin font name.
        size: Optional font size in points (not font_size).
        bold: Optional bold style toggle.
        italic: Optional italic style toggle.
        underline: Optional bool or supported underline style.
        color: Optional RGB, auto or supported theme color value.
        superscript: Optional superscript request.
        subscript: Optional subscript request.
        strike: Optional strikethrough toggle.
        all_caps: Optional capitals toggle.
        small_caps: Optional small capitals toggle.
        language: Optional RunLanguage hints.
        bold_complex_script: Optional complex-script bold toggle.
        italic_complex_script: Optional complex-script italic toggle.
        size_complex_script: Optional complex-script size in points.
        rtl: Optional RTL run request.
        complex_script: Optional complex-script selection request.
        font_east_asia: Optional East Asian literal font name.
        font_complex_script: Optional complex-script literal font name.
    """

    name: str
    base_style: str | None = None
    font: str | None = None
    size: float | None = None
    bold: bool | None = None
    italic: bool | None = None
    underline: bool | str | None = None
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
    snap_to_grid: bool | None = None
    linked_style: str | None = None


@dataclasses.dataclass
class TableSectionFormat:
    """Formatting options for a section of a table style.

    Attributes:
        font: Font name.
        font_size: Font size in points.
        bold: Apply bold formatting.
        italic: Apply italic formatting.
        underline: Apply underline formatting.
        color: Font color as (R, G, B) tuple (0-255).
        background: Cell shading fill color as (R, G, B) tuple (0-255).
        alignment: Paragraph alignment.
        spacing_before: Space before paragraph in points.
        spacing_after: Space after paragraph in points.
        line_height: Optional typed paragraph line-height request.
        tab_stops: Custom paragraph tab overrides in points; empty does not
            clear inherited stops. Explicit clear stops remove known positions.
        bidi: Optional paragraph direction request, not section/run direction.
            These fields support generic deferred inputs; no effective
            conditional formatting or layout is calculated.
        language: Optional RunLanguage hints, without registry validation.
        bold_complex_script: Optional complex-script bold style toggle.
        italic_complex_script: Optional complex-script italic style toggle.
        size_complex_script: Optional complex-script size in points.
        rtl: Optional RTL run request, separate from paragraph bidi.
        complex_script: Optional request to use complex-script formatting.
        font_east_asia: Optional literal East Asian font name.
        font_complex_script: Optional literal complex-script font name.
    """

    font: str | None = None
    font_size: int | None = None
    bold: bool | None = None
    italic: bool | None = None
    underline: bool | None = None
    color: tuple[int, int, int] | None = None
    background: tuple[int, int, int] | None = None
    alignment: docx_text.WD_PARAGRAPH_ALIGNMENT | None = None
    spacing_before: int | None = None
    spacing_after: int | None = None
    line_height: formatting.LineHeight | None = None
    tab_stops: list[formatting.TabStop] = dataclasses.field(default_factory=list)
    bidi: bool | None = None
    language: formatting.RunLanguage | None = None
    bold_complex_script: bool | None = None
    italic_complex_script: bool | None = None
    size_complex_script: float | None = None
    rtl: bool | None = None
    complex_script: bool | None = None
    font_east_asia: str | None = None
    font_complex_script: str | None = None


@dataclasses.dataclass
class TableStyleDefinition:
    """Definition of a named table style with per-section formatting.

    Attributes:
        name: The style name.
        base_style: Name of the style to inherit from.
        whole_table: Formatting applied to all cells.
        first_row: Header row formatting.
        last_row: Footer row formatting.
        first_column: First column formatting.
        last_column: Last column formatting.
        banding_1_row: Alternating row stripe 1 formatting.
        banding_2_row: Alternating row stripe 2 formatting.
        banding_1_column: Alternating column stripe 1 formatting.
        banding_2_column: Alternating column stripe 2 formatting.
        top_left_cell: Top-left corner cell formatting (nwCell).
        top_right_cell: Top-right corner cell formatting (neCell).
        bottom_left_cell: Bottom-left corner cell formatting (swCell).
        bottom_right_cell: Bottom-right corner cell formatting (seCell).

    Note:
        Unlike ``ParagraphStyleDefinition``, this always creates a new style.
        A ``ValueError`` is raised if a style with the given ``name`` already
        exists in the document.
    """

    name: str
    base_style: str | None = None
    whole_table: TableSectionFormat | None = None
    first_row: TableSectionFormat | None = None
    last_row: TableSectionFormat | None = None
    first_column: TableSectionFormat | None = None
    last_column: TableSectionFormat | None = None
    banding_1_row: TableSectionFormat | None = None
    banding_2_row: TableSectionFormat | None = None
    banding_1_column: TableSectionFormat | None = None
    banding_2_column: TableSectionFormat | None = None
    top_left_cell: TableSectionFormat | None = None
    top_right_cell: TableSectionFormat | None = None
    bottom_left_cell: TableSectionFormat | None = None
    bottom_right_cell: TableSectionFormat | None = None


@dataclasses.dataclass
class ParagraphStyleDefinition:
    """Definition of a named paragraph style.

    If a style with ``name`` already exists in the document it will be modified
    in place; otherwise a new style is created.

    Attributes:
        name: The style name. If a style with this name already exists in the
            document it will be modified in place; otherwise a new style is
            created.
        base_style: Name of the style to inherit from. Only applied when
            creating a new style.
        next_paragraph_style: Name of the style to apply to the next paragraph.
        linked_style: Optional character-style name/ID to pair with. Forward and
            deferred values work; omission preserves existing links. No automatic
            reciprocal link or effective formatting is produced.
        font: Font name.
        font_size: Font size in points.
        bold: Apply bold formatting.
        italic: Apply italic formatting.
        underline: Apply underline formatting.
        color: Font color as (R, G, B) tuple (0-255).
        alignment: Paragraph alignment.
        spacing_before: Space before paragraph in points.
        spacing_after: Space after paragraph in points.
        line_spacing: Line spacing multiplier.
        line_height: Typed multiple/exact/minimum line height, mutually exclusive
            with line_spacing. Exact and minimum values are points.
        tab_stops: Local custom tab overrides in points; inherited stops are
            additive. Use clear stops to remove known inherited positions.
        language: Optional RunLanguage hints, without registry validation.
        bold_complex_script: Optional complex-script bold style toggle.
        italic_complex_script: Optional complex-script italic style toggle.
        size_complex_script: Optional complex-script size in points.
        rtl: Optional right-to-left run request.
        complex_script: Optional request to use complex-script formatting.
        bidi: Optional paragraph RTL layout request (separate from run rtl).
        font_east_asia: Optional literal East Asian font name.
        font_complex_script: Optional literal complex-script font name.
        left_indent: Left indent in inches.
        right_indent: Right indent in inches.
        first_line_indent: First line indent in inches.
        keep_together: Keep all lines of paragraph on same page.
        keep_with_next: Keep paragraph with next paragraph.
        page_break_before: Start paragraph on new page.
        suppress_line_numbers: Optional line-number suppression.
        widow_control: Enable widow/orphan control.
    """

    name: str
    base_style: str | None = None
    next_paragraph_style: str | None = None
    font: str | None = None
    font_size: int | None = None
    bold: bool | None = None
    italic: bool | None = None
    underline: bool | None = None
    color: tuple[int, int, int] | None = None
    alignment: docx_text.WD_PARAGRAPH_ALIGNMENT | None = None
    spacing_before: int | None = None
    spacing_after: int | None = None
    line_spacing: float | None = None
    left_indent: float | None = None
    right_indent: float | None = None
    first_line_indent: float | None = None
    keep_together: bool | None = None
    keep_with_next: bool | None = None
    page_break_before: bool | None = None
    widow_control: bool | None = None
    line_height: formatting.LineHeight | None = None
    tab_stops: list[formatting.TabStop] = dataclasses.field(default_factory=list)
    language: formatting.RunLanguage | None = None
    bold_complex_script: bool | None = None
    italic_complex_script: bool | None = None
    size_complex_script: float | None = None
    rtl: bool | None = None
    complex_script: bool | None = None
    font_east_asia: str | None = None
    font_complex_script: str | None = None
    bidi: bool | None = None
    suppress_line_numbers: bool | None = None
    snap_to_grid: bool | None = None
    contextual_spacing: bool | None = None
    mirror_indents: bool | None = None
    linked_style: str | None = None
