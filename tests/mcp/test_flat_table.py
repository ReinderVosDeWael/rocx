"""Deep rich tables through installed stdio, with independent native and OOXML checks."""

from copy import deepcopy
import json
from select import select
import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.table import WD_ROW_HEIGHT_RULE, WD_TABLE_ALIGNMENT
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_rich_table import paragraph
from test_stdio import anyio_backend, assert_tool_failure, call, executable, launch, parts, validate

pytestmark = pytest.mark.anyio
DEPTH_LIMIT = 84
WIDE_SPAN = 8193
CHAIN_SPAN = 1024


def node(*blocks):
    return {"rows": [{"cells": [{"blocks": list(blocks)}]}]}


def reference(index):
    return {"kind": "table-ref", "node": index}


def declaration(depth):
    nodes = []
    for index in range(depth):
        blocks = [paragraph(f"Level {index} & <text>", bold=True)]
        if index + 1 < depth:
            blocks.extend([reference(index + 1), paragraph("After nested table")])
        table = node(*blocks)
        table["properties"] = dict(layout="fixed", alignment="right", column_widths=[1440])
        table["rows"][0]["properties"] = {"height": {"kind": "exact", "points": 14}}
        table["rows"][0]["cells"][0]["properties"] = {"background_color": [17, 34, 51]}
        nodes.append(table)
    return {"nodes": nodes}


def with_span(table, span):
    table["rows"][0]["cells"][0].setdefault("properties", {})["grid_span"] = span
    return table


def recursive(flat):
    nodes = deepcopy(flat["nodes"])
    for table in reversed(nodes):
        for row in table["rows"]:
            for cell in row["cells"]:
                for index, block in enumerate(cell["blocks"]):
                    if block["kind"] == "table-ref":
                        cell["blocks"][index] = dict(kind="rich-table", table=nodes[block["node"]])
    return nodes[0]


def insert(anchor, table):
    return {"kind": "insert-rich-table", "anchor": anchor, "table": table}


@pytest.fixture
def source(tmp_path):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("Original")
    fixture.add_table(1, 1).cell(0, 0).text = "Original cell"
    fixture.sections[0].header.paragraphs[0].text = "Original header"
    source = tmp_path / "source.docx"
    fixture.save(source)
    return source


async def open_source(client, source):
    document = (await call(client, "open_document", path=source.name))["document"]
    seen = await call(client, "inspect_document", document=document)
    return document, seen["body"][0]["anchor"]


async def preview_validator(client):
    schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                  if tool.name == "preview_edits")
    return validator_for(schema)(schema)


async def save(client, document, request, output):
    draft = await call(client, "preview_edits", document=document, **request)
    await call(client, "save_copy", document=document, preview=draft["preview"], output=output.name)
    return draft


@pytest.mark.parametrize(("depth", "mode"), [(24, "body"), (83, "body"),
                                             (DEPTH_LIMIT, "header"), (24, "wide"),
                                             (48, "wide-chain")])
async def test_flat_rich_table_matches_native_and_reopens(launch, source, depth, mode):
    output, native = (source.with_name(name) for name in ("out.docx", "native.docx"))
    original = source.read_bytes()
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_flat_table_oracle")
    args = [oracle, source, native, str(depth)]
    if mode != "body":
        args.append(mode)
    subprocess.run(args, check=True, timeout=20)
    table_input = declaration(depth)
    header = mode == "header"
    if mode == "wide":
        leaf = with_span(table_input["nodes"][-1], WIDE_SPAN)
        leaf["properties"]["column_widths"] = [1] * WIDE_SPAN
        leaf["rows"].append(deepcopy(leaf["rows"][0]))
    elif mode == "wide-chain":
        for table in table_input["nodes"]:
            with_span(table, CHAIN_SPAN)["properties"]["column_widths"] = [1] * CHAIN_SPAN
    async with launch() as client:
        document, anchor = await open_source(client, source)
        if header:
            table_input["nodes"][-1]["rows"][0]["cells"][0]["blocks"].clear()
            view = dict(kind="default-headers")
            seen, = (await call(client, "inspect_document", document=document,
                               view=view))["default_headers"]
            request = dict(operation=dict(
                kind="edit-story-blocks", anchor=seen["image_slot"]["anchor"],
                edit=dict(mode="shared", index=1, remove=0,
                          blocks=[dict(kind="rich-table", table=table_input)]),
            ))
        else:
            request = dict(edits=[insert(anchor, table_input)])
        (await preview_validator(client)).validate(dict(document=document, **request))
        draft = await save(client, document, request, output)
        reopened = await call(client, "open_document", path=output.name)
        assert reopened["document"] != document
        if header:
            seen, = (await call(client, "inspect_document", document=reopened["document"],
                               view=view))["default_headers"]
            assert len(seen["blocks"]) == 2
        else:
            columns = CHAIN_SPAN if mode == "wide-chain" else 1
            assert draft["changes"]["table_insertions"] == [
                {"target": 0, "rows": 1, "columns": columns},
            ]
    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    part = "word/header1.xml" if header else "word/document.xml"
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {part}
    assert len(list(ElementTree.fromstring(after[part]).iter(qn("w:tbl")))) == depth + (not header)
    document = docx.Document(output)
    if header:
        story = document.sections[0].header
        assert story.paragraphs[0].text == "Original header"
        table, = story.tables
        original_table, = document.tables
    else:
        table, original_table = document.tables
    assert original_table.cell(0, 0).text == "Original cell"
    for index in range(depth):
        assert table.alignment == WD_TABLE_ALIGNMENT.RIGHT and not table.autofit
        if mode == "wide" and index + 1 == depth:
            columns = table._tbl.tblGrid.gridCol_lst
            assert len(columns) == WIDE_SPAN
            assert all(column.get(qn("w:w")) == "1" for column in columns)
            assert len(table.rows) == 2
            for row in table.rows:
                tc, = row._tr.tc_lst  # No padding cells on either fully spanned row.
                assert tc.tcPr.gridSpan.val == WIDE_SPAN
                assert tc.p_lst[0].r_lst[0].t_lst[0].text == f"Level {index} & <text>"
            break
        if mode == "wide-chain":
            columns = table._tbl.tblGrid.gridCol_lst
            assert len(columns) == CHAIN_SPAN
            assert all(column.get(qn("w:w")) == "1" for column in columns)
        else:
            assert table.columns[0].width.twips == 1440
        row, = table.rows
        assert row.height.pt == 14 and row.height_rule == WD_ROW_HEIGHT_RULE.EXACTLY
        tc, = row._tr.tc_lst
        if mode == "wide-chain":
            assert tc.tcPr.gridSpan.val == CHAIN_SPAN
        cell = row.cells[0]
        assert cell._tc.tcPr.find(qn("w:shd")).get(qn("w:fill")) == "112233"
        if header and index + 1 == depth:
            assert cell.paragraphs[0].text == ""
        else:
            run, = cell.paragraphs[0].runs
            assert run.text == f"Level {index} & <text>" and run.bold
        if index + 1 == depth:
            assert not cell.tables
        else:
            assert cell.paragraphs[-1].text == "After nested table"
            table, = cell.tables
    validate(output)


async def test_flat_rich_table_uses_existing_consumers_and_coordinates(launch, source):
    flat = declaration(3)
    # Non-preorder forward references, a plain table and padding share one cell.
    flat["nodes"][1]["rows"][0]["cells"][0]["blocks"].pop(1)
    flat["nodes"][0]["rows"][0]["cells"][0]["blocks"].extend([
        {"kind": "table", "rows": [["Plain leaf"]]}, reference(2),
    ])
    root_blocks = flat["nodes"][0]["rows"][0]["cells"][0]["blocks"]
    root_blocks[1], root_blocks[-1] = root_blocks[-1], root_blocks[1]
    flat["nodes"][0]["rows"].append({"cells": []})
    nested = recursive(flat)
    async with launch() as client:
        document, anchor = await open_source(client, source)
        validator = await preview_validator(client)
        (body,) = (await call(client, "inspect_document", document=document,
                              view=dict(kind="body-tables")))["body_tables"]
        (physical,) = (await call(client, "inspect_document", document=document,
                                  view=dict(kind="physical-tables")))["physical_tables"]

        def body_edit(kind, **payload):
            return dict(edits=[dict(kind="table-structure", anchor=body["anchor"],
                                    edit=dict(kind=kind, **payload))])

        def physical_edit(kind, **payload):
            return dict(operation=dict(kind="edit-part-tables", anchor=physical["owner"]["anchor"],
                                       edits=[dict(anchor=physical["anchor"],
                                                   edit=dict(kind=kind, **payload))]))

        donor = dict(kind="table")
        for index, consumer in enumerate([
            lambda table: dict(edits=[insert(anchor, table)]),
            lambda table: body_edit("replace-node", site=donor, source=table),
            lambda table: body_edit("set-property", site=donor, field="layout", source=table),
            lambda table: physical_edit("set-property", site=donor, field="layout", source=table),
            lambda table: physical_edit("replace-cell", row=0, cell=0,
                                        blocks=[dict(kind="rich-table", table=table)]),
        ]):
            outputs = [source.with_name(f"{index}-{form}.docx") for form in ("flat", "nested")]
            for table, output in zip((flat, nested), outputs, strict=True):
                request = consumer(table)
                validator.validate(dict(document=document, **request))
                await save(client, document, request, output)
            assert outputs[0].read_bytes() == outputs[1].read_bytes()
        output = docx.Document(source.with_name("0-flat.docx"))
        cells = output.tables[0].rows
        assert cells[1].cells[0].text == ""
        leaf_tables = cells[0].cells[0].tables
        assert [table.cell(0, 0).paragraphs[0].text for table in leaf_tables] == [
            "Level 2 & <text>", "Plain leaf", "Level 1 & <text>",
        ]
        for request in [
            body_edit("replace-node", site=dict(kind="cell", row=0, cell=99), source=declaration(1)),
            physical_edit("set-property", site=dict(kind="cell", row=99, cell=0),
                          field="shd", source=declaration(1)),
            physical_edit("replace-cell", row=0, cell=99,
                          blocks=[dict(kind="rich-table", table=declaration(1))]),
        ]:
            validator.validate(dict(document=document, **request))
            result = await client.call_tool("preview_edits", dict(document=document, **request))
            assert_tool_failure(result, "rejected_batch")
            assert "index out of bounds" in result.structuredContent["message"]


async def test_flat_rich_table_bounds_topology_and_size(launch, source):
    leaf = node(paragraph("leaf"))
    crowded = node(*([paragraph("x")] * 16383))
    padded = with_span(node(), WIDE_SPAN)
    padded["rows"].extend([{"cells": []}, {"cells": []}])
    too_many_columns = {"nodes": [
        *(with_span(node(reference(index + 1)), 16384) for index in range(4)),
        node(),
    ]}
    deep_plain = declaration(DEPTH_LIMIT)
    deep_plain["nodes"][-1]["rows"][0]["cells"][0]["blocks"].append(
        {"kind": "table", "rows": [["Too deep"]]},
    )
    async with launch() as client:
        document, anchor = await open_source(client, source)
        validator = await preview_validator(client)
        for table, diagnostic in [
            ({"nodes": []}, "requires 1 to 1024 nodes"),
            ({"nodes": [leaf] * 1025}, "requires 1 to 1024 nodes"),
            ({"nodes": [node(reference(0))]}, "must reference a later node"),
            ({"nodes": [node(reference(1)), node(reference(0))]}, "must reference a later node"),
            ({"nodes": [node(reference(1))]}, "references out-of-bounds node"),
            ({"nodes": [node(reference(1), reference(2)), node(reference(2)), leaf]},
             "referenced more than once"),
            ({"nodes": [leaf, leaf]}, "unreferenced flat rich table node"),
            ({"nodes": [crowded]}, "16384 row, cell and block entries"),
            ({"nodes": [padded]}, "16384 emitted cells"),
            (too_many_columns, "65536 emitted grid columns"),
            (declaration(DEPTH_LIMIT + 1), "exceeds depth 84"),
            (deep_plain, "exceeds depth 84"),
            ({"nodes": [{"rows": [{"cells": []}]}]}, "at least one row with cells"),
            ({"nodes": [{**leaf, "properties": {"column_widths": [1, 2]}}]},
             "must match number of columns"),
        ]:
            result = await client.call_tool("preview_edits", dict(document=document,
                                            edits=[insert(anchor, table)]))
            assert_tool_failure(result, "rejected_batch")
            assert diagnostic in result.structuredContent["message"]
        for invalid in [
            {"nodes": [leaf], "rows": leaf["rows"]},
            {"nodes": [node(dict(kind="rich-table", table=leaf))]},
            {"nodes": [node(reference(-1))]},
        ]:
            assert not validator.is_valid(dict(document=document, edits=[insert(anchor, invalid)]))
            result = await client.call_tool("preview_edits", dict(document=document,
                                            edits=[insert(anchor, invalid)]))
            assert_tool_failure(result, "invalid_input")
        assert list(source.parent.iterdir()) == [source]
        await call(client, "preview_edits", document=document, edits=[insert(anchor, declaration(2))])


async def test_zero_column_donor_copies_only_property_and_is_not_a_fragment(launch, source):
    native = source.with_name("donor-native.docx")
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_flat_table_oracle")
    subprocess.run([oracle, source, native, "donor"], check=True, timeout=20)
    donor = {"rows": [{"cells": [], "properties": {"height": {"kind": "exact", "points": 18}}}]}
    original = source.read_bytes()
    async with launch() as client:
        document, anchor = await open_source(client, source)
        validator = await preview_validator(client)
        body, = (await call(client, "inspect_document", document=document,
                            view={"kind": "body-tables"}))["body_tables"]
        physical, = (await call(client, "inspect_document", document=document,
                                view={"kind": "physical-tables"}))["physical_tables"]

        def ordinary(edit):
            return dict(edits=[dict(kind="table-structure", anchor=body["anchor"], edit=edit)])

        def scoped(edit):
            return dict(operation=dict(kind="edit-part-tables", anchor=physical["owner"]["anchor"],
                                       edits=[dict(anchor=physical["anchor"], edit=edit)]))

        async def reject(request, message):
            result = await client.call_tool("preview_edits", dict(document=document, **request))
            assert_tool_failure(result, "rejected_batch")
            assert message in result.structuredContent["message"]

        for index, table in enumerate((donor, {"nodes": [donor]})):
            await reject(dict(edits=[insert(anchor, table)]), "at least one row with cells")
            await reject(ordinary(dict(kind="replace-node", site=dict(kind="table"), source=table)),
                         "at least one row with cells")
            await reject(ordinary(dict(kind="set-property", site=dict(kind="cell", row=0, cell=0),
                                       field="backgroundColor", source=table)),
                         "missing resolved source cell")
            edit = dict(kind="set-property", site=dict(kind="row", row=0), field="height",
                        source=table)
            for scope, request in (("body", ordinary(edit)), ("scoped", scoped(edit))):
                validator.validate(dict(document=document, **request))
                output = source.with_name(f"donor-{index}-{scope}.docx")
                await save(client, document, request, output)
                assert output.read_bytes() == native.read_bytes()
        for table in ({"rows": []}, {"nodes": [{"rows": []}]}):
            await reject(ordinary(dict(kind="set-property", site=dict(kind="row", row=0),
                                       field="height", source=table)), "requires at least one row")

    assert source.read_bytes() == original
    before, after = parts(source), parts(native)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    row, = docx.Document(native).tables[0].rows
    assert row.height.pt == 18 and row.height_rule == WD_ROW_HEIGHT_RULE.EXACTLY
    cell, = row.cells
    assert cell.text == "Original cell"
    validate(native)


async def test_zero_column_nested_control_table_is_not_transportable(launch, source):
    fixture = docx.Document(source)
    paragraph = fixture.add_paragraph("Original control")._p
    wrapper, props, content = map(OxmlElement, ["w:sdt", "w:sdtPr", "w:sdtContent"])
    props.append(OxmlElement("w:richText"))
    wrapper.extend([props, content])
    paragraph.addprevious(wrapper)
    content.append(paragraph)
    fixture.save(source)

    native = source.with_name("zero-column-control-native.docx")
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_flat_table_oracle")
    subprocess.run([oracle, source, native, "zero-column-control"], check=True, timeout=20)
    validate(native)  # Transitional XSD alone does not enforce Word's minimum row content.
    root = ElementTree.fromstring(parts(native)["word/document.xml"])
    outer = root.find(f".//{qn('w:sdtContent')}/{qn('w:tbl')}")
    inner = outer.find(f"./{qn('w:tr')}/{qn('w:tc')}/{qn('w:tbl')}")
    assert list(inner.find(qn("w:tblGrid"))) == []
    row, = inner.findall(qn("w:tr"))
    # MS-OI29500's note on Part 1 §17.4.78: Word treats a row without a
    # non-property child as corrupt, even when the package passes XSD.
    assert all(child.tag == qn("w:trPr") for child in row)

    empty = {"rows": [{"cells": []}]}
    outer = node(reference(1))
    outer["properties"] = {"column_widths": [1440]}
    flat = {"nodes": [outer, empty]}
    async with launch() as client:
        document, _ = await open_source(client, source)
        control, = (await call(client, "inspect_document", document=document,
                               view={"kind": "controls"}))["controls"]
        validator = await preview_validator(client)
        for table in (empty, {"nodes": [empty]}, recursive(flat), flat):
            request = dict(document=document, edits=[dict(kind="control", edit=dict(
                kind="replace-blocks", anchor=control["anchor"],
                blocks=[dict(kind="rich-table", table=table)],
            ))])
            validator.validate(request)
            result = await client.call_tool("preview_edits", request)
            assert_tool_failure(result, "rejected_batch")
            assert "at least one row with cells" in result.structuredContent["message"]


def test_raw_json_recursion_limit_is_unchanged(source):
    binary = executable("ROCX_MCP_BINARY")
    with subprocess.Popen([binary], cwd=source.parent, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          text=True) as server:
        def send(method, request_id=None, **params):
            frame = dict(jsonrpc="2.0", method=method)
            if request_id is not None:
                frame["id"] = request_id
            if params:
                frame["params"] = params
            server.stdin.write(json.dumps(frame) + "\n")
            server.stdin.flush()

        def receive():
            assert select([server.stdout], [], [], 5)[0], "MCP response timed out"
            return json.loads(server.stdout.readline())

        try:
            send("initialize", 1, protocolVersion="2025-11-25", capabilities={},
                 clientInfo=dict(name="raw-json-limit", version="1"))
            assert receive()["id"] == 1
            send("notifications/initialized")
            for request_id, depth in [(2, 2), (3, 24)]:
                send("tools/call", request_id, name="preview_edits", arguments=dict(
                    document="nonexistent", edits=[insert("unused", recursive(declaration(depth)))],
                ))
                if depth == 2:
                    response = receive()
                    assert response["id"] == request_id
                    assert response["result"]["structuredContent"]["category"] == "invalid_document"
            # A rejected input line cannot dispatch a preview or break subsequent requests.
            send("ping", 4)
            assert receive()["id"] == 4
            assert not select([server.stdout], [], [], 1)[0], "deep raw request was dispatched"
        finally:
            server.stdin.close()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
