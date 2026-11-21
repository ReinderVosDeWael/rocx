"""Section, header, and footer components for declarative documents."""

import dataclasses
from collections.abc import Callable, Coroutine, MutableSequence
from typing import Literal

from rocx.declarative import base, content_control, paragraph, table

type BlockElement = paragraph.Paragraph | table.Table | content_control.BlockContentControl
type BlockChildren = MutableSequence[BlockElement | Coroutine[None, None, BlockElement]]
type HeaderFooterType = Literal["default", "first", "even"]


@dataclasses.dataclass
class LineNumbering:
    """Section line numbering. Distance uses twips."""

    count_by: int | None = None
    start: int | None = None
    distance: int | None = None
    restart: Literal["newPage", "newSection", "continuous"] | None = None
    clear: bool = False


@dataclasses.dataclass
class PageBorder:
    """A physical page-edge line border. Size is in eighth-points and space in points."""

    style: Literal["single", "dashed"]
    size: int | None = None
    space: int | None = None
    color: tuple[int, int, int] | None = None


@dataclasses.dataclass
class PageBorders:
    top: PageBorder | None = None
    left: PageBorder | None = None
    bottom: PageBorder | None = None
    right: PageBorder | None = None
    offset_from: Literal["page", "text"] | None = None
    display: Literal["allPages", "firstPage", "notFirstPage"] | None = None
    z_order: Literal["front", "back"] | None = None


@dataclasses.dataclass
class DocumentGrid:
    """Section grid pitch. Lines use twips and character deltas use 1/4096 points."""

    mode: Literal["default", "lines", "linesAndChars", "snapToChars"] | None = None
    line_pitch: int | None = None
    character_pitch_delta: int | None = None


@dataclasses.dataclass
class SectionProperties:
    """Configuration for a document section.

    Attributes:
        page_size: Page dimensions.
        page_orientation: 'portrait' or 'landscape'.
        page_margins: Page margin sizes. Keys are 'top', 'bottom', 'left', 'right';
            values are in EMU (English Metric Units). All keys are optional.
        page_numbering: Page numbering configuration.
        line_numbering: Optional section line-numbering settings.
        columns: Column configuration (dict with 'count', 'space', 'separator').
        vertical_align: Vertical alignment ('top', 'center', 'bottom', 'both').
        title_page: Use different header/footer on first page.
        type: Section break type ('nextPage', 'nextColumn', 'continuous',
            'evenPage', 'oddPage').
        bidi: Optional right-to-left section page direction request, separate
            from paragraph bidi and run RTL. Omission preserves inherited or
            template section properties; false explicitly requests LTR.
        rtl_gutter: Optional right-side gutter placement request, independent
            of section/text bidi. Omission preserves template/previous settings;
            false explicitly disables the request. No gutter size, margin
            mirroring or page layout is computed.
    """

    page_size: dict[Literal["width", "height"], int] | None = None
    page_orientation: Literal["portrait", "landscape"] | None = None
    page_margins: dict[Literal["top", "bottom", "left", "right", "header", "footer"], int] | None = None
    page_numbering: dict[Literal["start", "format"], int | str] | None = None
    columns: dict[Literal["count", "space", "separator"], int | bool] | None = None
    vertical_align: Literal["top", "center", "bottom", "both"] | None = None
    title_page: bool | None = None
    type: (
        Literal["nextPage", "nextColumn", "continuous", "evenPage", "oddPage"] | None
    ) = None
    bidi: bool | None = None
    rtl_gutter: bool | None = None
    line_numbering: LineNumbering | None = None
    page_borders: PageBorders | None = None
    document_grid: DocumentGrid | None = None


@dataclasses.dataclass
class Header(base.Component):
    """A section header.

    Attributes:
        children: List of Paragraph or Table components, or coroutines that
            resolve to these types. May be a zero-argument callable for lazy
            evaluation (useful with ``condition``).
    """

    children: BlockChildren | Callable[[], BlockChildren] | None = None


@dataclasses.dataclass
class Footer(base.Component):
    """A section footer.

    Attributes:
        children: List of Paragraph or Table components, or coroutines that
            resolve to these types. May be a zero-argument callable for lazy
            evaluation (useful with ``condition``).
    """

    children: BlockChildren | Callable[[], BlockChildren] | None = None


@dataclasses.dataclass
class Section(base.Component):
    """A document section with optional headers and footers.

    Attributes:
        children: List of Paragraph or Table components, or coroutines that
            resolve to these types. May be a zero-argument callable for lazy
            evaluation (useful with ``condition``).
        properties: Section configuration.
        headers: Dictionary mapping header types ('default', 'first', 'even')
            to Header components.
        footers: Dictionary mapping footer types ('default', 'first', 'even')
            to Footer components.
    """

    children: BlockChildren | Callable[[], BlockChildren] | None = None
    properties: SectionProperties | None = None
    headers: dict[HeaderFooterType, Header] | None = None
    footers: dict[HeaderFooterType, Footer] | None = None
