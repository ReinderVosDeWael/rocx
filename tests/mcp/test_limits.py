"""Per-open limits, checked against the native reader at exact ZIP boundaries."""

import json
from zipfile import ZipFile

import docx
from jsonschema.validators import validator_for

from test_stdio import anyio_backend, call, digest, failure, launch, oracle, parts, pytestmark


async def test_import_budgets_match_native_reader(launch, tmp_path):
    source = tmp_path / "budgets.docx"
    fixture = docx.Document()
    fixture.add_paragraph("Open at the exact archive limits")
    fixture.save(source)
    with ZipFile(source) as package:
        members = package.infolist()
        sizes = [member.file_size for member in members if not member.is_dir()]
    budgets = dict(
        max_parts=len(members),
        max_part_bytes=max(sizes),
        max_expanded_bytes=sum(sizes),
    )
    exact = {name: str(amount) for name, amount in budgets.items()}
    expected_digest = digest(source.read_bytes())

    async with launch() as client:
        tools = await client.list_tools()
        schema = next(tool.inputSchema for tool in tools.tools if tool.name == "open_document")
        validator = validator_for(schema)(schema)

        for field, amount in budgets.items():
            insufficient = {**exact, field: str(amount - 1)}
            native = json.loads(oracle(
                tmp_path / f"{field}.json", "open-with-limits", source, *insufficient.values(),
            ))
            assert "exceed" in native["error"]
            result = await client.call_tool(
                "open_document", {"path": source.name, "limits": insufficient},
            )
            assert result.isError and result.structuredContent["category"] == "invalid_document_data"
            assert result.structuredContent["message"] == native["error"]
            assert "limit" in result.structuredContent["action"]
            assert "document" not in result.structuredContent

        for limits in (None, {"max_parts": "01"}, {"max_parts": ""}, {"max_parts": 17},
                       {"max_part_bytes": "-1"}, {"max_part_bytes": None}, {"unknown": "1"}):
            arguments = {"path": source.name, "limits": limits}
            assert not validator.is_valid(arguments)
            await failure(client, "open_document", "invalid_input", **arguments)
        await failure(client, "open_document", "invalid_input", path=source.name,
                      limits={"max_expanded_bytes": "18446744073709551616"})

        native = json.loads(oracle(
            tmp_path / "exact.json", "open-with-limits", source, *exact.values(),
        ))
        for limits in (exact, {}, {"max_part_bytes": "18446744073709551615"}):
            arguments = {"path": source.name, "limits": limits}
            assert validator.is_valid(arguments)
            reopened = await call(client, "open_document", **arguments)
            inspection = await call(client, "inspect_document", document=reopened["document"])
            assert len(inspection["body"]) == native["blocks"] == 1
            assert inspection["body"][0]["text"] == fixture.paragraphs[0].text
            assert reopened["snapshot"] == expected_digest
            await call(client, "close_document", document=reopened["document"])
        assert digest(source.read_bytes()) == expected_digest


async def test_empty_native_batch_has_an_explicit_retained_preview(launch, tmp_path):
    source = tmp_path / "source.docx"
    fixture = docx.Document()
    fixture.add_paragraph("No edits staged")
    fixture.save(source)
    expected = oracle(tmp_path / "native.docx", "no-op", source)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "preview_edits", "invalid_input", document=document, edits=[])
        preview = await call(client, "preview_edits", document=document, operation={"kind": "no-op"})
        assert preview["output_snapshot"] == digest(expected)
        assert preview["changes"] == dict.fromkeys((
            "changes", "table_insertions", "paragraph_insertions", "block_changes",
            "inline_changes", "header_images", "run_text", "run_removals", "cell_alignments",
        ), [])
        await failure(client, "preview_edits", "invalid_input", document=document,
                      operation={"kind": "no-op", "anchor": "anything"})
        target = tmp_path / "noop.docx"
        await call(client, "save_copy", document=document, preview=preview["preview"],
                   output=target.name)
        assert target.read_bytes() == expected
        assert parts(source) == parts(target)
        assert docx.Document(target).paragraphs[0].text == "No edits staged"
