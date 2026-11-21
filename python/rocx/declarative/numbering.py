"""Numbering declarations shared with the native domain (ECMA-376-1 §17.9)."""
import dataclasses

from rocx.declarative.defaults import ParagraphDefaults, RunDefaults


@dataclasses.dataclass
class NumberingLevel:
    """A numbering level request; displayed labels are computed by the consumer.

    Attributes:
        level: Level index in 0..8.
        start: Nonnegative starting value.
        format: Numbering format token.
        text: Label pattern using level placeholders.
        is_legal: Optional decimal-display request for all referenced levels.
            None omits isLgl; false and true emit explicit requests. Does not
            change format. Supports generic deferred values.
        suffix: Optional tab, space, or nothing separator request. None omits
            suff (the consumer defaults to tab); no text is inserted by the
            library. Supports generic deferred values.
        restart: Optional nonnegative 32-bit one-based restart trigger; zero
            requests never restarting. None omits lvlRestart. Values ignored by
            the consumer are preserved; no counters are calculated. Supports
            generic deferred values.
        alignment: Optional left, center, right, start, or end label alignment.
            Distinct from paragraph text alignment. None omits lvlJc and retains
            consumer direction-dependent defaults. Supports deferred values.
        label_format: Direct run formatting for the generated numbering label.
        paragraph_format: Level paragraph properties with indents in points.
        paragraph_style: Optional registered paragraph style name or ID.
    """

    level: int = 0
    start: int = 1
    format: str = "decimal"
    text: str = "%1."
    is_legal: bool | None = None
    suffix: str | None = None
    restart: int | None = None
    alignment: str | None = None
    label_format: RunDefaults | None = None
    paragraph_format: ParagraphDefaults | None = None
    paragraph_style: str | None = None


@dataclasses.dataclass
class NumberingStartOverride:
    """A concrete list's start override, without computing displayed labels.

    Attributes:
        level: A unique level defined by the numbering definition (0..8).
        start: Nonnegative 32-bit start, including zero.
    """

    level: int
    start: int


@dataclasses.dataclass
class NumberingDefinition:
    """An explicit concrete list definition using the current abstract subset.

    Attributes:
        id: Explicit nonnegative 32-bit concrete identity; imported collisions fail.
        levels: Unique abstract level definitions in 0..8.
        abstract_id: Optional nonnegative 32-bit abstract identity. Definitions
            sharing an identity provide identical ordered levels. Imported IDs
            are reserved. None allocates a private abstract identity.
        inherit_style: Optional authored numbering style name or ID; omit levels
            and concrete overrides when inheriting.
        start_overrides: Optional concrete start overrides at defined levels.
            Fields support the existing generic deferred interpreter.
        level_overrides: Optional NumberingLevel replacements at unique defined
            levels. May coexist with start_overrides on the same level; the two
            requests serialize together without computing effective counters.
    """

    id: int
    levels: list[NumberingLevel] = dataclasses.field(default_factory=list)
    start_overrides: list[NumberingStartOverride] = dataclasses.field(default_factory=list)
    level_overrides: list[NumberingLevel] = dataclasses.field(default_factory=list)
    abstract_id: int | None = None
    inherit_style: str | None = None


@dataclasses.dataclass
class NumberingReference:
    id: int
    level: int = 0
