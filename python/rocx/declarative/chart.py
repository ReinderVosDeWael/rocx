"""Workbook-free inline DrawingML column charts."""
import dataclasses

from rocx.declarative.base import Component


@dataclasses.dataclass
class ColumnChartRun(Component):
    """One inline series in a body paragraph.

    Attributes:
        width: Positive width in points.
        height: Positive height in points.
        categories: Nonempty ordered list of labels.
        values: One finite number per label, in the same order.
    """

    width: float
    height: float
    categories: list[str]
    values: list[float]
