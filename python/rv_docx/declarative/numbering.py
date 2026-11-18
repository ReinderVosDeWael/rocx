"""Numbering declarations shared with the native domain (ECMA-376-1 §17.9)."""
import dataclasses


@dataclasses.dataclass
class NumberingLevel:
    level: int = 0
    start: int = 1
    format: str = "decimal"
    text: str = "%1."


@dataclasses.dataclass
class NumberingDefinition:
    id: int
    levels: list[NumberingLevel] = dataclasses.field(default_factory=list)


@dataclasses.dataclass
class NumberingReference:
    id: int
    level: int = 0
