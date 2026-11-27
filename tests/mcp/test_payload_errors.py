"""Installed preview payload diagnostics and compatibility."""

from copy import deepcopy

import docx
import pytest

from test_field_tree import cached_link, insertion, open_fixture, preview_validator
from test_flat_table import insert, node, open_source, recursive, reference, save, source
from test_rich_table import paragraph
from test_stdio import anyio_backend, assert_tool_failure, launch

pytestmark = pytest.mark.anyio


async def assert_invalid_payload(client, request, diagnostic):
    result = await client.call_tool("preview_edits", request)
    assert_tool_failure(result, "invalid_input")
    assert diagnostic in result.structuredContent["message"]


async def test_rich_table_typo_diagnostics_and_compatibility(launch, source):
    flat = {"nodes": [
        {**node(reference(1)), "properties": {"layout": "fixed"}},
        node(paragraph("Nested cell")),
    ]}
    legacy = recursive(flat)
    output = []
    async with launch() as client:
        document, anchor = await open_source(client, source)
        validator = await preview_validator(client)
        for name, table in (("legacy", legacy), ("flat", flat)):
            bad = deepcopy(table)
            root = bad if name == "legacy" else bad["nodes"][0]
            row = root["rows"][0]
            row["celss"] = row.pop("cells")
            invalid = dict(document=document, edits=[insert(anchor, bad)])
            assert not validator.is_valid(invalid)
            await assert_invalid_payload(client, invalid, "unknown field `celss`")
            request = dict(edits=[insert(anchor, table)])
            validator.validate(dict(document=document, **request))
            path = source.with_name(f"{name}-table.docx")
            await save(client, document, request, path)
            output.append(path)
    assert output[0].read_bytes() == output[1].read_bytes()
    nested, = docx.Document(output[0]).tables[0].cell(0, 0).tables
    assert nested.cell(0, 0).text == "Nested cell"


async def test_field_tree_child_diagnostics_and_compatibility(launch, tmp_path):
    page = {"instruction": {"kind": "page"}}
    nested = {"instruction": {"kind": "num-pages"}}
    link = cached_link("cached")

    def tree(child):
        return {"kind": "field-tree", "nodes": [{**page, "children": [child]}]}

    source = tmp_path / "source.docx"
    async with launch() as client:
        document, anchor = await open_fixture(client, source)
        validator = await preview_validator(client)
        for child, diagnostic in [
            ({"kind": "text", "texxt": "cached"}, "unknown field `texxt`"),
            ({"kind": "text"}, "missing field `text`"),
            ({"kind": "field"}, "missing field `node`"),
            ({"kind": "hyperlink", "children": []}, "missing field `target`"),
            ({**link, "children": [{"kind": "text", "texxt": "cached"}]}, "unknown field `texxt`"),
        ]:
            invalid = dict(document=document, edits=insertion(anchor, tree(child)))
            assert not validator.is_valid(invalid)
            await assert_invalid_payload(client, invalid, diagnostic)

        legacy = {"kind": "field", **page, "children": [
            {"kind": "field", **nested, "children": [link]},
        ]}
        flat = tree({"kind": "field", "node": 1})
        flat["nodes"].append({**nested, "children": [link]})
        output = []
        for name, inline in (("legacy", legacy), ("flat", flat)):
            request = dict(edits=insertion(anchor, inline))
            validator.validate(dict(document=document, **request))
            path = source.with_name(f"{name}-field.docx")
            await save(client, document, request, path)
            output.append(path)
    assert output[0].read_bytes() == output[1].read_bytes()
