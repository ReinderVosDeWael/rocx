"""Right-side gutter requests are independent of section/text direction."""
import io
import pathlib

import docx
from lxml import etree
import pytest

from rocx import declarative as d
import test_notes

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


@pytest.mark.parametrize("rtl_gutter", [None, False, True])
async def test_section_gutter_request_keeps_bidi_and_physical_geometry_separate(rtl_gutter: bool | None) -> None:
    data = await d.Document([d.Section([d.Paragraph(children=[d.TextRun("first العربية last", rtl=False)], bidi=False)],
        properties=d.SectionProperties(rtl_gutter=rtl_gutter, bidi=False,
            page_margins={"left": 914400, "right": 1828800}))]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    properties = output.sections[0]._sectPr
    flag = properties.find(f"{W}rtlGutter")
    if rtl_gutter is None:
        assert flag is None
    else:
        assert flag.get(f"{W}val") == ("1" if rtl_gutter else "0")
    assert properties.find(f"{W}bidi").get(f"{W}val") == "0"
    assert output.sections[0].left_margin.inches == 1
    assert output.sections[0].right_margin.inches == 2
    assert output.paragraphs[0].text == "first العربية last"
    assert output.paragraphs[0].runs[0].font.rtl is False


async def test_section_gutter_inheritance_and_explicit_false_use_deferred_properties() -> None:
    source = d.Section([], properties=lambda: d.SectionProperties(rtl_gutter=lambda: True))
    hidden = d.Section([], properties=lambda: (_ for _ in ()).throw(AssertionError("hidden ran")))
    hidden.condition = False
    snapshot = await d.Document([
        d.Section([d.Paragraph(text="on")], properties=source.deferred("properties")), hidden,
        d.Section([d.Paragraph(text="inherited")]),
        d.Section([d.Paragraph(text="off")], properties=d.SectionProperties(rtl_gutter=False)),
        d.Section([d.Paragraph(text="inherited off")], properties=d.SectionProperties()),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert [s._sectPr.find(f"{W}rtlGutter").get(f"{W}val") for s in docx.Document(io.BytesIO(data)).sections] == ["1", "1", "0", "0"]


async def test_template_gutter_placement_is_preserved_or_overridden(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "gutter.docx"
    template.write_bytes(await d.Document([d.Section([], properties=d.SectionProperties(rtl_gutter=True))]).to_bytes())
    for value, expected in [(None, "1"), (False, "0")]:
        data = await d.Document([d.Section([], properties=d.SectionProperties(rtl_gutter=value))]).to_bytes(d.DocumentTemplate(template))
        assert docx.Document(io.BytesIO(data)).sections[0]._sectPr.find(f"{W}rtlGutter").get(f"{W}val") == expected


async def test_gutter_placement_does_not_change_size_top_setting_or_width_metrics(tmp_path: pathlib.Path) -> None:
    def gutter_size(root) -> None:
        root.find(f".//{W}sectPr/{W}pgMar").set(f"{W}gutter", "720")
    def top_gutter(root) -> None:
        etree.SubElement(root, f"{W}gutterAtTop").set(f"{W}val", "1")
    original = test_notes.rewrite(await d.Document([]).to_bytes(), "word/document.xml", gutter_size)
    original = test_notes.rewrite(original, "word/settings.xml", top_gutter)
    template = tmp_path / "top-gutter.docx"
    template.write_bytes(original)
    widths = []
    for value in [False, True]:
        data = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="same")])])])],
            properties=d.SectionProperties(rtl_gutter=value))]).to_bytes(d.DocumentTemplate(template))
        output = docx.Document(io.BytesIO(data))
        assert output.sections[-1]._sectPr.find(f"{W}pgMar").get(f"{W}gutter") == "720"
        assert output.settings.element.find(f"{W}gutterAtTop").get(f"{W}val") == "1"
        widths.append(output.tables[0].columns[0].width)
    assert widths[0] == widths[1]
