"""Independent readback for inline DrawingML shapes."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

NS = {
    "lc": "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas",
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
}


async def test_shapes_serialize_geometry_fill_and_unique_drawing_ids():
    shapes = [d.ShapeRun(20, 30), d.ShapeRun(width=lambda: 42, height=24,
                                           geometry="ellipse", fill=(16, 128, 240))]
    snapshot = await d.Document([d.Section([d.Paragraph(children=shapes)])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert not any(name.startswith("word/media/") for name in package.namelist())
    canvases = root.xpath("//lc:lockedCanvas", namespaces=NS)
    assert [canvas.xpath("a:sp/a:spPr/a:prstGeom/@prst", namespaces=NS) for canvas in canvases] == [
        ["rect"], ["ellipse"],
    ]
    assert canvases[1].xpath("a:sp/a:spPr/a:solidFill/a:srgbClr/@val", namespaces=NS) == ["1080F0"]
    assert root.xpath("//wp:inline/wp:extent/@cx", namespaces=NS) == ["254000", "533400"]
    assert len(set(root.xpath("//wp:docPr/@id", namespaces=NS))) == 2


async def test_shape_rejects_negative_size_and_unsupported_story():
    invalid = d.Paragraph(children=[d.ShapeRun(width=-1)])
    with pytest.raises(ValueError, match=r"shape\.width"):
        await d.Document([d.Section([invalid])]).to_bytes()
    header = d.Header([d.Paragraph(children=[d.ShapeRun()])])
    with pytest.raises(ValueError, match="only in body paragraphs"):
        await d.Document([d.Section([], headers={"default": header})]).to_bytes()


async def test_shape_text_uses_drawingml_paragraphs_and_preserves_blank_lines():
    shape = d.ShapeRun(text=lambda: " Café & tea\r\n\r\n  End ")
    data = await d.Document([d.Section([d.Paragraph(children=[shape])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    paragraphs = root.xpath("//a:txBody/a:p", namespaces=NS)
    assert [p.xpath("a:r/a:t/text()", namespaces=NS) for p in paragraphs] == [
        [" Café & tea"], [], ["  End "],
    ]


async def test_shape_outline_and_imported_nonvisual_ids_are_preserved(tmp_path):
    from test_notes import rewrite

    original = await d.Document([d.Section([d.Paragraph(children=[d.ShapeRun()])])]).to_bytes()
    template = tmp_path / "existing-shape.docx"

    def change_inner_id(root):
        root.xpath("//a:sp/a:nvSpPr/a:cNvPr", namespaces=NS)[0].set("id", "900")

    template.write_bytes(rewrite(original, "word/document.xml", change_inner_id))
    added = d.ShapeRun(outline=d.ShapeOutline(width=lambda: 1.5, color=(16, 48, 32)))
    data = await d.Document([d.Section([d.Paragraph(children=[added])])]).to_bytes(
        d.DocumentTemplate(template),
    )
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert root.xpath("//a:sp/a:nvSpPr/a:cNvPr/@id", namespaces=NS) == ["900", "902"]
    assert root.xpath("//a:spPr/a:ln/@w", namespaces=NS) == ["19050"]
    assert root.xpath("//a:spPr/a:ln/a:solidFill/a:srgbClr/@val", namespaces=NS) == ["103020"]


async def test_shape_outline_rejects_out_of_range_line_width():
    shape = d.ShapeRun(outline=d.ShapeOutline(1585, (0, 0, 0)))
    with pytest.raises(ValueError, match=r"shape\.outline\.width"):
        await d.Document([d.Section([d.Paragraph(children=[shape])])]).to_bytes()
