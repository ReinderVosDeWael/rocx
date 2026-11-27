"""Preview rejection at the native XML depth boundary."""

import pytest

from test_flat_table import insert, node, reference, save, source  # noqa: F401
from test_native_diagnostics import replace_part
from test_stdio import anyio_backend, assert_tool_failure, call, launch, parts  # noqa: F401

pytestmark = pytest.mark.anyio


def empty_chain(depth):
    return {"nodes": [node(reference(i)) for i in range(1, depth)] + [node()]}


def assert_depth_failure(result):
    assert_tool_failure(result, "rejected_batch")
    assert result.structuredContent["path"] == "word/document.xml"
    assert result.structuredContent["rule"] == "invalid_content"
    assert result.structuredContent["detail"] == "XML depth exceeds read limit (256)"


async def open_body(client, path):
    document = (await call(client, "open_document", path=path.name))["document"]
    body = await call(client, "inspect_document", document=document, view={"kind": "body"})
    return document, body["body"][0]["anchor"]


async def test_flat_table_rejects_unreopenable_body_depth(launch, source):
    output = source.with_name("retry.docx")
    async with launch() as client:
        document, anchor = await open_body(client, source)
        # 84 tables need no rich terminal content to exceed the import depth:
        # the deepest cell already emits a width under its property wrapper.
        failure = await client.call_tool(
            "preview_edits", dict(document=document, edits=[insert(anchor, empty_chain(84))]),
        )
        assert_depth_failure(failure)

        await save(client, document, {"edits": [insert(anchor, empty_chain(83))]}, output)
        await open_body(client, output)


async def test_flat_table_near_depth_limit_preserves_unrelated_malformed_header(launch, source):
    malformed = source.with_name("malformed-header.docx")
    output = source.with_name("preserved.docx")
    replace_part(source, malformed, "word/header1.xml", b"<broken")

    async with launch() as client:
        document, anchor = await open_body(client, malformed)
        await save(client, document, {"edits": [insert(anchor, empty_chain(83))]}, output)
        await open_body(client, output)

    before, after = parts(malformed), parts(output)
    assert before.keys() == after.keys()
    assert {part for part in before if before[part] != after[part]} == {"word/document.xml"}


async def test_shallow_table_insertion_in_an_existing_deep_cell(launch, source):
    deep_source = source.with_name("deep-source.docx")
    async with launch() as client:
        document, anchor = await open_body(client, source)
        await save(client, document, {"edits": [insert(anchor, empty_chain(83))]}, deep_source)
        document, _ = await open_body(client, deep_source)
        seen = await call(
            client, "inspect_document", document=document, view={"kind": "physical-tables", "limit": 100},
        )
        deepest = max(seen["physical_tables"], key=lambda table: table["depth"])
        assert deepest["depth"] == 82

        failure = await client.call_tool("preview_edits", dict(document=document, operation={
            "kind": "edit-part-tables",
            "anchor": deepest["owner"]["anchor"],
            "edits": [{
                "anchor": deepest["anchor"],
                "edit": {"kind": "replace-cell", "row": 0, "cell": 0,
                         "blocks": [{"kind": "rich-table", "table": empty_chain(1)}]},
            }],
        }))
        assert_depth_failure(failure)
