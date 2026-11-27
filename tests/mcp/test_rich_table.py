"""Independent check of nested rich table declarations."""
import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT, WD_ROW_HEIGHT_RULE, WD_TABLE_ALIGNMENT
from docx.oxml.ns import qn
from jsonschema.validators import validator_for

from test_stdio import anyio_backend, call, executable, failure, launch, parts, pytestmark, validate


def paragraph(text, **formatting):
    inline = {"kind": "text", "text": text}
    if formatting:
        inline["format"] = formatting
    return {"kind": "paragraph", "paragraph": {"inlines": [inline]}}


async def test_rich_table_declaration_matches_native(launch, tmp_path):
    source, output, native = (tmp_path / name for name in ["source.docx", "out.docx", "native.docx"])
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("Original")
    fixture.save(source)
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_rich_table_oracle")
    subprocess.run([binary, native, source], check=True, timeout=20)
    table = {
        "properties": {"column_widths": [1440, 1800], "layout": "fixed", "alignment": "right",
                       "borders": [{"side": "top", "size": 8, "color": [17, 34, 51], "line": "single"}]},
        "rows": [{
            "properties": {"cant_split": True, "height": {"kind": "exact", "points": 14}},
            "cells": [],
        }, {
            "properties": {"repeat_header": True, "cant_split": False,
                           "height": {"kind": "at-least", "points": 12}},
            "cells": [{
                "properties": {"background_color": [250, 128, 96], "vertical_alignment": "center",
                               "margins": {"start": 3}},
                "blocks": [
                    paragraph("Styled cell", bold=True),
                    {"kind": "rich-table", "table": {"properties": {"column_widths": [900]},
                                                    "rows": [{"cells": [{"blocks": [paragraph("Nested")]}]}]}},
                    paragraph("Trailing"),
                ],
            }, {"blocks": [paragraph("Second")]}],
        }],
    }
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document)
        edit = {"kind": "insert-rich-table", "anchor": seen["body"][0]["anchor"], "table": table}
        schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                      if tool.name == "preview_edits")
        validator_for(schema)(schema).validate({"document": document, "edits": [edit]})
        for rows in ([], table["rows"][:1]):
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=[{**edit, "table": {**table, "rows": rows}}])
        preview = await call(client, "preview_edits", document=document, edits=[edit])
        assert preview["changes"]["table_insertions"] == [{"target": 0, "rows": 2, "columns": 2}]
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {"word/document.xml"}
    readback = docx.Document(output).tables[0]
    assert readback.alignment == WD_TABLE_ALIGNMENT.RIGHT and not readback.autofit
    padded, populated = readback.rows
    assert [cell.text for cell in padded.cells] == ["", ""]
    assert [cell.width.twips for cell in padded.cells] == [1440, 1800]
    assert padded.height.pt == 14 and padded.height_rule == WD_ROW_HEIGHT_RULE.EXACTLY
    first, second = populated.cells
    assert first.vertical_alignment == WD_CELL_VERTICAL_ALIGNMENT.CENTER
    assert first.paragraphs[0].runs[0].bold is True
    assert first.tables[0].cell(0, 0).text == "Nested"
    assert first.paragraphs[-1].text == "Trailing"
    assert second.text == "Second"
    root = ElementTree.fromstring(after["word/document.xml"])
    assert next(root.iter(qn("w:shd"))).get(qn("w:fill")) == "FA8060"
    assert [node.get(qn("w:val")) for node in root.iter(qn("w:cantSplit"))] == ["1", "0"]
    validate(output)
