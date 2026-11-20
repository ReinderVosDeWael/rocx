"""Explicit document defaults; no effective formatting is calculated."""
import dataclasses

from rv_docx.declarative import formatting


@dataclasses.dataclass
class RunDefaults:
    """Run property overrides in document defaults; omitted fields retain templates.

    Attributes:
        font: Literal Latin font name.
        size: Nonnegative finite size in points.
        bold: Optional bold request.
        italic: Optional italic request.
        underline: Optional boolean or supported underline style.
        color: Optional RGB, auto or theme color.
        superscript: Optional superscript request.
        subscript: Optional subscript request.
        strike: Optional strikethrough request.
        all_caps: Optional capitals request.
        small_caps: Optional small capitals request.
        language: Language hints, without registry validation.
        bold_complex_script: Optional complex-script bold request.
        italic_complex_script: Optional complex-script italic request.
        size_complex_script: Complex-script size in points.
        rtl: Optional RTL run request.
        complex_script: Optional complex-script selection request.
        font_east_asia: Literal East Asian font name.
        font_complex_script: Literal complex-script font name.
    """

    font: str | None = None
    size: float | None = None
    bold: bool | None = None
    italic: bool | None = None
    underline: bool | str | None = None
    color: tuple[int, int, int] | str | dict | None = None
    superscript: bool | None = None
    subscript: bool | None = None
    strike: bool | None = None
    all_caps: bool | None = None
    small_caps: bool | None = None
    language: formatting.RunLanguage | None = None
    bold_complex_script: bool | None = None
    italic_complex_script: bool | None = None
    size_complex_script: float | None = None
    rtl: bool | None = None
    complex_script: bool | None = None
    font_east_asia: str | None = None
    font_complex_script: str | None = None


@dataclasses.dataclass
class ParagraphDefaults:
    """Paragraph defaults using points for lengths, not paragraph-style inches.

    Attributes:
        alignment: Optional left/center/right/justify request.
        spacing_before: Space before in points, including explicit zero.
        spacing_after: Space after in points, including explicit zero.
        line_spacing: Legacy line-height multiplier, exclusive with line_height.
        line_height: Optional typed line-height request.
        tab_stops: Nonempty list replaces template defaults' local tabs; empty
            preserves them. Positions use signed points.
        left_indent: Left indent in points.
        right_indent: Right indent in points.
        first_line_indent: Signed points; negative means hanging. Explicit values
            remove the conflicting template first-line/hanging attribute.
        keep_together: Optional keep-lines request.
        keep_with_next: Optional keep-next request.
        page_break_before: Optional page-break request.
        widow_control: Optional widow/orphan control.
        bidi: Optional paragraph direction, independent of run RTL.
    """

    alignment: str | int | None = None
    spacing_before: float | None = None
    spacing_after: float | None = None
    line_spacing: float | None = None
    line_height: formatting.LineHeight | None = None
    tab_stops: list[formatting.TabStop] = dataclasses.field(default_factory=list)
    left_indent: float | None = None
    right_indent: float | None = None
    first_line_indent: float | None = None
    keep_together: bool | None = None
    keep_with_next: bool | None = None
    page_break_before: bool | None = None
    widow_control: bool | None = None
    bidi: bool | None = None


@dataclasses.dataclass
class DocumentDefaults:
    """Merge explicit defaults without materializing formatting in content.

    Attributes:
        run: Run-default overrides, or None to preserve template defaults.
        paragraph: Paragraph-default overrides, or None to preserve templates.
            All fields support the generic deferred-value mechanism.
    """

    run: RunDefaults | None = None
    paragraph: ParagraphDefaults | None = None
