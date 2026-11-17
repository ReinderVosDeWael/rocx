"""Use the same declarations with the pinned CMI renderer and the Rust bindings."""
import io
from pathlib import Path

import pytest
from cmi_docx import declarative as reference
from rv_docx import declarative as bindings
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml.ns import qn

IMAGES = Path(__file__).parents[1] / "fixtures/images"


def semantics(document):
    def paragraph(p):
        fmt = p.paragraph_format
        return (p.text, p.style.name,
                tuple(getattr(fmt, n) for n in ["alignment", "space_before", "space_after", "line_spacing",
                    "left_indent", "right_indent", "first_line_indent", "keep_together", "keep_with_next",
                    "page_break_before", "widow_control"]),
                [(r.text, r.bold, r.italic, r.underline, r.font.name, r.font.size,
                  str(r.font.color.rgb), r.font.superscript, r.font.subscript, r.font.strike,
                  r.font.all_caps, r.font.small_caps) for r in p.runs])
    def table(t):
        return (t.style.name, t.autofit, [c.width for c in t.columns],
                [[(c.text, c.vertical_alignment, c._tc.grid_span, c._tc.vMerge,
                   [table(n) for n in c.tables]) for c in r.cells] for r in t.rows])
    images = []
    for shape in document.inline_shapes:
        rel = shape._inline.graphic.graphicData.pic.blipFill.blip.get(qn("r:embed"))
        images.append((shape.width, shape.height, document.part.related_parts[rel].blob))
    properties = document.core_properties
    styles = []
    for name in ["TestBody", "TestTable"]:
        if name in document.styles:
            s = document.styles[name]
            styles.append((name, s.base_style.name, s.font.name, s.font.size,
                           s.font.bold, s.font.italic, s.font.underline, str(s.font.color.rgb),
                           s.paragraph_format.left_indent, s.paragraph_format.first_line_indent))
    return {
        "paragraphs": [paragraph(p) for p in document.paragraphs],
        "tables": [table(t) for t in document.tables],
        "images": images,
        "comments": [(c.author, c.text) for c in document.comments],
        "sections": [(s.page_width, s.page_height, s.orientation, s.left_margin, s.right_margin,
                      [p.text for p in s.header.paragraphs], [p.text for p in s.footer.paragraphs])
                     for s in document.sections],
        "styles": styles,
        "metadata": tuple(getattr(properties, n) for n in ["author", "title", "subject", "comments", "keywords", "category", "version"]),
    }


def formatted(api):
    return api.Document(
        [api.Section(children=[api.Paragraph(text="Heading", heading=2),
            api.Paragraph(children=[
                api.TextRun(text="Unicode Ελληνικά 日本語 🌍 <&>", bold=True, italic=False,
                    underline=True, font="Arial", size=14, color=(10, 20, 30),
                    superscript=True, strike=True, all_caps=True, small_caps=True,
                    comment_text="Run comment"),
                api.Tab(), api.TextRun(text="sub", subscript=True), api.Break(),
                api.TextRun(text="line"), api.Break(type="page"), api.Break(type="column"),
            ], style="TestBody", alignment=WD_ALIGN_PARAGRAPH.JUSTIFY,
                spacing_before=6, spacing_after=12, line_spacing=1.25,
                left_indent=18, right_indent=9, first_line_indent=-6,
                keep_together=True, keep_with_next=False, page_break_before=False,
                widow_control=False, comment_text="Paragraph comment", comment_author="Ada"),
            api.Paragraph(text=""),
        ], headers={"default": api.Header(children=[api.Paragraph(text="header")])},
           footers={"default": api.Footer(children=[api.Paragraph(text="footer")])})],
        title="Title", creator="Creator", subject="Subject", description="Description",
        comments="Core comments", keywords="keywords", category="category", version="2.0",
        comment_author="Default author",
        styles=[api.ParagraphStyleDefinition(name="TestBody", base_style="Normal",
            next_paragraph_style="Normal", font="Calibri", font_size=11, bold=False, italic=True,
            underline=False, color=(30, 20, 10), spacing_after=4, left_indent=.25,
            first_line_indent=-.125)],
    )


def tables(api):
    cell = lambda text, **kw: api.TableCell(children=[api.Paragraph(text=text)], **kw)
    nested = api.Table(rows=[api.TableRow(children=[cell("nested")])])
    return api.Document([api.Section(children=[api.Table(
        style="TestTable", column_widths=[1440, 2000, 2200],
        borders=[api.TableBorder(side="insideH", sz=8, color=(1, 2, 3))],
        rows=[api.TableRow(children=[
            cell("merged", grid_span=2, vmerge="restart", background_color=(255, 0, 0),
                vertical_alignment="center", borders=[api.CellBorder(side="top", val="dashed", sz=4)]),
            api.TableCell(children=[nested, api.Paragraph(text="after")]),
        ]), api.TableRow(children=[cell("continued", grid_span=2, vmerge="continue"), cell("last")]),
            api.TableRow(children=[cell("short")])],
    )])], styles=[api.TableStyleDefinition(name="TestTable", base_style="Normal Table",
        whole_table=api.TableSectionFormat(font="Arial", font_size=10),
        first_row=api.TableSectionFormat(bold=True, color=(255, 255, 255), background=(0, 32, 96)))])


def images(api):
    runs = []
    for name in ["png", "jpg", "gif", "bmp", "tiff"]:
        runs.append(api.ImageRun(data=IMAGES / f"sample.{name}", transformation={"width": 72}))
    return api.Document([api.Section(children=[api.Paragraph(children=runs)])])


@pytest.mark.parametrize("factory", [formatted, tables, images])
async def test_binding_output_matches_actual_cmi(factory):
    expected = semantics(await factory(reference).to_docx())
    actual = semantics(await factory(bindings).to_docx())
    assert actual == expected


@pytest.mark.parametrize("index", [0, 1, 999])
async def test_templates_created_by_cmi_are_edited_equivalently(tmp_path, index):
    source = tmp_path / "template.docx"
    original = await reference.Document([reference.Section(children=[
        reference.Paragraph(children=[reference.TextRun(text="Hello {{NA", bold=True),
                                      reference.TextRun(text="ME}}", italic=True)]),
        reference.Table(rows=[reference.TableRow(children=[reference.TableCell(children=[reference.Paragraph(text="{{NAME}}")])])]),
        reference.Paragraph(text="Tail"),
    ])]).to_docx()
    original.save(source)
    def declaration(api):
        return api.Document([api.Section(children=[api.Paragraph(text="Inserted A")]),
                             api.Section(children=[api.Paragraph(text="Inserted B")])], title="Updated")
    expected = await declaration(reference).to_docx(reference.DocumentTemplate(source, {"{{NAME}}": "Alice"}, index))
    actual = await declaration(bindings).to_docx(bindings.DocumentTemplate(source, {"{{NAME}}": "Alice"}, index))
    assert semantics(actual) == semantics(expected)


@pytest.mark.parametrize("case", ["unknown_style", "bad_heading", "duplicate_style", "bad_widths", "missing_image", "missing_template", "invalid_template"])
async def test_common_rendering_errors_keep_reference_exception_types(tmp_path, case):
    invalid = tmp_path / "invalid.docx"
    invalid.write_text("not a zip")
    async def render(api):
        paragraph = api.Paragraph(text="text")
        styles = None
        template = None
        children = [paragraph]
        if case == "unknown_style":
            paragraph.style = "NoSuchStyle"
        elif case == "bad_heading":
            paragraph.heading = 10
        elif case == "duplicate_style":
            styles = [api.TableStyleDefinition(name="Duplicate"), api.TableStyleDefinition(name="Duplicate")]
        elif case == "bad_widths":
            children = [api.Table(rows=[api.TableRow(children=[api.TableCell()])], column_widths=[1440, 2880])]
        elif case == "missing_image":
            children = [api.Paragraph(children=[api.ImageRun(data=tmp_path / "missing.png")])]
        elif case == "missing_template":
            template = api.DocumentTemplate(tmp_path / "missing.docx")
        elif case == "invalid_template":
            template = api.DocumentTemplate(invalid)
        return await api.Document([api.Section(children=children)], styles=styles).to_docx(template)
    with pytest.raises(Exception) as expected:
        await render(reference)
    with pytest.raises(type(expected.value)):
        await render(bindings)
