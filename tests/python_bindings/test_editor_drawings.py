"""Occurrence-local drawing edits on an independently authored shared asset."""
import io
from pathlib import Path
from zipfile import ZipFile

import docx
from lxml import etree
import pytest
from rocx import declarative as d
from rocx.editor import HeaderImageEdit, ImportedDocument, InlineEdit

IMAGES = Path(__file__).parents[1] / "fixtures/images"


def package():
    doc = docx.Document()
    p = doc.add_paragraph()
    for _ in range(2):
        p.add_run().add_picture(str(IMAGES / "sample.png"))
    pic = p._p.xpath(".//pic:spPr")[0]
    pic.append(etree.Comment("preserve opaque shape markup"))
    p.add_run("after")
    doc.sections[0].header.paragraphs[0].text = "Header"
    stream = io.BytesIO()
    doc.save(stream)
    return stream.getvalue()


async def test_shared_asset_change_preserves_peer_and_opaque_markup():
    original = package()
    source = ImportedDocument.open(original)
    first, second, _ = source.body[0].inline_structure
    assert first.drawing.kind == "image" and first.drawing.placement == "inline"
    assert first.drawing.relationship_id == second.drawing.relationship_id
    jpeg = await d.ImageRun((IMAGES / "sample.jpg").read_bytes()).resolve()
    change = InlineEdit.set_image_asset(first.anchor, jpeg)
    for edits in ([change, change], [change, InlineEdit.remove_object(second.anchor), InlineEdit.remove_object(first.anchor)]):
        with pytest.raises(ValueError, match="duplicate|conflict"):
            source.preview([], inline_edits=edits)
    with pytest.raises(ValueError, match="opaque"):
        source.preview([], inline_edits=[InlineEdit.remove_object(first.anchor)])
    result = source.preview([], inline_edits=[change]).to_docx()
    opened = ImportedDocument.open(result)
    with pytest.raises(ValueError, match="snapshot"):
        opened.preview([], inline_edits=[change])
    saved = docx.Document(io.BytesIO(result))
    a, b = saved.paragraphs[0]._p.xpath(".//a:blip/@r:embed")
    assert saved.part.related_parts[a].blob == (IMAGES / "sample.jpg").read_bytes()
    assert saved.part.related_parts[b].blob == (IMAGES / "sample.png").read_bytes()
    assert saved.paragraphs[0]._p.xpath(".//comment()")[0].text == "preserve opaque shape markup"
    with ZipFile(io.BytesIO(original)) as old, ZipFile(io.BytesIO(result)) as new:
        for name in old.namelist():
            if name not in {"word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"}:
                assert old.read(name) == new.read(name)
    first = opened.body[0].inline_structure[0]
    assert opened.preview([], inline_edits=[InlineEdit.set_image_asset(first.anchor, jpeg)]).inline_changes == []
    desc = opened.preview([], inline_edits=[InlineEdit.set_drawing_description(first.anchor, "Separate alt")])
    opened = ImportedDocument.open(desc.to_docx())
    first = opened.body[0].inline_structure[0]
    assert first.drawing.description == "Separate alt"
    clear = opened.preview([], inline_edits=[InlineEdit.set_drawing_description(first.anchor, None)])
    assert ImportedDocument.open(clear.to_docx()).body[0].inline_structure[0].drawing.description is None


async def test_patch_groups_preserve_object_identity_and_package_parts():
    data = package()
    before = docx.Document(io.BytesIO(data)).paragraphs[0]._p.xpath(".//wp:docPr/@id")
    payload = await d.ImageRun((IMAGES / "sample.jpg").read_bytes(), transformation={
        "width": 30, "height": 20, "rotation": 90, "flip_horizontal": True, "crop": d.ImageCrop(left=15),
        "anchor": d.ImageAnchor(x=10, wrap="square")}, alt_text={"description": "Accessible"}).resolve()
    for group in ("size", "crop", "transform", "accessibility", "placement"):
        imported = ImportedDocument.open(data)
        run = imported.body[0].inline_structure[0]
        preview = imported.preview([], inline_edits=[InlineEdit.patch_drawing(run.anchor, group, payload)])
        assert preview.inline_changes == [(0, 0)]
        changed = preview.to_docx()
        with ZipFile(io.BytesIO(data)) as old, ZipFile(io.BytesIO(changed)) as new:
            assert set(old.namelist()) == set(new.namelist())
            for name in old.namelist():
                if name != "word/document.xml":
                    assert old.read(name) == new.read(name)
        data = changed
    saved = docx.Document(io.BytesIO(data)).paragraphs[0]._p
    info = ImportedDocument.open(data).body[0].inline_structure[0].drawing
    assert list(info.crop) == [15000, 0, 0, 0] and info.rotation == 5400000
    assert info.flip_horizontal and info.description == "Accessible" and info.placement == "floating"
    assert saved.xpath(".//wp:docPr/@id") == before
    assert saved.xpath(".//wp:anchor/wp:extent/@cx") == [str(30 * 12700)]
    assert saved.xpath(".//wp:anchor//a:srcRect/@l") == ["15000"]
    assert saved.xpath(".//wp:anchor//a:xfrm/@rot") == ["5400000"]
    assert saved.xpath(".//wp:anchor//a:xfrm/@flipH") == ["1"]
    assert saved.xpath(".//wp:anchor/wp:docPr/@descr") == ["Accessible"]
    assert saved.xpath(".//wp:anchor//comment()")[0].text == "preserve opaque shape markup"


async def test_body_and_header_drawings_share_id_ceiling():
    imported = ImportedDocument.open(package())
    text = imported.body[0].inline_structure[-1]
    png = await d.ImageRun((IMAGES / "sample.png").read_bytes()).resolve()
    preview = imported.preview([], inline_edits=[InlineEdit.insert_after(text.anchor, png)],
                               header_images=[HeaderImageEdit(imported.default_headers[0].anchor, png, mode="shared")])
    doc = docx.Document(io.BytesIO(preview.to_docx()))
    ids = doc.element.xpath(".//wp:docPr/@id") + doc.sections[0].header._element.xpath(".//wp:docPr/@id")
    assert len(ids) == len(set(ids)) == 4


async def test_shape_groups_and_guarded_fraction_removal():
    imported = ImportedDocument.open(package())
    first, second, text = imported.body[0].inline_structure
    shape = await d.ShapeRun(40, 20, text="Old").resolve()
    frac = await d.InlineFraction("1", "2").resolve()
    data = imported.preview([], inline_edits=[InlineEdit.insert_after(text.anchor, shape),
        InlineEdit.insert_after(second.anchor, frac)]).to_docx()
    payload = await d.ShapeRun(15, 10, fill=(1, 2, 3), outline=d.ShapeOutline(2, (4, 5, 6)), text="New\n\nText").resolve()
    for prop in ("shapeFill", "shapeOutline", "shapeText"):
        imported = ImportedDocument.open(data)
        node = imported.body[0].inline_structure[-1]
        data = imported.preview([], inline_edits=[InlineEdit.patch_drawing(node.anchor, prop, payload)]).to_docx()
    source = ImportedDocument.open(data)
    info = source.body[0].inline_structure[-1].drawing
    assert info.geometry == "rect" and info.shape_text == "New\n\nText"
    assert info.width_emu == 40 * 12700
    saved = docx.Document(io.BytesIO(data)).paragraphs[0]._p
    assert saved.xpath(".//a:sp/a:spPr/a:ln/a:solidFill/a:srgbClr/@val") == ["040506"]
    assert saved.xpath(".//a:sp/a:spPr/a:solidFill/a:srgbClr/@val") == ["010203"]
    fraction = source.body[0].inline_structure[2]
    assert (fraction.numerator, fraction.denominator) == ("1", "2")
    clean = source.preview([], inline_edits=[InlineEdit.remove_object(fraction.anchor)])
    assert not docx.Document(io.BytesIO(clean.to_docx())).paragraphs[0]._p.xpath(".//m:oMath")


async def test_chart_data_forks_a_shared_literal_series():
    from copy import deepcopy
    source = ImportedDocument.open(package())
    chart = await d.ColumnChartRun(100, 75, ["A", "B"], [1, 2]).resolve()
    edited = source.preview([], inline_edits=[InlineEdit.insert_after(source.body[0].inline_structure[-1].anchor, chart)])
    independent = docx.Document(io.BytesIO(edited.to_docx()))
    sibling = deepcopy(independent.paragraphs[0]._p[-1])
    sibling.xpath(".//wp:docPr")[0].set("id", "200")
    independent.paragraphs[0]._p.append(sibling)
    buf = io.BytesIO()
    independent.save(buf)
    old = ImportedDocument.open(buf.getvalue())
    first, second = old.body[0].inline_structure[-2:]
    assert first.drawing.relationship_id == second.drawing.relationship_id
    bad = await d.ColumnChartRun(12, 10, ["X"], [0]).resolve()
    with pytest.raises(ValueError, match="point count"):
        old.preview([], inline_edits=[InlineEdit.patch_drawing(first.anchor, "chartSeries", bad)])
    new = await d.ColumnChartRun(12, 10, ["X", "Y"], [1, 3]).resolve()
    out = old.preview([], inline_edits=[InlineEdit.patch_drawing(first.anchor, "chartSeries", new)]).to_docx()
    reopened = ImportedDocument.open(out)
    a, b = reopened.body[0].inline_structure[-2:]
    assert a.drawing.relationship_id != b.drawing.relationship_id
    assert a.drawing.width_emu == first.drawing.width_emu
    assert reopened.chart_series(a.anchor) == (["X", "Y"], [1., 3.])
    assert reopened.chart_series(b.anchor) == (["A", "B"], [1., 2.])
    with ZipFile(io.BytesIO(buf.getvalue())) as src, ZipFile(io.BytesIO(out)) as dst:
        assert all(dst.read(name) == src.read(name) for name in src.namelist()
                   if name not in {"word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"})
