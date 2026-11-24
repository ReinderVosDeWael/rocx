"""Typed dictionary shapes for direct imported numbering readback."""

from typing import Literal, NotRequired, TypedDict


class _Language(TypedDict, total=False):
    latin: str
    east_asia: str
    bidi: str


class _ThemeColor(TypedDict):
    theme: str
    tint: NotRequired[int]
    shade: NotRequired[int]


class _LabelFormat(TypedDict, total=False):
    style: str
    bold: bool
    italic: bool
    underline: Literal["none", "single", "double", "dotted", "dash", "wave", "words"]
    font: str
    size: float
    color: Literal["auto"] | list[int] | _ThemeColor
    superscript: bool
    subscript: bool
    strike: bool
    all_caps: bool
    small_caps: bool
    snap_to_grid: bool
    language: _Language
    bold_complex_script: bool
    italic_complex_script: bool
    size_complex_script: float
    rtl: bool
    complex_script: bool
    font_east_asia: str
    font_complex_script: str


class _LineHeight(TypedDict):
    kind: Literal["multiple", "exact", "at-least"]
    value: float


class _TabStop(TypedDict):
    position: float
    alignment: Literal["left", "center", "right", "decimal", "bar", "clear", "start", "end"]
    leader: NotRequired[Literal["none", "dot", "hyphen", "underscore", "heavy", "middleDot"]]


class _ParagraphFormat(TypedDict, total=False):
    alignment: str
    spacing_before: float
    spacing_after: float
    left_indent: float
    right_indent: float
    first_line_indent: float
    line_height: _LineHeight
    tab_stops: list[_TabStop]
    bidi: bool
    keep_together: bool
    keep_with_next: bool
    page_break_before: bool
    widow_control: bool
    suppress_line_numbers: bool
    contextual_spacing: bool
    mirror_indents: bool
    snap_to_grid: bool


class ImportedNumberingLevel(TypedDict):
    """Direct modeled level fields; source-only content is indicated separately."""

    level: int
    has_opaque_content: bool
    start: NotRequired[int]
    format: NotRequired[str]
    text: NotRequired[str]
    restart: NotRequired[int]
    is_legal: NotRequired[bool]
    suffix: NotRequired[Literal["tab", "space", "nothing"]]
    alignment: NotRequired[Literal["left", "center", "right", "start", "end"]]
    paragraph_style: NotRequired[str]
    label_format: NotRequired[_LabelFormat]
    paragraph_format: NotRequired[_ParagraphFormat]


class ImportedNumberingAbstract(TypedDict):
    """A physical abstract and its stored numbering-style links."""

    abstract_id: int
    style_link: NotRequired[str]
    num_style_link: NotRequired[str]
    levels: list[ImportedNumberingLevel]


class _StartOverride(TypedDict):
    level: int
    start: int


class ImportedConcreteNumbering(TypedDict):
    """A physical instance, its direct style IDs and separate override kinds."""

    num_id: int
    abstract_id: int
    style_ids: list[str]
    start_overrides: list[_StartOverride]
    level_overrides: list[ImportedNumberingLevel]


class ImportedNumberingCatalog(TypedDict):
    """Direct numbering readback in package order, without inherited values."""

    abstracts: list[ImportedNumberingAbstract]
    instances: list[ImportedConcreteNumbering]
