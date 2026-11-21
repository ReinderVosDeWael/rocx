"""Inline DrawingML shapes with dimensions in points."""
import dataclasses
from typing import Literal

from rocx.declarative.base import Component


@dataclasses.dataclass
class ShapeOutline:
    width: float
    color: tuple[int, int, int]


@dataclasses.dataclass
class ShapeTextFormat:
    """Uniform DrawingML text requests; None preserves consumer defaults.

    Attributes:
        font_size: Points in 1..4000, rounded to hundredths of a point.
        bold: Explicit true/false, independent of surrounding paragraph text.
        color: RGB text color, independent of the shape fill and outline.
        alignment: Left, center or right for each shape-text paragraph.
        vertical_alignment: Top, center or bottom within the shape text body.
    """

    font_size: float | None = None
    bold: bool | None = None
    color: tuple[int, int, int] | None = None
    alignment: Literal["left", "center", "right"] | None = None
    vertical_alignment: Literal["top", "center", "bottom"] | None = None


@dataclasses.dataclass
class ShapeRun(Component):
    width: float = 72
    height: float = 72
    geometry: Literal["rectangle", "ellipse"] = "rectangle"
    fill: tuple[int, int, int] | None = None
    text: str | None = None
    outline: ShapeOutline | None = None
    text_format: ShapeTextFormat | None = None
