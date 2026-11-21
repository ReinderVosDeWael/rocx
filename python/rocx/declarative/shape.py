"""Inline DrawingML shapes with dimensions in points."""
import dataclasses
from typing import Literal

from rocx.declarative.base import Component


@dataclasses.dataclass
class ShapeOutline:
    width: float
    color: tuple[int, int, int]


@dataclasses.dataclass
class ShapeRun(Component):
    width: float = 72
    height: float = 72
    geometry: Literal["rectangle", "ellipse"] = "rectangle"
    fill: tuple[int, int, int] | None = None
    text: str | None = None
    outline: ShapeOutline | None = None
