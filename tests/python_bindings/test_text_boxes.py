"""Standalone DrawingML text boxes use a direct canvas text shape."""
import io
import zipfile

from lxml import etree
import pytest

from rocx import declarative as d

NS = {"a": "http://schemas.openxmlformats.org/drawingml/2006/main",
      "lc": "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas",
      "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"}


async def test_text_boxes_use_their_own_transform_and_preserve_text_styling() -> None:
    box = d.ShapeRun(width=lambda: 144, height=72, geometry="text_box", text="First\n\n Last ",
        text_format=d.ShapeTextFormat(font="Arial", font_size=12, bold=False, italic=True, alignment="right"))
    snapshot = await d.Document([d.Section([d.Paragraph(children=[box, d.ShapeRun()])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert not any(name.startswith("word/media/") for name in package.namelist())
    canvases = root.xpath("//lc:lockedCanvas", namespaces=NS)
    node = canvases[0].find("a:txSp", NS)
    assert node is not None
    assert canvases[0].find("a:sp", NS) is None
    assert [etree.QName(child).localname for child in node] == ["txBody", "xfrm"]
    assert node.find("a:xfrm/a:off", NS).attrib == {"x": "0", "y": "0"}
    assert node.find("a:xfrm/a:ext", NS).attrib == {"cx": "1828800", "cy": "914400"}
    assert node.xpath("a:txBody/a:p/a:r/a:t/text()", namespaces=NS) == ["First", " Last "]
    assert node.xpath("a:txBody/a:p/a:pPr/@algn", namespaces=NS) == ["r"] * 3
    assert node.xpath("a:txBody/a:p/a:endParaRPr/@i", namespaces=NS) == ["1"] * 3
    assert node.xpath("a:txBody/a:p/a:r/a:rPr/a:latin/@typeface", namespaces=NS) == ["Arial"] * 2
    ids = root.xpath("//wp:docPr/@id | //a:cNvPr/@id", namespaces=NS)
    assert len(ids) == len(set(ids))
    assert canvases[1].find("a:sp", NS) is not None


async def test_empty_text_box_still_has_one_empty_drawingml_paragraph() -> None:
    data = await d.Document([d.Section([d.Paragraph(children=[d.ShapeRun(geometry="text_box")])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert len(root.xpath("//lc:lockedCanvas/a:txSp/a:txBody/a:p", namespaces=NS)) == 1
    assert not root.xpath("//lc:lockedCanvas/a:txSp/a:txBody//a:r | //a:useSpRect", namespaces=NS)


@pytest.mark.parametrize("property", ["fill", "outline"])
async def test_text_box_rejects_unrepresentable_geometric_properties(property) -> None:
    value = (16, 48, 32) if property == "fill" else d.ShapeOutline(1, (16, 48, 32))
    with pytest.raises(ValueError, match="text boxes.*fill or outline"):
        await d.Document([d.Section([d.Paragraph(children=[d.ShapeRun(geometry="text_box", **{property: value})])])]).to_bytes()


async def test_text_boxes_keep_shape_story_restrictions() -> None:
    header = d.Header([d.Paragraph(children=[d.ShapeRun(geometry="text_box", text="Header")])])
    with pytest.raises(ValueError, match="only in body paragraphs"):
        await d.Document([d.Section([], headers={"default": header})]).to_bytes()
