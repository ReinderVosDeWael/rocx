"""Six logical header/footer slots, qualified with native shared and local previews."""

import subprocess

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_stdio import (
    PNG, anyio_backend, assert_tool_failure, call, digest, executable, failure, launch,
    oracle, parts, pytestmark, validate,
)


SLOTS = [(kind, variant) for kind in ("header", "footer")
         for variant in ("default", "first", "even")]


def slot(section, kind, variant):
    prefix = {"default": "", "first": "first_page_", "even": "even_page_"}[variant]
    return getattr(section, f"{prefix}{kind}")


def make_source(path):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.settings.odd_and_even_pages_header_footer = True
    fixture.add_paragraph("First section")
    first = fixture.sections[0]
    first.different_first_page_header_footer = True
    for kind, variant in SLOTS:
        slot(first, kind, variant).paragraphs[0].add_run(f"{kind} {variant}")
    fixture.add_section()
    fixture.add_paragraph("Second section")
    fixture.save(path)


def image_declaration():
    return {
        "bytes": list(PNG.read_bytes()),
        "source_name": "source-logo.png",
        "transformation": {
            "width": 84, "height": 42,
            "crop": {"left": 10, "top": 5, "right": 15, "bottom": 20},
            "rotation": 30.5, "flip_horizontal": True, "flip_vertical": False,
        },
        "alt_text": {
            "name": "Story logo", "title": "Story logo title",
            "description": "Story logo description",
        },
    }


def assert_image_readback(story):
    inline, = story._element.xpath(".//wp:inline")
    extent = inline.find(qn("wp:extent"))
    assert dict(extent.attrib) == {"cx": "1066800", "cy": "533400"}
    doc_properties = inline.find(qn("wp:docPr"))
    assert [doc_properties.get(key) for key in ("name", "title", "descr")] == [
        "Story logo", "Story logo title", "Story logo description",
    ]
    picture_properties, = story._element.xpath(".//pic:cNvPr")
    assert picture_properties.get("name") == "source-logo.png"
    transform, = story._element.xpath(".//a:xfrm")
    assert dict(transform.attrib) == {"rot": "1830000", "flipH": "1", "flipV": "0"}
    crop, = story._element.xpath(".//a:srcRect")
    assert dict(crop.attrib) == {"l": "10000", "t": "5000", "r": "15000", "b": "20000"}
    blip, = story._element.xpath(".//a:blip")
    image_part = story.part.related_parts[blip.get(qn("r:embed"))]
    assert image_part.content_type == "image/png" and image_part.blob == PNG.read_bytes()


@pytest.mark.parametrize(("kind", "variant"), SLOTS)
@pytest.mark.parametrize("mode", ["shared", "section-local"])
@pytest.mark.parametrize("rich", [False, True], ids=["shorthand", "declaration"])
async def test_story_image_all_slots_from_paged_anchors(launch, tmp_path, kind, variant, mode, rich):
    source = tmp_path / "source.docx"
    output = tmp_path / "edited.docx"
    make_source(source)
    native = tmp_path / "native.docx"
    if rich:
        binary = executable("ROCX_MCP_ORACLE").with_name("mcp_story_image_oracle")
        subprocess.run([binary, native, source, PNG, f"{kind}:{variant}", mode],
                       check=True, cwd=tmp_path, timeout=20)
        expected = native.read_bytes()
        payload = {"image": image_declaration()}
    else:
        expected = oracle(native, "story-image", source, PNG, f"{kind}:{variant}", mode)
        payload = {"bytes": list(PNG.read_bytes())}

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document, view={"kind": "stories"})
        rows = seen["stories"]
        selected, = [row for row in rows if row["section_index"] == 1 and
                     (row["kind"], row["variant"]) == (kind, variant)]
        assert selected["inherited"]
        staged = await call(client, "preview_edits", document=document, edits=[{
            "kind": "header-image", "anchor": selected["anchor"],
            "mode": mode, **payload,
        }])
        change, = staged["changes"]["header_images"]
        assert change["source_part"] == selected["owner"]["part_name"] and change["mode"] == mode
        assert change["affected_sections"] == ([0, 1] if mode == "shared" else [1])
        assert (change["output_part"] == change["source_part"]) == (mode == "shared")
        assert staged["output_snapshot"] == digest(expected)
        await call(client, "save_copy", document=document, preview=staged["preview"],
                   output=output.name)
    assert output.read_bytes() == expected
    before, after = parts(source), parts(output)
    for row in rows:
        name = row["owner"]["part_name"]
        if name != selected["owner"]["part_name"]:
            assert before[name] == after[name]
    if mode == "section-local":
        name = selected["owner"]["part_name"]
        assert before[name] == after[name]

    readback = docx.Document(output)
    for index, section in enumerate(readback.sections):
        for other_kind, other_variant in SLOTS:
            story = slot(section, other_kind, other_variant)
            has_image = bool(story._element.xpath(".//w:drawing"))
            expected_image = (other_kind, other_variant) == (kind, variant) and (
                index == 1 or mode == "shared"
            )
            assert has_image == expected_image
            if rich and expected_image:
                assert_image_readback(story)
    validate(output)


async def test_story_image_schema_and_inline_only_boundary(launch, tmp_path):
    source = tmp_path / "source.docx"
    make_source(source)
    async with launch() as client:
        tools = (await client.list_tools()).tools
        schema = next(tool.inputSchema for tool in tools if tool.name == "preview_edits")
        validator = validator_for(schema)(schema)
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document, view={"kind": "stories"})
        anchor = seen["stories"][0]["anchor"]
        edit = {"kind": "header-image", "anchor": anchor, "mode": "shared"}
        image = image_declaration()

        def request(payload):
            return {"document": document, "edits": [{**edit, **payload}]}

        for payload in ({"bytes": image["bytes"]}, {"image": image}):
            validator.validate(request(payload))
        for payload in ({}, {"bytes": image["bytes"], "image": image}):
            arguments = request(payload)
            assert not validator.is_valid(arguments)
            result = await client.call_tool("preview_edits", arguments)
            assert result.isError
        for payload in ({"bytes": None}, {"image": None}, {"image": {**image, "typo": True}}):
            arguments = request(payload)
            assert not validator.is_valid(arguments)
            await failure(client, "preview_edits", "invalid_input", **arguments)

        floating = {**image, "transformation": {"anchor": {}}}
        result = await client.call_tool("preview_edits", request({"image": floating}))
        assert_tool_failure(result, "rejected_batch")
        assert "header image insertion requires an inline image" in result.structuredContent["message"]
        await call(client, "preview_edits", **request({"image": image}))


async def test_story_image_rejects_missing_slot_and_physical_owner(launch, tmp_path):
    source = tmp_path / "source.docx"
    fixture = docx.Document()
    fixture.add_paragraph("Body")
    fixture.sections[0].footer.paragraphs[0].add_run("Footer")
    fixture.save(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document, view={"kind": "stories"})
        rows = seen["stories"]
        missing = next(row for row in rows if row["owner"] is None)
        present = next(row for row in rows if row["owner"] is not None)
        edit = {"kind": "header-image", "mode": "shared", "bytes": list(PNG.read_bytes())}
        for wrong in (missing["anchor"], present["owner"]["anchor"]):
            await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                          edits=[{**edit, "anchor": wrong}])
        await call(client, "preview_edits", document=document,
                   edits=[{**edit, "anchor": present["anchor"]}])
