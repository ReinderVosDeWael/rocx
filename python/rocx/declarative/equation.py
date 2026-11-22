"""Inline Office Math authoring."""
import dataclasses

from rocx.declarative.base import Component


@dataclasses.dataclass
class InlineFraction(Component):
    """A stacked fraction in a body paragraph, including body table cells.

    Both operands are nonempty single-line literal text and can be deferred.
    """

    numerator: str
    denominator: str
