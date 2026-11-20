"""Table components for declarative documents.

See also https://ooxml.dev/docs/tables/ for details.
"""

from __future__ import annotations

import dataclasses
from typing import TYPE_CHECKING

from rv_docx.declarative import base, paragraph

if TYPE_CHECKING:
    from collections.abc import Callable, Coroutine, MutableSequence, Sequence
    from typing import Literal


@dataclasses.dataclass(frozen=True)
class CellMargins:
    """Cell-local margin overrides, independent of table defaults or bidi layout.

    Attributes:
        top: Optional nonnegative points above content.
        start: Optional nonnegative points at the leading cell edge.
        bottom: Optional nonnegative points below content.
        end: Optional nonnegative points at the trailing cell edge.
            Omitted sides are not authored; zero explicitly removes that padding.
    """

    top: float | None = None
    start: float | None = None
    bottom: float | None = None
    end: float | None = None


@dataclasses.dataclass(frozen=True)
class RowHeight:
    """An exact or minimum table row height, not a calculated row size.

    Attributes:
        rule: exact or at_least.
        value: Nonnegative finite points, rounded to the nearest twip.
    """

    rule: str
    value: float

    @classmethod
    def exact(cls, points: float) -> RowHeight:
        """Request an exact height; content may be clipped by consumers."""
        return cls("exact", points)

    @classmethod
    def at_least(cls, points: float) -> RowHeight:
        """Request a minimum height; consumers may expand it for content."""
        return cls("at_least", points)


@dataclasses.dataclass
class TableBorder:
    """Defines a table border.

    Attributes:
        side: The side of the cell for the border.
        val: The type of border; only single is currently supported.
        sz: Size of the border.
        color: Color of the border (RGB); 0-255.
    """

    side: Literal["top", "left", "bottom", "right", "insideH", "insideV"]
    sz: int = 1
    color: tuple[int, int, int] = (0, 0, 0)
    val: Literal["single"] = "single"

    @property
    def hex_color(self) -> str:
        """Color as hexadecimal."""
        return f"{self.color[0]:02x}{self.color[1]:02x}{self.color[2]:02x}".upper()


@dataclasses.dataclass
class CellBorder:
    """Defines a border for an individual table cell.

    Attributes:
        side: The side of the cell for the border. Valid sides are ``"top"``,
            ``"bottom"``, ``"start"`` (left in LTR documents), and ``"end"``
            (right in LTR documents).
        val: The type of border line.
        sz: Size of the border in eighths of a point (8 = 1pt).
        color: Color of the border as an RGB tuple; each component 0-255.
    """

    side: Literal["top", "bottom", "start", "end"]
    sz: int = 1
    color: tuple[int, int, int] = (0, 0, 0)
    val: Literal["single", "dashed"] = "single"

    @property
    def hex_color(self) -> str:
        """Color as uppercase hexadecimal string (no leading '#')."""
        return f"{self.color[0]:02x}{self.color[1]:02x}{self.color[2]:02x}".upper()


@dataclasses.dataclass
class TableCell(base.Component):
    """A table cell containing paragraphs or nested tables.

    Attributes:
        children: List of Paragraph or Table components, or coroutines that
            resolve to these types. May be a zero-argument callable for lazy
            evaluation (useful with ``condition``).
        grid_span: Number of columns this cell spans (horizontal merge).
            Defaults to None (no spanning).
        vmerge: Vertical merge role. ``"restart"`` marks the top cell of a
            vertical merge group; ``"continue"`` marks subsequent cells in the
            group. Defaults to None.
        borders: Per-side border definitions for this cell. Defaults to None.
        background_color: RGB background fill colour for the cell as a
            ``(red, green, blue)`` tuple; each component 0-255. Defaults to
            None (no background).
        vertical_alignment: Vertical alignment of content within the cell.
            ``"top"``, ``"center"``, or ``"bottom"``. Defaults to None (uses
            Word's default, which is top).
        margins: Optional CellMargins in points; accepts generic deferred inputs.
        text_direction: Optional checked lrTb/tbRl/btLr/lrTbV/tbRlV/tbLrV text flow,
            independent of paragraph bidi; accepts generic deferred inputs.
    """

    children: (
        MutableSequence[
            paragraph.Paragraph
            | Table
            | Coroutine[None, None, paragraph.Paragraph | Table]
        ]
        | Callable[
            [],
            MutableSequence[
                paragraph.Paragraph
                | Table
                | Coroutine[None, None, paragraph.Paragraph | Table]
            ],
        ]
        | None
    ) = None
    grid_span: int | None = None
    vmerge: Literal["restart", "continue"] | None = None
    borders: MutableSequence[CellBorder] | None = None
    background_color: tuple[int, int, int] | None = None
    vertical_alignment: Literal["top", "center", "bottom"] | None = None
    margins: CellMargins | None = None
    text_direction: str | None = None


@dataclasses.dataclass
class TableRow(base.Component):
    """A table row containing cells.

    Attributes:
        children: List of TableCell components or coroutines that resolve to
            cells. May be a zero-argument callable for lazy evaluation (useful
            with ``condition``).
        height: Optional exact/minimum RowHeight in points.
        repeat_header: Optional header repetition request. Only a contiguous
            leading group of header rows repeats in consuming applications.
        cant_split: Optional request to keep the row on one page if it fits.
            All three properties support generic deferred inputs.
    """

    children: (
        MutableSequence[TableCell | Coroutine[None, None, TableCell]]
        | Callable[[], MutableSequence[TableCell | Coroutine[None, None, TableCell]]]
    )
    height: RowHeight | None = None
    repeat_header: bool | None = None
    cant_split: bool | None = None


@dataclasses.dataclass
class Table(base.Component):
    """A table with rows and cells.

    Attributes:
        rows: List of TableRow components or coroutines that resolve to rows.
            May be a zero-argument callable for lazy evaluation (useful with
            ``condition``).
        column_widths: List of column widths in twips (DXA). 1440 twips equals
            1 inch; approximately 567 twips equals 1cm. Setting this implies
            fixed layout (autofit is disabled automatically).
        layout: Table layout type. ``"fixed"`` sets fixed layout (autofit
            disabled). ``"autofit"`` sets autofit layout and suppresses column
            widths even if ``column_widths`` is also provided.
        style: Table style name.
        borders: Cell border configuration.
        look: Optional TableLook activation requests. Missing fields retain the
            library defaults; no effective conditional formatting is computed.
        alignment: Optional left, center, right, start, or end inline-table
            alignment request. None retains style/consumer defaults. Independent
            of paragraph/cell alignment; supports generic deferred fields.
    """

    rows: (
        MutableSequence[TableRow | Coroutine[None, None, TableRow]]
        | Callable[[], MutableSequence[TableRow | Coroutine[None, None, TableRow]]]
    )
    column_widths: Sequence[int] | None = None
    layout: Literal["autofit", "fixed"] | None = None
    style: str | None = None
    borders: MutableSequence[TableBorder] | None = None
    look: TableLook | None = None
    alignment: str | None = None


@dataclasses.dataclass(frozen=True)
class TableLook:
    """Conditional table-style activation, independent of direct cell formatting.

    Attributes:
        first_row: Optional first-row formatting request; defaults to true.
        last_row: Optional last-row request; defaults to false.
        first_column: Optional first-column request; defaults to true.
        last_column: Optional last-column request; defaults to false.
        horizontal_banding: Optional row-banding request; defaults to true.
        vertical_banding: Optional column-banding request; defaults to false.
            Defaults here are existing library policy, not the normative default
            for omitted tblLook. Each field supports generic deferred inputs.
    """

    first_row: bool | None = None
    last_row: bool | None = None
    first_column: bool | None = None
    last_column: bool | None = None
    horizontal_banding: bool | None = None
    vertical_banding: bool | None = None
