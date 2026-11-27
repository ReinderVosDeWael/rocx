"""Flat nested field transport through the installed stdio client."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, assert_tool_failure, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio

MAIN = "word/document.xml"
FIELD_DEPTH_CAP = 256


def declaration(depth):
    nodes = [
        {
            "instruction": {"kind": "raw", "text": f" LEVEL {index} "},
            "children": [{"kind": "field", "node": index + 1}],
        }
        for index in range(depth)
    ]
    nodes[0]["dirty"] = False
    nodes[-1]["locked"] = True
    nodes[-1]["children"] = [
        {"kind": "text", "text": "cached & <value>", "format": {"bold": True}},
    ]
    return {"kind": "field-tree", "nodes": nodes}


async def open_fixture(client, source):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("before")
    fixture.save(source)
    document = (await call(client, "open_document", path=source.name))["document"]
    page = await call(client, "inspect_document", document=document, view={"kind": "body-inlines"})
    run, = page["body_inlines"]
    return document, run["anchor"]


def insertion(anchor, inline):
    return [{
        "kind": "inline",
        "anchor": anchor,
        "edit": {"kind": "insert-after", "inline": inline},
    }]


@pytest.mark.parametrize("depth", [64, 200, 248])
async def test_flat_field_tree_matches_native(launch, tmp_path, depth):
    source, output, native = (tmp_path / name for name in ("source.docx", "out.docx", "native.docx"))
    async with launch() as client:
        document, anchor = await open_fixture(client, source)
        original = source.read_bytes()
        oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_field_tree_oracle")
        subprocess.run([oracle, source, native, str(depth)], check=True, cwd=tmp_path, timeout=20)

        request = dict(document=document, edits=insertion(anchor, declaration(depth)))
        tools = (await client.list_tools()).tools
        schema = next(tool.inputSchema for tool in tools if tool.name == "preview_edits")
        validator_for(schema)(schema).validate(request)
        draft = await call(client, "preview_edits", **request)
        await call(client, "save_copy", document=document, preview=draft["preview"], output=output.name)
        reopened = (await call(client, "open_document", path=output.name))["document"]
        if depth == 64:
            page = await call(client, "inspect_document", document=reopened, view={"kind": "fields"})
            field, = page["fields"]
            assert field["instruction"] == " LEVEL 0 "
            assert not field["complex"]

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {MAIN}
    xml = ElementTree.fromstring(after[MAIN])
    assert sum(1 for _ in xml.iter(qn("w:fldSimple"))) == depth
    paragraph = xml.find(f"./{qn('w:body')}/{qn('w:p')}")
    assert paragraph.find(f"./{qn('w:r')}/{qn('w:t')}").text == "before"
    node = paragraph
    for index in range(depth):
        node, = node.findall(qn("w:fldSimple"))
        assert node.get(qn("w:instr")) == f" LEVEL {index} "
        assert node.get(qn("w:dirty")) == ("0" if index == 0 else None)
        assert node.get(qn("w:fldLock")) == ("1" if index == depth - 1 else None)
    run, = node
    assert run.find(qn("w:t")).text == "cached & <value>"
    assert run.find(f"./{qn('w:rPr')}/{qn('w:b')}") is not None
    validate(source, output)


async def test_flat_field_tree_rejects_invalid_nodes_and_allows_retry(launch, tmp_path):
    source = tmp_path / "source.docx"
    page = {"instruction": {"kind": "page"}}
    tab = {"kind": "tab"}

    def refs(*indices):
        return {**page, "children": [{"kind": "field", "node": index} for index in indices]}

    crowded_root = refs(1)
    crowded_root["children"].extend([tab] * 512)
    async with launch() as client:
        document, anchor = await open_fixture(client, source)
        for nodes in [
            [],
            [refs(0)],  # Cycle.
            [refs(1), refs(0)],  # Back edge.
            [refs(1)],  # Out of bounds.
            [page, page],  # Unreachable node.
            [refs(1, 2), refs(2), page],  # Shared subtree.
            [page] * 257,
            [crowded_root, {**page, "children": [tab] * 512}],  # Aggregate child-entry limit.
            [{"instruction": {"kind": "raw", "text": "\0"}}],  # Native XML validation.
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=insertion(anchor, {"kind": "field-tree", "nodes": nodes}))
        # Exceed depth without exceeding the flat node count.
        too_deep = {"kind": "field", **page, "children": [declaration(FIELD_DEPTH_CAP)]}
        result = await client.call_tool("preview_edits", dict(
            document=document, edits=insertion(anchor, too_deep),
        ))
        assert_tool_failure(result, "rejected_batch")
        assert "field-tree exceeds depth 256" in result.structuredContent["message"]
        assert list(tmp_path.iterdir()) == [source]
        await call(client, "preview_edits", document=document, edits=insertion(anchor, declaration(2)))
