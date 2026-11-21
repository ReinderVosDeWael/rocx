"""Page borders preserve unedited sides and use schema edge order."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_page_borders_merge_a_deferred_side_in_schema_order(tmp_path):
    template = tmp_path / "bordered.docx"
    original = d.SectionProperties(page_borders=d.PageBorders(
        left=d.PageBorder("single", size=24, color=(0, 32, 64)),
        right=d.PageBorder("dashed", size=8),
    ))
    await d.Document([d.Section([], properties=original)]).save(template)
    update = d.SectionProperties(page_borders=d.PageBorders(
        top=lambda: d.PageBorder("dashed", size=12, space=7),
        bottom=d.PageBorder("single", size=2, space=0),
    ))
    data = await d.Document([d.Section([], properties=update)]).to_bytes(d.DocumentTemplate(template))
    borders = docx.Document(io.BytesIO(data)).sections[-1]._sectPr.find(f"{W}pgBorders")
    assert [border.tag for border in borders] == [f"{W}{side}" for side in ("top", "left", "bottom", "right")]
    assert borders.find(f"{W}top").get(f"{W}space") == "7"
    assert borders.find(f"{W}left").get(f"{W}color") == "002040"
    assert borders.find(f"{W}bottom").get(f"{W}space") == "0"
    assert borders.find(f"{W}right").get(f"{W}sz") == "8"


@pytest.mark.parametrize("field, value", [("size", 1), ("size", 97), ("space", 32)])
async def test_page_border_measurements_are_checked(field, value):
    props = d.SectionProperties(page_borders=d.PageBorders(top=d.PageBorder("single", **{field: value})))
    with pytest.raises(ValueError, match=rf"page_borders\.top\.{field}"):
        await d.Document([d.Section([], properties=props)]).to_bytes()


async def test_page_border_options_merge_without_replacing_edges(tmp_path):
    template = tmp_path / "page-border-options.docx"
    original = d.SectionProperties(page_borders=d.PageBorders(
        left=d.PageBorder("single"),
        offset_from="page",
        display="firstPage",
        z_order="back",
    ))
    await d.Document([d.Section([], properties=original)]).save(template)
    update = d.SectionProperties(page_borders=d.PageBorders(
        offset_from=lambda: "text",
        display="notFirstPage",
    ))
    data = await d.Document([d.Section([], properties=update)]).to_bytes(d.DocumentTemplate(template))
    borders = docx.Document(io.BytesIO(data)).sections[-1]._sectPr.find(f"{W}pgBorders")
    assert borders.get(f"{W}offsetFrom") == "text"
    assert borders.get(f"{W}display") == "notFirstPage"
    assert borders.get(f"{W}zOrder") == "back"
    assert borders.find(f"{W}left").get(f"{W}val") == "single"


@pytest.mark.parametrize("option", ["offset_from", "display", "z_order"])
async def test_invalid_page_border_option_is_rejected(option):
    props = d.SectionProperties(page_borders=d.PageBorders(**{option: "invalid"}))
    with pytest.raises(ValueError, match=f"unknown page border {option}"):
        await d.Document([d.Section([], properties=props)]).to_bytes()
