"""Shape text formatting authors DrawingML, not paragraph/run WordprocessingML."""
import io
import zipfile

from lxml import etree
import pytest

from rocx import declarative as d

NS = {"a": "http://schemas.openxmlformats.org/drawingml/2006/main"}


async def render(shape: d.ShapeRun) -> etree._Element:
    """Read the actual package using an independent XML implementation."""
    snapshot = await d.Document([d.Section([d.Paragraph(children=[shape])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/document.xml"))


async def test_styled_multiline_shape_preserves_false_and_blank_lines() -> None:
    root = await render(d.ShapeRun(text="First\n\n Last ", text_format=d.ShapeTextFormat(
        font_size=lambda: 14.25, bold=False, color=(16, 48, 32),
        alignment="center", vertical_alignment="bottom")))
    body = root.find(".//a:txBody", NS)
    assert body.find("a:bodyPr", NS).get("anchor") == "b"
    paragraphs = body.findall("a:p", NS)
    assert len(paragraphs) == 3
    assert [p.find("a:pPr", NS).get("algn") for p in paragraphs] == ["ctr"] * 3
    assert [p.xpath("a:r/a:t/text()", namespaces=NS) for p in paragraphs] == [["First"], [], [" Last "]]
    for properties in body.xpath("a:p/a:r/a:rPr | a:p/a:endParaRPr", namespaces=NS):
        assert properties.get("sz") == "1425"
        assert properties.get("b") == "0"
        assert properties.find("a:solidFill/a:srgbClr", NS).get("val") == "103020"
    assert len(body.xpath("a:p/a:r/a:rPr", namespaces=NS)) == 2
    assert len(body.xpath("a:p/a:endParaRPr", namespaces=NS)) == 3


@pytest.mark.parametrize("horizontal, vertical, expected_h, expected_v", [
    ("left", "top", "l", "t"), ("center", "center", "ctr", "ctr"), ("right", "bottom", "r", "b")])
async def test_all_shape_text_alignments(horizontal, vertical, expected_h, expected_v) -> None:
    root = await render(d.ShapeRun(text="aligned", text_format=d.ShapeTextFormat(
        alignment=horizontal, vertical_alignment=vertical, bold=True)))
    assert root.xpath("//a:bodyPr/@anchor", namespaces=NS) == [expected_v]
    assert root.xpath("//a:pPr/@algn", namespaces=NS) == [expected_h]
    assert root.xpath("//a:rPr/@b", namespaces=NS) == ["1"]
    assert not root.xpath("//a:rPr/@sz | //a:rPr/a:solidFill", namespaces=NS)


async def test_omitted_and_empty_shape_text_format_preserve_existing_bytes() -> None:
    async def data(fmt):
        return await d.Document([d.Section([d.Paragraph(children=[d.ShapeRun(text="plain", text_format=fmt)])])],
            timestamp="2026-11-21T00:00:00Z").to_bytes()
    assert await data(None) == await data(d.ShapeTextFormat())
    root = await render(d.ShapeRun(text_format=d.ShapeTextFormat(bold=True)))
    assert not root.xpath("//a:txBody", namespaces=NS)


@pytest.mark.parametrize("size, expected", [(1, "100"), (4000, "400000"), (12.345, "1235")])
async def test_shape_font_size_limits_and_rounding(size, expected) -> None:
    root = await render(d.ShapeRun(text="size", text_format=d.ShapeTextFormat(font_size=size)))
    assert root.xpath("//a:rPr/@sz", namespaces=NS) == [expected]


@pytest.mark.parametrize("size", [0, 0.999, 4000.001, -1, float("nan"), float("inf")])
async def test_invalid_shape_font_size_is_rejected_even_without_text(size) -> None:
    with pytest.raises(ValueError, match=r"shape.text_format.font_size"):
        await render(d.ShapeRun(text_format=d.ShapeTextFormat(font_size=size)))


@pytest.mark.parametrize("field, token", [("alignment", "justify"), ("vertical_alignment", "both")])
async def test_unsupported_alignment_tokens_are_rejected(field, token) -> None:
    with pytest.raises(ValueError, match="shape text"):
        await render(d.ShapeRun(text="bad", text_format=d.ShapeTextFormat(**{field: token})))
