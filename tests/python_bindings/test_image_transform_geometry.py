"""Picture source rect and rotation do not change the stored media or target extent."""
import io
from pathlib import Path
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

NS = {"a": "http://schemas.openxmlformats.org/drawingml/2006/main",
      "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"}
PNG = Path(__file__).parents[1] / "fixtures/images/sample.png"


async def test_insets_and_rotation_are_quantized_and_media_are_shared():
    data = PNG.read_bytes()
    image = d.ImageRun(data, transformation={
        "width": 60, "height": 40, "rotation": lambda: -12.5,
        "crop": d.ImageCrop(left=10.25, top=5, right=20, bottom=15),
    })
    doc = d.Document([d.Section([d.Paragraph(children=[image, d.ImageRun(data)])])])
    snapshot = await doc.resolve()
    rendered = await snapshot.to_bytes()
    assert rendered == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(rendered)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        media = [name for name in package.namelist() if name.startswith("word/media/")]
        assert len(media) == 1
        assert package.read(media[0]) == data
    modified, original = root.xpath("//wp:inline", namespaces=NS)
    assert modified.xpath("wp:extent/@cx | wp:extent/@cy", namespaces=NS) == ["762000", "508000"]
    assert modified.xpath(".//a:xfrm/@rot", namespaces=NS) == ["-750000"]
    rect = modified.xpath(".//a:srcRect", namespaces=NS)[0]
    assert [rect.get(key) for key in ("l", "t", "r", "b")] == ["10250", "5000", "20000", "15000"]
    assert not original.xpath(".//a:srcRect | .//a:xfrm/@rot", namespaces=NS)


@pytest.mark.parametrize("transformation, message", [
    ({"rotation": 361}, "rotation"),
    ({"crop": d.ImageCrop(left=-1)}, "crop.left"),
    ({"crop": d.ImageCrop(left=49.9999, right=49.9999)}, "positive source rectangle"),
])
async def test_invalid_image_transform_rejects_nonrepresentable_geometry(transformation, message):
    image = d.ImageRun(PNG, transformation=transformation)
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


async def test_anchor_uses_signed_offsets_and_shared_picture_geometry():
    anchor = d.ImageAnchor(horizontal_reference="margin", x=lambda: -12.5,
                           y=20, relative_height=7, behind_text=True)
    image = d.ImageRun(PNG, transformation={"anchor": anchor, "width": 42, "rotation": 15})
    snapshot = await d.Document([d.Section([d.Paragraph(children=[image, d.ImageRun(PNG)])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert len([name for name in package.namelist() if name.startswith("word/media/")]) == 1
    anchored, = root.xpath("//wp:anchor", namespaces=NS)
    assert anchored.xpath("wp:positionH[@relativeFrom='margin']/wp:posOffset/text()", namespaces=NS) == ["-158750"]
    assert anchored.xpath("wp:positionV[@relativeFrom='page']/wp:posOffset/text()", namespaces=NS) == ["254000"]
    assert anchored.xpath("wp:wrapNone", namespaces=NS)
    assert (anchored.get("relativeHeight"), anchored.get("behindDoc")) == ("7", "1")
    assert anchored.xpath(".//a:xfrm/@rot", namespaces=NS) == ["900000"]
    assert len(root.xpath("//wp:inline", namespaces=NS)) == 1


async def test_anchor_rejects_offsets_outside_the_wire_range():
    image = d.ImageRun(PNG, transformation={"anchor": d.ImageAnchor(x=200000)})
    with pytest.raises(ValueError, match=r"anchor\.x"):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


@pytest.mark.parametrize("anchor, tag, attrs", [
    (d.ImageAnchor(wrap="square"), "wrapSquare", {"wrapText": "bothSides"}),
    (d.ImageAnchor(wrap="square", wrap_side="left", distance_left=lambda: 2.5),
     "wrapSquare", {"wrapText": "left", "distL": "31750"}),
    (d.ImageAnchor(wrap="topAndBottom", distance_top=0, distance_bottom=3),
     "wrapTopAndBottom", {"distT": "0", "distB": "38100"}),
])
async def test_anchor_wrap_requests(anchor, tag, attrs):
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    data = await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    wrap, = root.xpath(f"//wp:anchor/wp:{tag}", namespaces=NS)
    assert dict(wrap.attrib) == attrs


@pytest.mark.parametrize("anchor, error", [
    (d.ImageAnchor(wrap_side="right"), r"anchor\.wrap"),
    (d.ImageAnchor(wrap="topAndBottom", distance_right=1), r"anchor\.wrap"),
    (d.ImageAnchor(wrap="square", distance_top=-1), r"anchor\.distance_top"),
    (d.ImageAnchor(wrap="square", distance_bottom=400000), r"anchor\.distance_bottom"),
])
async def test_anchor_wrap_rejects_inapplicable_or_unrepresentable_values(anchor, error):
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    with pytest.raises(ValueError, match=error):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


@pytest.mark.parametrize("flags, expected", [
    ({}, ("0", "1", "1")),
    ({"locked": True, "layout_in_cell": False, "allow_overlap": lambda: False}, ("1", "0", "0")),
])
async def test_anchor_layout_flags(flags, expected):
    image = d.ImageRun(PNG, transformation={"anchor": d.ImageAnchor(**flags)})
    data = await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    anchor, = root.xpath("//wp:anchor", namespaces=NS)
    assert tuple(anchor.get(key) for key in ("locked", "layoutInCell", "allowOverlap")) == expected


async def test_anchor_alignment_uses_axis_specific_elements():
    anchor = d.ImageAnchor(horizontal_reference="margin", horizontal_alignment=lambda: "inside",
                           vertical_alignment="top")
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    data = await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert root.xpath("//wp:positionH[@relativeFrom='margin']/wp:align/text()", namespaces=NS) == ["inside"]
    assert root.xpath("//wp:positionV[@relativeFrom='page']/wp:align/text()", namespaces=NS) == ["top"]
    assert not root.xpath("//wp:anchor//wp:posOffset", namespaces=NS)


@pytest.mark.parametrize("anchor, message", [
    (d.ImageAnchor(horizontal_alignment="center", x=1), "nonzero offset"),
    (d.ImageAnchor(vertical_alignment="left"), "vertical alignment"),
])
async def test_anchor_alignment_rejects_invalid_axis_requests(anchor, message):
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


async def test_anchor_serializes_axis_specific_reference_frames():
    anchor = d.ImageAnchor(horizontal_reference="character", vertical_reference="line")
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    data = await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert root.xpath("//wp:positionH/@relativeFrom", namespaces=NS) == ["character"]
    assert root.xpath("//wp:positionV/@relativeFrom", namespaces=NS) == ["line"]


@pytest.mark.parametrize("anchor, message", [
    (d.ImageAnchor(horizontal_reference="line"), "horizontal picture reference frame"),
    (d.ImageAnchor(vertical_reference="column"), "vertical picture reference frame"),
])
async def test_anchor_rejects_reference_frames_on_the_wrong_axis(anchor, message):
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


@pytest.mark.parametrize("mode, tag", [("tight", "wrapTight"), ("through", "wrapThrough")])
async def test_polygon_wrapping_retains_vertices_and_distances(mode, tag):
    anchor = d.ImageAnchor(wrap=mode, distance_right=2, wrap_side="right",
                           wrap_polygon=lambda: [(0, 0), (21600, 0), (0, 21600)])
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    data = await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    wrap, = root.xpath(f"//wp:{tag}", namespaces=NS)
    assert (wrap.get("wrapText"), wrap.get("distR")) == ("right", "25400")
    polygon, = wrap.xpath("wp:wrapPolygon", namespaces=NS)
    assert polygon.get("edited") == "1"
    assert [(point.get("x"), point.get("y")) for point in polygon] == [
        ("0", "0"), ("21600", "0"), ("0", "21600"),
    ]


@pytest.mark.parametrize("anchor", [
    d.ImageAnchor(wrap="tight", wrap_polygon=[(0, 0), (1, 1)]),
    d.ImageAnchor(wrap="through", wrap_polygon=[(0, 0), (1, 1), (2, 2)]),
    d.ImageAnchor(wrap="square", wrap_polygon=[(0, 0), (1, 0), (0, 1)]),
])
async def test_polygon_wrapping_rejects_invalid_geometry_or_mode(anchor):
    image = d.ImageRun(PNG, transformation={"anchor": anchor})
    with pytest.raises(ValueError, match=r"anchor\.wrap_polygon"):
        await d.Document([d.Section([d.Paragraph(children=[image])])]).to_bytes()


async def test_picture_flips_preserve_explicit_false_and_omission():
    flipped = d.ImageRun(PNG, transformation={"flip_horizontal": lambda: True, "flip_vertical": False})
    data = await d.Document([d.Section([d.Paragraph(children=[flipped, d.ImageRun(PNG)])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert len([name for name in package.namelist() if name.startswith("word/media/")]) == 1
    changed, original = root.xpath("//a:xfrm", namespaces=NS)
    assert (changed.get("flipH"), changed.get("flipV")) == ("1", "0")
    assert (original.get("flipH"), original.get("flipV")) == (None, None)
