"""Image components for declarative documents."""

import dataclasses
import pathlib
from collections.abc import Coroutine
from typing import Literal

from rocx.declarative import base


@dataclasses.dataclass
class ImageCrop:
    """Physical crop insets in percent of the original picture dimensions."""

    left: float = 0
    top: float = 0
    right: float = 0
    bottom: float = 0


@dataclasses.dataclass
class ImageAnchor:
    """Floating picture offsets in points relative to the page or margins."""

    horizontal_reference: Literal["page", "margin", "column", "character", "leftMargin", "rightMargin", "insideMargin", "outsideMargin"] = "page"
    vertical_reference: Literal["page", "margin", "paragraph", "line", "topMargin", "bottomMargin", "insideMargin", "outsideMargin"] = "page"
    x: float = 0
    y: float = 0
    wrap: Literal["none", "square", "topAndBottom", "tight", "through"] = "none"
    relative_height: int = 0
    behind_text: bool = False
    wrap_side: Literal["bothSides", "left", "right", "largest"] | None = None
    distance_top: float | None = None
    distance_bottom: float | None = None
    distance_left: float | None = None
    distance_right: float | None = None
    locked: bool | None = None
    layout_in_cell: bool | None = None
    allow_overlap: bool | None = None
    horizontal_alignment: Literal["left", "center", "right", "inside", "outside"] | None = None
    vertical_alignment: Literal["top", "center", "bottom", "inside", "outside"] | None = None
    wrap_polygon: list[tuple[int, int]] = dataclasses.field(default_factory=list)


@dataclasses.dataclass
class ImageRun(base.Component):
    """An image embedded in the document.

    Attributes:
        data: Image data as bytes, file path, or a coroutine that resolves to bytes.
        type: Image type (e.g., 'png', 'jpg', 'jpeg', 'bmp', 'gif', 'svg').
        transformation: Optional width and height in points, rotation in degrees, crop and anchor.
        alt_text: Alternative text for accessibility.
    """

    data: bytes | str | pathlib.Path | Coroutine[None, None, bytes]
    type: str | None = None
    transformation: dict[Literal["width", "height", "rotation", "crop", "anchor", "flip_horizontal", "flip_vertical"], int | float | bool | ImageCrop | ImageAnchor] | None = None
    alt_text: dict[Literal["title", "description", "name"], str] | None = None
