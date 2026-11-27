"""Flat nested field transport through the installed stdio client."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.oxml.ns import qn
from jsonschema import ValidationError
from jsonschema.validators import validator_for
import pytest

from test_fields import simple
from test_stdio import anyio_backend, assert_tool_failure, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio

MAIN = "word/document.xml"
MAIN_RELS = "word/_rels/document.xml.rels"
FIELD_DEPTH_CAP = 256
FIELD_NODE_CAP = 16_384
FIELD_CHILD_CAP = 16_384


def cached_link(label):
    return {
        "kind": "hyperlink",
        "target": {"kind": "url", "url": f"https://example.org/{label}"},
        "tooltip": f"cached & {label}",
        "history": False,
        "children": [
            {"kind": "text", "text": label, "format": {"italic": True}},
            {"kind": "tab"},
            {"kind": "break", "type": "line"},
        ],
    }


def declaration(depth, rich=False):
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
    if rich:
        nodes[-1]["children"].append(cached_link("deepest"))
        nodes[-2]["children"] = [cached_link("before"), *nodes[-2]["children"], cached_link("after")]
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


async def preview_validator(client):
    tools = (await client.list_tools()).tools
    schema = next(tool.inputSchema for tool in tools if tool.name == "preview_edits")
    return validator_for(schema)(schema)


@pytest.mark.parametrize(("depth", "rich"), [(64, False), (200, False), (248, False), (248, True)])
async def test_flat_field_tree_matches_native(launch, tmp_path, depth, rich):
    source, output, native = (tmp_path / name for name in ("source.docx", "out.docx", "native.docx"))
    async with launch() as client:
        document, anchor = await open_fixture(client, source)
        original = source.read_bytes()
        oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_field_tree_oracle")
        args = [oracle, source, native, str(depth)]
        if rich:
            args.append("rich")
        subprocess.run(args, check=True, cwd=tmp_path, timeout=20)

        request = dict(document=document, edits=insertion(anchor, declaration(depth, rich)))
        (await preview_validator(client)).validate(request)
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
    assert {name for name in before if before[name] != after[name]} == (
        {MAIN, MAIN_RELS} if rich else {MAIN}
    )
    xml = ElementTree.fromstring(after[MAIN])
    assert sum(1 for _ in xml.iter(qn("w:fldSimple"))) == depth
    paragraph = xml.find(f"./{qn('w:body')}/{qn('w:p')}")
    assert paragraph.find(f"./{qn('w:r')}/{qn('w:t')}").text == "before"
    node = paragraph
    for index in range(depth):
        parent = node
        node, = parent.findall(qn("w:fldSimple"))
        assert node.get(qn("w:instr")) == f" LEVEL {index} "
        assert node.get(qn("w:dirty")) == ("0" if index == 0 else None)
        assert node.get(qn("w:fldLock")) == ("1" if index == depth - 1 else None)
    if rich:
        left, nested, right = parent
        assert nested is node
        run, deepest = node
        relations = {rel.get("Id"): rel for rel in ElementTree.fromstring(after[MAIN_RELS])}
        for link, label in zip((left, deepest, right), ("before", "deepest", "after"), strict=True):
            assert link.tag == qn("w:hyperlink")
            assert link.get(qn("w:tooltip")) == f"cached & {label}"
            assert link.get(qn("w:history")) == "0"
            text, tab, br = link
            assert text.find(qn("w:t")).text == label
            assert text.find(f"./{qn('w:rPr')}/{qn('w:i')}") is not None
            assert tab.find(qn("w:tab")) is not None
            assert br.find(qn("w:br")).get(qn("w:type")) == "textWrapping"
            relation = relations[link.get(qn("r:id"))]
            assert relation.get("Target") == f"https://example.org/{label}"
            assert relation.get("TargetMode") == "External"
    else:
        run, = node
    assert run.find(qn("w:t")).text == "cached & <value>"
    assert run.find(f"./{qn('w:rPr')}/{qn('w:b')}") is not None
    validate(source, output)


async def test_flat_field_tree_preserves_deep_wide_cache(launch, tmp_path):
    source, output, native = (tmp_path / name for name in ("source.docx", "out.docx", "native.docx"))
    depth = 80
    inline = declaration(depth)
    children = inline["nodes"][-1]["children"]
    children.extend([{"kind": "tab"}] * 1_100)
    children.extend({"kind": "field", "node": depth + i} for i in range(300))
    inline["nodes"].extend(
        {"instruction": {"kind": "raw", "text": f" SIBLING {i} "}} for i in range(300)
    )
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("before")._p.append(simple(" PAGE ", "old"))
    fixture.save(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        (field,) = (await call(client, "inspect_document", document=document, view={"kind": "fields"}))["fields"]
        original = source.read_bytes()
        oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_field_tree_oracle")
        subprocess.run([oracle, source, native, str(depth), "wide"], check=True, cwd=tmp_path, timeout=20)
        request = dict(document=document, edits=[dict(
            kind="field", anchor=field["anchor"], edit=dict(kind="cache", children=[inline]),
        )])
        (await preview_validator(client)).validate(request)
        draft = await call(client, "preview_edits", **request)
        await call(client, "save_copy", document=document, preview=draft["preview"], output=output.name)
        await call(client, "open_document", path=output.name)

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {MAIN}
    root = ElementTree.fromstring(after[MAIN])
    outer, = root.iterfind(f".//{qn('w:fldSimple')}[@{qn('w:instr')}=' PAGE ']")
    field, = outer.iterfind(f".//{qn('w:fldSimple')}[@{qn('w:instr')}=' LEVEL 79 ']")
    assert len(list(field.iter(qn("w:tab")))) == 1_100
    assert [child.get(qn("w:instr")) for child in field.findall(qn("w:fldSimple"))] == [
        f" SIBLING {i} " for i in range(300)
    ]
    validate(source, output)


async def test_flat_field_tree_rejects_invalid_nodes_and_allows_retry(launch, tmp_path):
    source = tmp_path / "source.docx"
    page = {"instruction": {"kind": "page"}}
    tab = {"kind": "tab"}

    def refs(*indices):
        return {**page, "children": [{"kind": "field", "node": index} for index in indices]}

    crowded_root = refs(1)
    crowded_root["children"].extend([tab] * (FIELD_CHILD_CAP // 2))
    async with launch() as client:
        document, anchor = await open_fixture(client, source)
        for nodes in [
            [],
            [refs(0)],  # Cycle.
            [refs(1), refs(0)],  # Back edge.
            [refs(1)],  # Out of bounds.
            [page, page],  # Unreachable node.
            [refs(1, 2), refs(2), page],  # Shared subtree.
            [page] * (FIELD_NODE_CAP + 1),
            [crowded_root, {**page, "children": [tab] * (FIELD_CHILD_CAP // 2)}],
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
        deep_link = declaration(FIELD_DEPTH_CAP)
        deep_link["nodes"][-1]["children"].append(cached_link("too-deep"))
        crowded_link = {**cached_link("too-many-children"), "children": [tab] * (FIELD_CHILD_CAP - 1)}
        crowded_tree = {
            "kind": "field-tree",
            "nodes": [{**page, "children": [crowded_link, tab]}],
        }
        validator = await preview_validator(client)
        for inline, message in [
            (deep_link, "field-tree exceeds depth 256"),
            (crowded_tree, f"field-tree exceeds {FIELD_CHILD_CAP} total child entries"),
        ]:
            request = dict(document=document, edits=insertion(anchor, inline))
            validator.validate(request)
            result = await client.call_tool("preview_edits", request)
            assert_tool_failure(result, "rejected_batch")
            assert message in result.structuredContent["message"]

        fraction = {"kind": "fraction", "numerator": "1", "denominator": "2"}
        field_in_link = {**cached_link("invalid"), "children": [{"kind": "field", "node": 1}]}
        for nodes in ([{**page, "children": [fraction]}], [{**page, "children": [field_in_link]}, page]):
            inline = {"kind": "field-tree", "nodes": nodes}
            with pytest.raises(ValidationError):
                validator.validate(dict(document=document, edits=insertion(anchor, inline)))

        assert list(tmp_path.iterdir()) == [source]
        await call(client, "preview_edits", document=document, edits=insertion(anchor, declaration(2, rich=True)))
