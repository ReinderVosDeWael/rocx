"""Typed formatting values shared by paragraphs and paragraph styles."""
from __future__ import annotations

import dataclasses


@dataclasses.dataclass(frozen=True)
class RunLanguage:
    """Unparsed language hints; no registry lookup, proofing or shaping occurs.

    Attributes:
        latin: Optional default language string (w:val).
        east_asia: Optional East Asian language string (w:eastAsia).
        bidi: Optional complex-script language string (w:bidi).
            Strings are preserved, not normalized or checked against BCP 47.
    """

    latin: str | None = None
    east_asia: str | None = None
    bidi: str | None = None


@dataclasses.dataclass(frozen=True)
class TabStop:
    """A local custom tab override, not a computed effective paragraph tab.

    Attributes:
        position: Signed points relative to the paragraph's leading text margin;
            rounded to twips. Positions must be unique after rounding.
        alignment: left, center, right, decimal, bar, clear, start, or end.
        leader: Optional none, dot, hyphen, underscore, heavy, or middle_dot.
            Clear and bar stops do not advance text or generate a leader.
    """

    position: float
    alignment: str = "left"
    leader: str | None = None


@dataclasses.dataclass(frozen=True)
class LineHeight:
    """A line-height request, not a computed or effective layout measurement.

    Attributes:
        rule: multiple, exact, or at_least.
        value: Nonnegative finite multiple or point value, according to rule.
            Wire conversion rounds to 1/240 line or one twip respectively.
    """

    rule: str
    value: float

    @classmethod
    def multiple(cls, value: float) -> LineHeight:
        """Specify a multiple of the consumer's single-line height."""
        return cls("multiple", value)

    @classmethod
    def exact(cls, points: float) -> LineHeight:
        """Specify an exact height in points (which may clip tall content)."""
        return cls("exact", points)

    @classmethod
    def at_least(cls, points: float) -> LineHeight:
        """Specify a minimum height in points; consumers may expand it."""
        return cls("at_least", points)
