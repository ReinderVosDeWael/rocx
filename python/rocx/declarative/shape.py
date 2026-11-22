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
        italic: Explicit true/false; None omits the request.
        font: Latin typeface request; no installed-font lookup or embedding.
    """

    font_size: float | None = None
    bold: bool | None = None
    color: tuple[int, int, int] | None = None
    alignment: Literal["left", "center", "right"] | None = None
    vertical_alignment: Literal["top", "center", "bottom"] | None = None
    italic: bool | None = None
    font: str | None = None


@dataclasses.dataclass
class ShapeRun(Component):
    """Inline geometric shape or standalone plain DrawingML text box.

    Attributes:
        width: Positive width in points, within DrawingML coordinate bounds.
        height: Positive height in points, within DrawingML coordinate bounds.
        geometry: Rectangle, ellipse or text_box. Text boxes have no geometric
            fill/outline and use their own text transform rather than useSpRect.
        fill: Optional RGB geometric fill; not supported for text_box.
        text: Optional plain multiline text. None omits geometric-shape text;
            a text_box still creates one empty text paragraph.
        outline: Optional geometric line; not supported for text_box.
        text_format: Optional uniform text requests, shared by both modes.
    """

    width: float = 72
    height: float = 72
    geometry: Literal["rectangle", "ellipse", "text_box"] = "rectangle"
    fill: tuple[int, int, int] | None = None
    text: str | None = None
    outline: ShapeOutline | None = None
    text_format: ShapeTextFormat | None = None
