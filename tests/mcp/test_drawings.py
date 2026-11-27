"""Original drawing anchors, native byte parity, and occurrence-local assets."""

from copy import deepcopy
import json
from pathlib import Path
import subprocess
from zipfile import ZipFile

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_fields import opened
from test_ranges import inspect, saved
from test_stdio import anyio_backend, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio
IMAGES = Path(__file__).resolve().parents[1] / "fixtures" / "images"


def inline(anchor, kind, **fields):
    return dict(kind="inline", anchor=anchor, edit=dict(kind=kind, **fields))


def patch(anchor, property, donor):
    return inline(anchor, "drawing-property", property=property, inline=donor)


def chart(categories, values, width=100, height=75):
    return dict(kind="column-chart", categories=categories, values=values, width=width, height=height)


def shape(**properties):
    return dict(kind="shape", properties=properties)


def image(**fields):
    return dict(kind="image", bytes=list((IMAGES / "sample.jpg").read_bytes()), **fields)


def native(source, action, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_drawing_oracle")
    result = subprocess.run([oracle, source, action, output], check=True,
                            capture_output=True, timeout=20, cwd=source.parent)
    return json.loads(result.stdout)


async def body(client, document):
    rows = (await inspect(client, document, "body-inlines"))["body_inlines"]
    return {(row["body_index"], row["inline_index"]): row for row in rows}


async def preview(client, document, source, action, edits):
    result = await call(client, "preview_edits", document=document, edits=edits)
    output = source.parent / f"{action}.docx"
    expected = source.parent / f"native-{action}.docx"
    await saved(client, document, result, output)
    assert result["changes"]["inline_changes"] == native(source, action, expected)
    assert output.read_bytes() == expected.read_bytes()
    return output


async def test_drawing_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    validator = validator_for(schema)(schema)
    assert validator.is_valid(dict(document="doc", view=dict(kind="chart-series", anchor="a")))
    assert not validator.is_valid(dict(document="doc", view=dict(kind="chart-series", anchor="a", limit=1)))
    assert not validator.is_valid(dict(document="doc", view=dict(kind="chart-series")))
    schema = schemas["preview_edits"]
    validator = validator_for(schema)(schema)
    for edit in [inline("a", "remove-object"), inline("a", "drawing-description", value=None),
                 inline("a", "image-asset", bytes=[1, 2]),
                 patch("a", "chart-series", chart(["x"], [0])),
                 patch("a", "placement", image(transformation=dict(anchor=dict(
                     wrap="tight", wrap_polygon=[[0, 0], [0, 10], [10, 0]],
                 )))),
                 patch("a", "shape-text", shape(text="new", text_format=dict(bold=False)))]:
        assert validator.is_valid(dict(document="doc", edits=[edit]))
    for edit in [inline("a", "drawing-description"), inline("a", "image-asset", path="x.jpg"),
                 patch("a", "chartSeries", chart(["x"], [0])),
                 patch("a", "chart-series", chart([], [])),
                 patch("a", "placement", image(transformation=dict(anchor=dict(wrap="unknown")))),
                 patch("a", "shape-text", shape(fill=[1, 2, 256]))]:
        assert not validator.is_valid(dict(document="doc", edits=[edit]))


@pytest.mark.parametrize("large_axis", ["width", "height"])
async def test_drawing_extent_retains_native_integer_precision(launch, tmp_path, large_axis):
    source = tmp_path / "wide-extent.docx"
    document = docx.Document()
    paragraph = document.add_paragraph()
    paragraph.add_run().add_picture(str(IMAGES / "sample.png"))
    extent = paragraph._p.xpath(".//wp:extent")[0]
    small_axis = "height" if large_axis == "width" else "width"
    extents = {small_axis: "9007199254740991", large_axis: "9007199254740993"}
    extent.set("cx", extents["width"])
    extent.set("cy", extents["height"])
    document.save(source)
    direct = native(source, "extent", tmp_path / "unused.docx")
    assert direct == extents
    async with launch() as client:
        handle = await opened(client, source)
        row = (await body(client, handle))[0, 0]
        drawing = row["drawing"]
        assert drawing[f"{large_axis}_emu_exact"] == direct[large_axis]
        assert drawing[f"{small_axis}_emu"] == int(direct[small_axis])
        assert f"{small_axis}_emu_exact" not in drawing
        looked_up = await inspect(client, handle, "inline-at", inline_handle=row["inline_handle"])
        assert looked_up["inline"]["drawing"] == drawing


async def test_drawings_native_bytes_and_chart_fork(launch, tmp_path):
    source = tmp_path / "source.docx"
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.sections[0].header.paragraphs[0].text = "header stays"
    pictures = doc.add_paragraph()
    for _ in range(2):
        pictures.add_run().add_picture(str(IMAGES / "sample.png"))
    pictures._p.xpath(".//pic:spPr")[0].append(etree.Comment("preserve opaque markup"))
    pictures._p.xpath(".//wp:docPr")[1].set("descr", "clear me")
    for text in ("shape", "chart", "fraction"):
        doc.add_paragraph(text)
    doc.save(source)
    async with launch() as client:
        document = await opened(client, source)
        original = await body(client, document)
        seed = await preview(client, document, source, "seed", [
            inline(original[1, 0]["anchor"], "insert-after", inline=shape(text="original shape")),
            inline(original[2, 0]["anchor"], "insert-after", inline=chart(["A", "B"], [1, 2])),
            inline(original[3, 0]["anchor"], "insert-after", inline=dict(
                kind="fraction", numerator="1", denominator="2",
            )),
        ])
        shared = tmp_path / "shared.docx"
        peer = docx.Document(seed)
        runs = peer.paragraphs[2]._p
        sibling = deepcopy(runs[-1])
        sibling.xpath(".//wp:docPr")[0].set("id", "200")
        runs.append(sibling)
        peer.save(shared)
        document = await opened(client, shared)
        rows = await body(client, document)
        picture, picture_peer = rows[0, 0], rows[0, 1]
        chart_a, chart_b = rows[2, 1], rows[2, 2]
        assert picture["drawing"]["relationship_id"] == picture_peer["drawing"]["relationship_id"]
        assert chart_a["drawing"]["relationship_id"] == chart_b["drawing"]["relationship_id"]
        assert chart_a["drawing"]["width_emu"] == 100 * 12700
        assert (rows[3, 1]["numerator"], rows[3, 1]["denominator"]) == ("1", "2")
        assert await inspect(client, document, "chart-series", anchor=chart_a["anchor"]) == {
            "document": document, "categories": ["A", "B"], "values": [1, 2],
        }
        await failure(client, "inspect_document", "invalid_anchor", document=document,
                      view=dict(kind="chart-series", anchor=f"{document}/body-inline:2:999"))
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="chart-series", anchor=picture["anchor"]))
        foreign = await opened(client, shared)
        await failure(client, "inspect_document", "wrong_owner", document=foreign,
                      view=dict(kind="chart-series", anchor=chart_a["anchor"]))
        # Even recognized chart occurrences can have a series the native
        # reader refuses. That must not hide other drawing anchors.
        guarded = tmp_path / "opaque-series.docx"
        chart_namespace = "http://schemas.openxmlformats.org/drawingml/2006/chart"
        with ZipFile(guarded, "w") as output:
            for name, content in parts(shared).items():
                if name.endswith("/chart1.xml"):
                    root = etree.fromstring(content)
                    point = root.find(f".//{{{chart_namespace}}}strLit/{{{chart_namespace}}}pt")
                    assert point is not None
                    point.set("idx", "999")
                    content = etree.tostring(root)
                output.writestr(name, content)
        guarded_doc = await opened(client, guarded)
        guarded_rows = await body(client, guarded_doc)
        assert guarded_rows[0, 0]["drawing"]["kind"] == "image"
        assert guarded_rows[2, 1]["drawing"]["kind"] == "chart"
        await failure(client, "inspect_document", "inspection_failed", document=guarded_doc,
                      view=dict(kind="chart-series", anchor=guarded_rows[2, 1]["anchor"]))
        update = patch(chart_a["anchor"], "chart-series", chart(["New"], [0], 12, 10))
        for rejected in [
            [inline(picture["anchor"], "remove-object")],  # Would discard opaque markup.
            [patch(picture["anchor"], "size", shape())],  # Wrong donor kind.
            [patch(chart_a["anchor"], "chart-series", chart(["X", "Y"], [1]))],
            [update, update],
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=rejected)
        mixed = await preview(client, document, shared, "mixed", [
            inline(picture["anchor"], "image-asset", bytes=image()["bytes"]),
            inline(picture_peer["anchor"], "drawing-description", value=None),
            patch(rows[1, 1]["anchor"], "shape-geometry", shape(geometry="ellipse")),
            update,
            inline(rows[3, 1]["anchor"], "remove-object"),
        ])
        reopened = await opened(client, mixed)
        rows = await body(client, reopened)
        assert (3, 1) not in rows
        assert rows[0, 1]["drawing"]["description"] is None
        assert rows[1, 1]["drawing"]["geometry"] == "ellipse"
        assert rows[1, 1]["drawing"]["shape_text"] == "original shape"
        assert rows[2, 1]["drawing"]["width_emu"] == 100 * 12700  # Donor size is ignored.
        for index, categories, values in [(1, ["New"], [0]), (2, ["A", "B"], [1, 2])]:
            series = await inspect(client, reopened, "chart-series", anchor=rows[2, index]["anchor"])
            assert (series["categories"], series["values"]) == (categories, values)
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[update])

        # Patch independent groups on freshly inspected occurrences, retaining
        # IDs, relationships, and all other package parts.
        donor = image(
            transformation=dict(width=30, height=20, rotation=90, flip_horizontal=True,
                                flip_vertical=False, crop=dict(left=15),
                                anchor=dict(x=10, wrap="square", wrap_side="left", allow_overlap=False)),
            alt_text=dict(title="A title", description="Accessible"),
        )
        shape_donor = shape(fill=[1, 2, 3], outline=dict(width=3, color=[4, 5, 6]),
                            text="Revised\nshape", text_format=dict(bold=False, font_size=12,
                                                                   alignment="center"))
        current = mixed
        group_outputs = []
        for group, wire in [
            ("size", "size"), ("crop", "crop"), ("transform", "transform"),
            ("accessibility", "accessibility"), ("placement", "placement"),
            ("shapeFill", "shape-fill"), ("shapeOutline", "shape-outline"), ("shapeText", "shape-text"),
        ]:
            key, payload = ((1, 1), shape_donor) if group.startswith("shape") else ((0, 0), donor)
            output = await preview(client, reopened, current, group, [patch(rows[key]["anchor"], wire, payload)])
            before, after = parts(current), parts(output)
            assert before.keys() == after.keys()
            assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
            group_outputs.append(output)
            current, reopened = output, await opened(client, output)
            rows = await body(client, reopened)
        final = rows[0, 0]["drawing"]
        assert final["crop"] == [15000, 0, 0, 0]
        assert final["rotation"] == 5400000
        assert (final["flip_horizontal"], final["flip_vertical"]) == (True, False)
        assert (final["placement"], final["width_emu"], final["height_emu"]) == ("floating", 381000, 254000)
        assert (final["title"], final["description"]) == ("A title", "Accessible")
        assert rows[1, 1]["drawing"]["shape_text"] == "Revised\nshape"

    saved_doc = docx.Document(current)
    pictures = saved_doc.paragraphs[0]._p
    a, b = pictures.xpath(".//a:blip/@r:embed")
    assert saved_doc.part.related_parts[a].blob == (IMAGES / "sample.jpg").read_bytes()
    assert saved_doc.part.related_parts[b].blob == (IMAGES / "sample.png").read_bytes()
    assert pictures.xpath(".//comment()")[0].text == "preserve opaque markup"
    assert saved_doc.paragraphs[1]._p.xpath(".//a:spPr/a:solidFill/a:srgbClr/@val") == ["010203"]
    assert saved_doc.paragraphs[1]._p.xpath(".//a:ln/a:solidFill/a:srgbClr/@val") == ["040506"]
    a, b = saved_doc.paragraphs[2]._p.xpath(".//*[local-name()='chart']/@r:id")
    assert a != b
    before, after = parts(shared), parts(mixed)
    assert set(before) <= set(after)
    assert {name for name in before if before[name] != after[name]} == {
        "[Content_Types].xml", "word/document.xml", "word/_rels/document.xml.rels",
    }
    validate(source, seed, shared, mixed, *group_outputs)
