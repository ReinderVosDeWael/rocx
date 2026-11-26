"""Acceptance tests for the installed rocx-mcp process, not its Rust handlers."""

from contextlib import asynccontextmanager
from copy import deepcopy
from datetime import datetime, timedelta, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import sys
from xml.etree import ElementTree
from zipfile import ZipFile

import anyio
import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.opc.part import Part
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.table import Table
from jsonschema.validators import validator_for
from lxml import etree
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import McpError
import pytest


ROOT = Path(__file__).resolve().parents[2]
PNG = ROOT / "tests/fixtures/images/sample.png"
PROTOCOL = "2025-11-25"
EXPECTED_TOOLS = {
    "create_document": set(),
    "open_document": {"path"},
    "inspect_document": {"document"},
    "preview_edits": {"document"},
    "save_copy": {"document", "preview", "output"},
    "close_document": {"document"},
}

pytestmark = pytest.mark.anyio


@pytest.fixture
def anyio_backend():
    return "asyncio"


def executable(name):
    assert name in os.environ, f"Set {name} to an absolute executable path (see tests/mcp/README.md)"
    path = Path(os.environ[name])
    assert path.is_absolute() and path.is_file() and os.access(path, os.X_OK), path
    return path


@pytest.fixture
def launch(tmp_path):
    server = StdioServerParameters(command=str(executable("ROCX_MCP_BINARY")), cwd=tmp_path)

    @asynccontextmanager
    async def connect():
        frame_errors = []

        async def collect_errors(message):
            if isinstance(message, Exception):
                frame_errors.append(message)

        async with stdio_client(server, errlog=sys.stderr) as (read, write):
            async with ClientSession(
                read, write, read_timeout_seconds=timedelta(seconds=20), message_handler=collect_errors,
            ) as client:
                handshake = await client.initialize()  # SDK also sends notifications/initialized.
                assert handshake.serverInfo.name == "rocx-mcp"
                assert handshake.protocolVersion == PROTOCOL
                assert handshake.capabilities.tools is not None
                yield client
                assert not frame_errors, frame_errors

    return connect


async def call(client, tool, **arguments):
    result = await client.call_tool(tool, arguments)
    assert not result.isError and isinstance(result.structuredContent, dict), (tool, result)
    return result.structuredContent


def assert_tool_failure(result, category):
    assert result.isError, result
    data = result.structuredContent
    if data is not None:
        assert data["category"] == category
        assert data["message"] and data["action"]
        assert "preview" not in data
    else:
        (content,) = result.content
        code, sep, action = content.text.partition(":")
        assert code == category and sep and action.strip(), result


async def failure(client, tool, category, **arguments):
    assert_tool_failure(await client.call_tool(tool, arguments), category)


def digest(data):
    return sha256(data).hexdigest()


def validate(*paths):
    subprocess.run([sys.executable, ROOT / "tools/validate_ooxml.py", *paths], check=True, timeout=30)


def oracle(output, command, *args):
    binary = executable("ROCX_MCP_ORACLE")
    subprocess.run([binary, command, output, *args], check=True, cwd=output.parent, timeout=20)
    return output.read_bytes()


def parts(path):
    with ZipFile(path) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def make_source(path, title="Original"):
    document = docx.Document()
    # python-docx's default zoom element omits the required Transitional attribute.
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    heading = document.add_paragraph()
    heading.add_run(title).bold = True
    heading.add_run(" suffix")._r.set(qn("w:rsidR"), "00A10B20")
    # Legitimate non-text content for which inspection must not offer an editable run.
    symbol = OxmlElement("w:sym")
    symbol.set(qn("w:font"), "Symbol")
    symbol.set(qn("w:char"), "F041")
    heading.add_run()._r.append(symbol)
    document.add_paragraph("Table goes next")
    first = document.sections[0]
    first.header.paragraphs[0].text = "Shared header"
    first.first_page_header.paragraphs[0].text = "First-page header"
    first.different_first_page_header_footer = True
    document.add_section()
    tail = document.add_paragraph("Keep tail")
    tail.add_run(" drop me")
    row = document.add_table(1, 2).rows[0]
    row.cells[0].text = "left cell"
    row.cells[1].text = "target cell"
    document.save(path)
    return path.read_bytes()


ENTRY_CASES = {
    "delete-unused": (("footnote", "8"), dict(kind="delete-unused-note")),
    "replace-first": (
        ("footnote", "007"), dict(kind="replace-paragraph", index=0, text="New footnote 😀"),
    ),
    "insert-last": (
        ("endnote", "007"), dict(kind="insert-paragraph", index=2, text="New endnote & tail"),
    ),
    "remove-last": (("endnote", "007"), dict(kind="remove-paragraph", index=1)),
    "author": (("comment", "0"), dict(kind="comment-author", value="Café & Co.")),
    "same-author": (("comment", "0"), dict(kind="comment-author", value="Original")),
    "initials-set": (("comment", "0"), dict(kind="comment-initials", value="CC")),
    "initials-clear": (("comment", "0"), dict(kind="comment-initials", value=None)),
    "date-set": (("comment", "0"), dict(kind="comment-date", value="2030-02-03T04:05:06Z")),
    "date-clear": (("comment", "0"), dict(kind="comment-date", value=None)),
}


def make_entry_source(path, comments=1):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    paragraph = document.add_paragraph("Body unchanged")

    def element(tag, **attributes):
        node = OxmlElement(f"w:{tag}")
        for key, value in attributes.items():
            node.set(qn(f"w:{key}"), value)
        return node

    for kind in ("footnote", "endnote"):
        # python-docx cannot author notes: assemble their XML and relationships
        # with its OPC package API. Neither path is the usual Word default.
        plural = f"{kind}s"
        root = element(plural)
        notes = [("-1", "separator"), ("007", None)]
        if kind == "footnote":
            notes.append(("8", None))
        for ident, note_type in notes:
            entry = element(kind, id=ident)
            root.append(entry)
            p, run = element("p"), element("r")
            entry.append(p)
            p.append(run)
            if note_type:
                entry.set(qn("w:type"), note_type)
                run.append(element(note_type))
                continue
            run.append(element(f"{kind}Ref"))
            text_run = element("r")
            text = element("t")
            text.text = f"{kind} {ident}"
            text_run.append(text)
            p.append(text_run)
            if kind == "endnote":
                tail, tail_run, tail_text = element("p"), element("r"), element("t")
                tail_text.text = "Original endnote tail"
                tail_run.append(tail_text)
                tail.append(tail_run)
                entry.append(tail)
        paragraph.add_run()._r.append(element(f"{kind}Reference", id="007"))
        part = Part(
            PackURI(f"/word/notes/{plural}.xml"),
            f"application/vnd.openxmlformats-officedocument.wordprocessingml.{plural}+xml",
            etree.tostring(root, encoding="UTF-8", xml_declaration=True, standalone=True),
        )
        document.part.relate_to(part, getattr(RELATIONSHIP_TYPE, plural.upper()))

    comment = document.add_comment(paragraph.runs[0], text="First comment",
                                   author="Original", initials="OO")
    comment._element.set(qn("w:date"), "2025-01-02T03:04:05Z")
    comment.add_paragraph("Opaque symbol").add_run()._r.append(
        element("sym", font="Symbol", char="F041"),
    )
    table = document.add_table(1, 1)
    table.cell(0, 0).text = "Opaque table"
    comment._element.append(deepcopy(table._tbl))
    table._tbl.getparent().remove(table._tbl)
    for i in range(1, comments):
        document.comments.add_comment(text=f"Extra {i}", author="Extra", initials="E")
    document.save(path)
    return path.read_bytes()


def entry_operation(anchor, edit):
    return dict(kind="edit-entry", anchor=anchor, edit=edit)


async def entry_page(client, document, **view):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="entries", **view))


def entry_key(entry):
    return entry["kind"], entry["id"]


def mixed_edits(seen, mode):
    first, before_table, _, tail, table = seen["body"]
    assert [b["type"] for b in seen["body"]] == ["paragraph"] * 4 + ["table"]
    assert [b["text"] for b in seen["body"][:4]] == [
        "Original suffix", "Table goes next", "", "Keep tail drop me",
    ]
    title_run = next(run for run in first["runs"] if run["text"] == "Original")
    drop_run = next(run for run in tail["runs"] if run["text"] == " drop me")
    assert first["has_opaque_content"]
    (opaque_run,) = [run for run in first["runs"] if run["has_opaque_content"]]
    assert not opaque_run.get("anchor")
    assert [r["text"] for r in first["runs"] if r.get("anchor")] == ["Original", " suffix"]
    assert [(c["row_index"], c["cell_index"]) for c in table["cells"]] == [(0, 0), (0, 1)]
    cell = table["cells"][1]
    slots = {(h["section_index"], h["variant"]): h for h in seen["headers"]}
    shared = slots[0, "default"]
    assert slots[1, "default"]["part_name"] == shared["part_name"]
    assert slots[1, "default"]["anchor"] != shared["anchor"]
    assert shared["shared"] and slots[1, "default"]["inherited"]
    assert slots[0, "first"]["part_name"] != shared["part_name"]
    return [
        dict(kind="alignment", anchor=first["anchor"], alignment="center"),
        dict(kind="insert-table", anchor=before_table["anchor"], rows=[["new A", "new B"]]),
        dict(kind="insert-paragraph", anchor=tail["anchor"], text="Inserted & placed"),
        dict(kind="run-text", anchor=title_run["anchor"], text=" Edited "),
        dict(kind="run-remove", anchor=drop_run["anchor"]),
        dict(kind="cell-alignment", anchor=cell["anchor"], alignment="bottom"),
        dict(kind="header-image", anchor=shared["anchor"], mode=mode, bytes=list(PNG.read_bytes())),
    ], shared


async def test_discovery(launch):
    async with launch() as client:
        listing = await client.list_tools()
        tools = {tool.name: tool for tool in listing.tools}
        assert len(listing.tools) == 6 and tools.keys() == EXPECTED_TOOLS.keys()
        assert listing.nextCursor is None
        for name, tool in tools.items():
            assert tool.description and "example" in tool.description.lower()
            schema = tool.inputSchema
            assert schema["type"] == "object"
            validator_for(schema).check_schema(schema)
            assert set(schema.get("required", [])) == EXPECTED_TOOLS[name]
            assert schema["additionalProperties"] is False
            for field in EXPECTED_TOOLS[name]:
                assert schema["properties"][field]["type"] == "string"

        declared_paragraphs = tools["create_document"].inputSchema["properties"]["paragraphs"]
        assert declared_paragraphs["type"] == "array"
        assert declared_paragraphs["items"]["type"] == "string"
        preview = tools["preview_edits"].inputSchema
        edits_schema = preview["properties"]["edits"]
        assert edits_schema["type"] == "array" and edits_schema["minItems"] == 1
        items = edits_schema["items"]
        variants = items.get("oneOf") or items["anyOf"]
        kinds = {v["properties"]["kind"]["const"]: v for v in variants}
        expected = {
            "alignment": {"alignment"}, "insert-paragraph": {"text"}, "insert-table": {"rows"},
            "run-text": {"text"}, "run-remove": set(), "cell-alignment": {"alignment"},
            "header-image": {"mode", "bytes"}, "text-range": {"start", "end", "text"},
            "insert-text": {"offset", "text"}, "resource": {"edit"}, "block": {"edit"},
            "insert-rich-paragraph": {"paragraph"}, "field": {"edit"}, "inline": {"edit"},
            "insert-rich-table": {"table"},
            "table-structure": {"edit"},
            "paired-range": {"id"},
            "control": {"edit"},
        }
        assert len(variants) == len(expected) and kinds.keys() == expected.keys()
        for kind, variant_schema in kinds.items():
            anchor = set() if kind in {"resource", "paired-range", "control"} else {"anchor"}
            assert set(variant_schema["required"]) == {"kind"} | anchor | expected[kind]
            assert variant_schema["additionalProperties"] is False
            if anchor:
                assert variant_schema["properties"]["anchor"]["type"] == "string"
        assert set(kinds["alignment"]["properties"]["alignment"]["enum"]) == {
            "left", "center", "right", "justify", "distribute", "justify-medium",
            "justify-high", "justify-low", "thai-justify",
        }
        assert set(kinds["cell-alignment"]["properties"]["alignment"]["enum"]) == {
            "top", "center", "bottom", "both",
        }
        assert set(kinds["header-image"]["properties"]["mode"]["enum"]) == {"shared", "section-local"}
        rows = kinds["insert-table"]["properties"]["rows"]
        assert rows["type"] == rows["items"]["type"] == "array"
        assert rows["minItems"] == rows["items"]["minItems"] == 1
        assert rows["items"]["items"]["type"] == "string"
        byte = kinds["header-image"]["properties"]["bytes"]["items"]
        assert (byte["type"], byte["minimum"], byte["maximum"]) == ("integer", 0, 255)
        for kind, coordinates in [("text-range", ("start", "end")), ("insert-text", ("offset",))]:
            properties = kinds[kind]["properties"]
            assert properties["text"]["type"] == "string"
            for coordinate in coordinates:
                assert properties[coordinate]["type"] == "integer"
                assert properties[coordinate]["minimum"] == 0
        validate_preview = validator_for(preview)(preview)
        assert validate_preview.is_valid({"document": "doc", "edits": [
            {"kind": "run-remove", "anchor": "run"},
        ]})
        assert not validate_preview.is_valid({"document": "doc", "edits": [
            {"kind": "run-remove", "anchor": "run", "text": "unknown"},
        ]})
        assert validate_preview.is_valid({"document": "doc", "edits": [
            {"kind": "text-range", "anchor": "paragraph", "start": 1, "end": 2, "text": ""},
            {"kind": "insert-text", "anchor": "paragraph", "offset": 0, "text": "😀"},
        ]})
        assert not validate_preview.is_valid({"document": "doc", "edits": [
            {"kind": "insert-text", "anchor": "paragraph", "offset": -1, "text": "😀"},
        ]})
        with pytest.raises(McpError):
            await client.call_tool("not_a_rocx_tool", {})


async def test_entry_discovery(launch):
    async with launch() as client:
        tools = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}

    inspect = validator_for(tools["inspect_document"])(tools["inspect_document"])
    assert inspect.is_valid({"document": "doc"})
    for view in [
        {"kind": "entries"},
        {"kind": "entries", "limit": 1},
        {"kind": "entries", "limit": 100, "cursor": "next"},
    ]:
        assert inspect.is_valid({"document": "doc", "view": view})
    for view in [
        None, {}, {"kind": "other"}, {"kind": "entries", "extra": 1},
        {"kind": "entries", "limit": 0}, {"kind": "entries", "limit": 101},
        {"kind": "entries", "limit": 1.5}, {"kind": "entries", "cursor": 7},
    ]:
        assert not inspect.is_valid({"document": "doc", "view": view}), view

    preview = validator_for(tools["preview_edits"])(tools["preview_edits"])
    request = {"document": "doc"}
    ordinary = dict(request, edits=[{"kind": "run-remove", "anchor": "run"}])
    assert preview.is_valid(ordinary)
    for case in ENTRY_CASES.values():
        target, edit = case
        operation = entry_operation("entry", edit)
        scoped = dict(request, operation=operation)
        assert preview.is_valid(scoped), (target, edit)
        assert not preview.is_valid(dict(ordinary, operation=operation))
        assert not preview.is_valid({"operation": operation})
        assert not preview.is_valid(dict(request, operation=dict(operation, extra=True)))
        assert not preview.is_valid(dict(request, operation=entry_operation(
            "entry", dict(edit, extra=True),
        )))
        for member in edit:
            invalid = {key: value for key, value in edit.items() if key != member}
            assert not preview.is_valid(dict(request, operation=entry_operation("entry", invalid)))
    for invalid in [
        request,
        dict(request, edits=[]),
        dict(request, operation=None),
        dict(request, operation={"kind": "unknown", "anchor": "entry", "edit": {}}),
        dict(request, operation=entry_operation("entry", {"kind": "remove-paragraph", "index": -1})),
        dict(request, operation=entry_operation("entry", {"kind": "comment-author", "value": None})),
    ]:
        assert not preview.is_valid(invalid), invalid


async def test_entry_paging_native_inspection_and_identity(launch, tmp_path):
    source = tmp_path / "entries.docx"
    make_entry_source(source, comments=103)
    native = json.loads(oracle(tmp_path / "native-entries.json", "inspect-entries", source))
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        other = (await call(client, "open_document", path=source.name))["document"]
        empty = (await call(client, "create_document"))["document"]
        assert await entry_page(client, empty) == {
            "document": empty, "entries": [], "next_cursor": None,
        }
        default = await entry_page(client, document)
        assert len(default["entries"]) == 25
        assert default["next_cursor"]
        first = await entry_page(client, document, limit=1)
        assert len(first["entries"]) == 1
        first_entry, = first["entries"]
        assert entry_key(first_entry) == ("footnote", "-1")
        assert first_entry["note_type"] == "separator" and "anchor" not in first_entry
        cursor = first["next_cursor"]
        assert isinstance(cursor, str) and cursor
        assert await entry_page(client, document, limit=1) == first
        second = await entry_page(client, document, limit=100, cursor=cursor)
        assert second["document"] == first["document"] == document
        assert len(second["entries"]) == 100
        assert await entry_page(client, document, limit=100, cursor=cursor) == second
        third = await entry_page(client, document, limit=100, cursor=second["next_cursor"])
        assert third["document"] == document and third["next_cursor"] is None
        entries = first["entries"] + second["entries"] + third["entries"]
        assert len(entries) == 108 and default["entries"] == entries[:len(default["entries"])]
        default_next = await entry_page(client, document, cursor=default["next_cursor"])
        assert default_next["entries"] == entries[25:50]
        assert [{k: v for k, v in e.items() if k != "anchor"} for e in entries] == native

        by_id = {entry_key(entry): entry for entry in entries}
        assert {entry_key(e) for e in entries if "anchor" not in e} == {
            ("footnote", "-1"), ("endnote", "-1"),
        }
        anchors = [e["anchor"] for e in entries if "anchor" in e]
        assert all(isinstance(a, str) and a for a in anchors)
        assert len(set(anchors)) == 106
        assert by_id["footnote", "007"]["part_name"] == "word/notes/footnotes.xml"
        assert by_id["endnote", "007"]["part_name"] == "word/notes/endnotes.xml"
        comment = by_id["comment", "0"]
        assert comment["blocks"] == [
            # python-docx adds an annotation reference to the initial comment paragraph.
            {"index": 0, "type": "paragraph", "text": "First comment", "has_opaque_content": True},
            {"index": 1, "type": "paragraph", "text": "Opaque symbol", "has_opaque_content": True},
            {"index": 2, "type": "opaque", "name": "w:tbl"},
        ]
        assert by_id["endnote", "007"]["blocks"][1] == {
            "index": 1, "type": "paragraph", "text": "Original endnote tail",
            "has_opaque_content": False,
        }
        for view, category in [
            (dict(kind="entries", cursor=cursor), "wrong_owner"),
            (dict(kind="entries", cursor="never issued"), "invalid_cursor"),
            (dict(kind="entries", limit=0), "invalid_input"),
            (dict(kind="entries", limit=101), "invalid_input"),
        ]:
            await failure(client, "inspect_document", category, document=other, view=view)
        foreign = await entry_page(client, other, limit=100)
        assert not set(anchors) & {e.get("anchor") for e in foreign["entries"]}
        assert (await entry_page(client, document, limit=100))["entries"] == entries[:100]
        await call(client, "close_document", document=document)
        await failure(client, "inspect_document", "invalid_document", document=document,
                      view=dict(kind="entries", cursor=cursor))


def assert_entry_package_delta(source, output, target, case):
    kind, ident = target
    owner = f"word/notes/{kind}s.xml" if kind != "comment" else "word/comments.xml"
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == (
        set() if case == "same-author" else {owner}
    )
    old_entries, new_entries = [
        {entry.get(qn("w:id")): entry for entry in ElementTree.fromstring(package[owner])}
        for package in (before, after)
    ]
    assert new_entries.keys() == old_entries.keys() - ({ident} if case == "delete-unused" else set())
    for untouched in old_entries.keys() - {ident}:
        assert ElementTree.tostring(new_entries[untouched]) == ElementTree.tostring(
            old_entries[untouched],
        )

    document = docx.Document(output)
    assert [p.text for p in document.paragraphs] == ["Body unchanged"]
    comment = document.comments.get(0)
    assert [p.text for p in comment.paragraphs] == ["First comment", "Opaque symbol"]
    assert comment._element.xpath("w:p/w:r/w:sym/@w:char") == ["F041"]
    assert comment._element.xpath("w:tbl//w:t/text()") == ["Opaque table"]
    if kind == "comment":
        old, new = old_entries[ident], new_entries[ident]
        assert [ElementTree.tostring(node) for node in old] == [
            ElementTree.tostring(node) for node in new
        ]
        assert comment.author == ("Café & Co." if case == "author" else "Original")
        assert new.get(qn("w:initials")) == {
            "initials-clear": None, "initials-set": "CC",
        }.get(case, "OO")
        date = {"date-clear": None, "date-set": "2030-02-03T04:05:06Z"}.get(
            case, "2025-01-02T03:04:05Z",
        )
        assert new.get(qn("w:date")) == date
        if date is not None:
            assert comment.timestamp == (
                datetime(2030, 2, 3, 4, 5, 6, tzinfo=timezone.utc) if case == "date-set"
                else datetime(2025, 1, 2, 3, 4, 5, tzinfo=timezone.utc)
            )
    elif case != "delete-unused":
        entry = new_entries[ident]
        assert entry.get(qn("w:id")) == "007"
        assert entry[0][0][0].tag == qn(f"w:{kind}Ref")
        assert ["".join(t.text for t in p.iter(qn("w:t"))) for p in entry] == {
            "replace-first": ["New footnote 😀"],
            "insert-last": ["endnote 007", "Original endnote tail", "New endnote & tail"],
            "remove-last": ["endnote 007"],
        }[case]


@pytest.mark.parametrize("case", ENTRY_CASES)
async def test_entry_scoped_native_bytes_and_preservation(launch, tmp_path, case):
    source, output = [tmp_path / name for name in ("entries.docx", "copy.docx")]
    original = make_entry_source(source)
    target, edit = ENTRY_CASES[case]
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await entry_page(client, document, limit=100)
        entry = next(e for e in seen["entries"] if entry_key(e) == target)
        operation = entry_operation(entry["anchor"], edit)
        draft = await call(client, "preview_edits", document=document, operation=operation)
        assert draft["document"] == document and draft["changes"] is None

        # Retain a different preview before saving the selected scoped bytes.
        comment = next(e for e in seen["entries"] if entry_key(e) == ("comment", "0"))
        alternative = await call(client, "preview_edits", document=document, operation=entry_operation(
            comment["anchor"], dict(kind="comment-author", value="Other preview"),
        ))
        assert draft["preview"] != alternative["preview"]
        assert draft["output_snapshot"] != alternative["output_snapshot"]
        assert list(tmp_path.iterdir()) == [source]
        saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())
        assert await entry_page(client, document, limit=100) == seen
        reopened = await call(client, "open_document", path=output.name)
        assert reopened["snapshot"] == saved["output_snapshot"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened["document"],
                      operation=operation)
        reopened_entries = (await entry_page(client, reopened["document"], limit=100))["entries"]
        native_inspection = json.loads(oracle(
            tmp_path / "native-entries.json", "inspect-entries", output,
        ))
        assert [{k: v for k, v in e.items() if k != "anchor"} for e in reopened_entries] == (
            native_inspection
        )
        await call(client, "close_document", document=document)
        await failure(client, "save_copy", "invalid_document", document=document,
                      preview=draft["preview"], output="closed.docx")
        assert not (tmp_path / "closed.docx").exists()

    assert source.read_bytes() == original
    assert output.read_bytes() == oracle(tmp_path / "native.docx", "entry", source, case)
    assert_entry_package_delta(source, output, target, case)
    validate(source, output)


async def test_entry_rejection_retry_and_foreign_handles(launch, tmp_path):
    source = tmp_path / "entries.docx"
    original = make_entry_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        other = (await call(client, "open_document", path=source.name))["document"]
        compact = await call(client, "inspect_document", document=document)
        body = compact["body"][0]["anchor"]
        page = await entry_page(client, document, limit=100)
        entry_anchors = {entry_key(e): e["anchor"] for e in page["entries"] if "anchor" in e}
        live = entry_anchors["footnote", "007"]
        comment = entry_anchors["comment", "0"]
        delete = ENTRY_CASES["delete-unused"][1]
        delete_live = entry_operation(live, delete)
        for target, edit in [
            (live, delete),
            (live, dict(kind="remove-paragraph", index=0)),
            (live, dict(kind="insert-paragraph", index=2, text="out of bounds")),
            (live, dict(kind="comment-author", value="not a comment")),
            (comment, dict(kind="remove-paragraph", index=2)),  # Opaque table.
            (comment, dict(kind="comment-date", value="not a timestamp")),
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          operation=entry_operation(target, edit))
        for edit in [
            dict(kind="comment-initials"), dict(kind="comment-date"),
            dict(kind="comment-author", value=None), dict(kind="remove-paragraph", index=-1),
            dict(kind="delete-unused-note", extra=True),
        ]:
            await failure(client, "preview_edits", "invalid_input", document=document,
                          operation=entry_operation(comment, edit))
        await failure(client, "preview_edits", "invalid_input", document=document)
        ordinary = [dict(kind="alignment", anchor=body, alignment="center")]
        for edits in ([], ordinary):
            await failure(client, "preview_edits", "invalid_input", document=document,
                          edits=edits, operation=delete_live)
        await failure(client, "preview_edits", "wrong_owner", document=other, operation=delete_live)
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      operation=entry_operation(body, delete))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[dict(ordinary[0], anchor=live)])
        await failure(client, "preview_edits", "invalid_anchor", document=document,
                      operation=entry_operation("never issued", delete))
        assert list(tmp_path.iterdir()) == [source] and source.read_bytes() == original
        assert await entry_page(client, document, limit=100) == page

        # A valid edit of the *same* live note still works on its already issued anchor.
        retry = await call(client, "preview_edits", document=document,
                           operation=entry_operation(live, ENTRY_CASES["replace-first"][1]))
        await failure(client, "save_copy", "wrong_owner", document=other, preview=retry["preview"],
                      output="foreign.docx")
        await call(client, "save_copy", document=document, preview=retry["preview"],
                   output="retry.docx")
        await failure(client, "save_copy", "destination_exists", document=document,
                      preview=retry["preview"], output="retry.docx")
        assert not (tmp_path / "foreign.docx").exists()
        retry_output = tmp_path / "retry.docx"
        assert retry_output.read_bytes() == oracle(
            tmp_path / "native.docx", "entry", source, "replace-first",
        )


@pytest.mark.parametrize("mode", ["shared", "section-local"])
async def test_mixed_native_bytes_and_preservation(launch, tmp_path, mode):
    source = tmp_path / "source.docx"
    original = make_source(source)
    async with launch() as client:
        opened = await call(client, "open_document", path=source.name)
        assert opened["snapshot"] == digest(original)
        document = opened["document"]
        seen = await call(client, "inspect_document", document=document)
        assert seen == await call(client, "inspect_document", document=document)
        edits, header = mixed_edits(seen, mode)
        for malformed in [
            [],
            [dict(edits[0], kind="unknown")],
            [dict(edits[0], alignment="middle")],
            [dict(edits[1], rows=[["a"], ["b", "c"]])],
            [dict(edits[2], text=None)],
            [dict(edits[4], text="extra")],
            [dict(edits[6], bytes=[256])],
        ]:
            await failure(client, "preview_edits", "invalid_input", document=document,
                          edits=malformed)
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[dict(edits[4], anchor=seen["body"][0]["anchor"])])
        # The text-only run is a candidate, but removing it would discard unmodeled attributes.
        suffix = next(run for run in seen["body"][0]["runs"] if run["text"] == " suffix")
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      edits=[dict(kind="run-remove", anchor=suffix["anchor"])])
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      edits=[*edits, dict(edits[4], kind="run-text", text="conflict")])
        assert list(tmp_path.iterdir()) == [source]
        assert source.read_bytes() == original
        preview = await call(client, "preview_edits", document=document, edits=edits)
        assert preview["document"] == document
        report = preview["changes"]
        assert report["changes"] == [{"target": 0, "before": None, "after": "Center"}]
        assert report["table_insertions"] == [{"target": 1, "rows": 1, "columns": 2}]
        assert report["paragraph_insertions"] == [3]
        assert report["run_text"] == [
            {"target": 0, "inline_index": 0, "before": "Original", "after": " Edited "},
        ]
        assert report["run_removals"] == [
            {"target": 3, "inline_index": 1, "before": " drop me"},
        ]
        assert report["cell_alignments"] == [
            {"target": 4, "row_index": 0, "cell_index": 1, "before": None, "after": "bottom"},
        ]
        (image_change,) = report["header_images"]
        assert image_change["mode"] == mode
        assert image_change["source_part"] == header["part_name"]
        assert image_change["affected_sections"] == ([0, 1] if mode == "shared" else [0])
        assert image_change["restored_section"] == (None if mode == "shared" else 1)
        assert list(tmp_path.iterdir()) == [source]
        saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                           output="copy.docx")
        assert seen == await call(client, "inspect_document", document=document)
        assert (await call(client, "close_document", document=document))["closed"] == document

    output = tmp_path / "copy.docx"
    assert Path(saved["output"]).name == output.name
    assert source.read_bytes() == original
    assert saved["output_snapshot"] == preview["output_snapshot"] == digest(output.read_bytes())
    expected = oracle(tmp_path / "native.docx", "mixed", source, PNG, mode)
    assert output.read_bytes() == expected

    document = docx.Document(output)
    assert [b.rows[0].cells[0].text if isinstance(b, Table) else b.text
            for b in document.iter_inner_content()] == [
        " Edited  suffix", "Table goes next", "new A", "", "Keep tail", "Inserted & placed",
        "left cell",
    ]
    first = document.paragraphs[0]
    assert first.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert first.runs[0].bold is True and first.runs[0].text == " Edited "
    assert first.runs[1]._r.get(qn("w:rsidR")) == "00A10B20"
    assert first._p.xpath("w:r/w:sym/@w:char") == ["F041"]
    assert [c.text for c in document.tables[0].rows[0].cells] == ["new A", "new B"]
    cell = document.tables[1].cell(0, 1)
    assert (cell.text, cell.vertical_alignment) == ("target cell", WD_CELL_VERTICAL_ALIGNMENT.BOTTOM)
    source_header = header["part_name"].lstrip("/")
    for i, section in enumerate(document.sections):
        h = section.header
        assert h.paragraphs[0].text == "Shared header"
        embeds = h._element.xpath(".//a:blip/@r:embed")
        if i == 0 or mode == "shared":
            (rid,) = embeds
            media = h.part.related_parts[rid]
            assert media.blob == PNG.read_bytes()
            assert media.partname not in {p.partname for p in document.part.related_parts.values()}
        else:
            assert not embeds
            assert str(h.part.partname).lstrip("/") == source_header
    assert document.sections[0].first_page_header.paragraphs[0].text == "First-page header"

    original_parts, output_parts = parts(source), parts(output)
    allowed_changes = {"word/document.xml", "[Content_Types].xml"}
    if mode == "shared":
        allowed_changes.add(source_header)
        assert str(document.sections[0].header.part.partname).lstrip("/") == source_header
    else:
        allowed_changes.add("word/_rels/document.xml.rels")
        assert str(document.sections[0].header.part.partname).lstrip("/") != source_header
    assert original_parts.keys() <= output_parts.keys()
    for name in original_parts.keys() - allowed_changes:
        assert original_parts[name] == output_parts[name], name
    validate(source, output)


async def test_scalar_edits_native_bytes_and_retry(launch, tmp_path):
    source = tmp_path / "scalar.docx"
    output = tmp_path / "scalar-copy.docx"
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    formatted = fixture.add_paragraph()
    formatted.add_run("A😀B").bold = True
    formatted.add_run("CDe").italic = True
    formatted.add_run("XY").underline = True
    fixture.add_paragraph()
    fixture.add_paragraph("Do not\tedit")
    fixture.save(source)
    original = source.read_bytes()

    async with launch() as client:
        opened = await call(client, "open_document", path=source.name)
        document = opened["document"]
        assert opened["snapshot"] == digest(original)
        seen = await call(client, "inspect_document", document=document)
        plain, empty, mixed = seen["body"]
        assert plain["text"] == "A😀BCDeXY" and empty["text"] == ""
        assert [p["scalar_length"] for p in (plain, empty, mixed)] == [8, 0, None]
        # Unsorted edits refer to original scalar positions, not UTF-8 bytes or UTF-16 units.
        replace = dict(kind="text-range", anchor=plain["anchor"], start=4, end=7, text="🦊&")
        point = dict(kind="insert-text", anchor=plain["anchor"], offset=3, text=" + ")
        edits = [
            replace,
            dict(kind="insert-text", anchor=empty["anchor"], offset=0, text="runless"),
            point,
            dict(kind="text-range", anchor=plain["anchor"], start=0, end=1, text=""),
            dict(kind="insert-text", anchor=plain["anchor"], offset=plain["scalar_length"], text="!"),
        ]

        for edit in [dict(replace, start=-1), dict(point, offset=1.5)]:
            await failure(client, "preview_edits", "invalid_input", document=document, edits=[edit])
        for edit in [replace, point]:
            await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                          edits=[dict(edit, anchor=plain["runs"][0]["anchor"])])
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=[*edits, dict(edit, anchor=mixed["anchor"])])
        for edit in [
            dict(replace, start=2, end=2),
            dict(replace, start=7, end=6),
            dict(replace, end=9),  # 9 would be in bounds if lengths used UTF-16 units.
            dict(point, offset=9),
            dict(point, text=""),
            dict(point, text="\n"),
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=[edit])
        for conflicting in [
            dict(point, offset=5),  # Strictly inside the cross-run replacement.
            point,  # Duplicate insertion point.
            dict(kind="run-text", anchor=plain["runs"][1]["anchor"], text="conflict"),
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=[*edits, conflicting])
        assert list(tmp_path.iterdir()) == [source]
        assert source.read_bytes() == original
        assert await call(client, "inspect_document", document=document) == seen

        preview = await call(client, "preview_edits", document=document, edits=edits)
        assert list(tmp_path.iterdir()) == [source]
        assert preview["changes"]["run_text"] == [
            {"target": 0, "inline_index": 0, "before": "A😀B", "after": "😀B"},
            {"target": 0, "inline_index": 1, "before": "CDe", "after": " + C🦊&"},
            {"target": 0, "inline_index": 2, "before": "XY", "after": "Y!"},
            {"target": 1, "inline_index": 0, "before": "", "after": "runless"},
        ]
        saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == preview["output_snapshot"] == digest(output.read_bytes())
        assert await call(client, "inspect_document", document=document) == seen
        reopened = await call(client, "open_document", path=output.name)
        assert reopened["snapshot"] == saved["output_snapshot"]
        edited_body = (await call(client, "inspect_document", document=reopened["document"]))["body"]
        assert [p["text"] for p in edited_body[:2]] == ["😀B + C🦊&Y!", "runless"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened["document"],
                      edits=[replace])

    assert source.read_bytes() == original
    assert output.read_bytes() == oracle(tmp_path / "native.docx", "scalar", source)
    formatted, formerly_empty, untouched = docx.Document(output).paragraphs
    assert [r.text for r in formatted.runs] == ["😀B", " + C🦊&", "Y!"]
    assert formatted.runs[0].bold and formatted.runs[1].italic and formatted.runs[2].underline
    (new_run,) = formerly_empty.runs
    assert new_run.text == "runless" and new_run._r.rPr is None
    assert untouched.text == "Do not\tedit"
    original_parts, edited_parts = parts(source), parts(output)
    assert original_parts.keys() == edited_parts.keys()
    for name in original_parts.keys() - {"word/document.xml"}:
        assert original_parts[name] == edited_parts[name], name
    validate(source, output)


@pytest.mark.parametrize("paragraphs", [None, [], ["First café", "Second & last"]])
async def test_create_edit_and_save_initial_preview(launch, tmp_path, paragraphs):
    args = {} if paragraphs is None else {"paragraphs": paragraphs}
    expected_text = paragraphs or [""]
    async with launch() as client:
        created = await call(client, "create_document", **args)
        document = created["document"]
        assert created["snapshot"] == created["output_snapshot"]
        seen = await call(client, "inspect_document", document=document)
        assert seen["document"] == document
        assert seen["headers"] == []
        assert [(b["type"], b["text"]) for b in seen["body"]] == [
            ("paragraph", text) for text in expected_text
        ]
        draft = await call(client, "preview_edits", document=document, edits=[
            {"kind": "insert-paragraph", "anchor": seen["body"][0]["anchor"], "text": "New after first"},
        ])
        assert draft["preview"] != created["preview"]
        assert not list(tmp_path.iterdir())
        for name, selected in [("initial.docx", created), ("edited.docx", draft)]:
            result = await call(client, "save_copy", document=document, preview=selected["preview"],
                                output=name)
            assert result["output_snapshot"] == selected["output_snapshot"]
            assert digest((tmp_path / name).read_bytes()) == result["output_snapshot"]
        await call(client, "close_document", document=document)

    initial, edited = (tmp_path / n for n in ("initial.docx", "edited.docx"))
    # Core authoring consults the system clock (to the second) unless given a timestamp.
    # Supply the same clock value to the independent build; compare all bytes as-is.
    metadata = ElementTree.fromstring(parts(initial)["docProps/core.xml"])
    terms = "{http://purl.org/dc/terms/}"
    timestamp = metadata.findtext(terms + "created")
    assert timestamp and metadata.findtext(terms + "modified") == timestamp
    assert initial.read_bytes() == oracle(
        tmp_path / "native.docx", "create", timestamp, *(paragraphs or []),
    )
    original_doc = docx.Document(initial)
    edited_doc = docx.Document(edited)
    assert len(original_doc.sections) == len(edited_doc.sections) == 1
    assert [p.text for p in original_doc.paragraphs] == expected_text
    assert [p.text for p in edited_doc.paragraphs] == [
        expected_text[0], "New after first", *expected_text[1:],
    ]
    validate(initial, edited)


async def test_retained_selection_lifetime_and_exclusive_save(launch, tmp_path):
    source = tmp_path / "source.docx"
    original = make_source(source)
    corrupt = tmp_path / "corrupt.docx"
    corrupt.write_bytes(b"not a ZIP")
    occupied = tmp_path / "occupied.docx"
    occupied.write_bytes(b"sentinel bytes")

    async with launch() as client:
        await failure(client, "open_document", "invalid_document_data", path=corrupt.name)
        await failure(client, "open_document", "io", path="missing.docx")
        first = await call(client, "open_document", path=source.name)
        second = await call(client, "open_document", path=str(source))
        document, other = first["document"], second["document"]
        assert document != other
        assert first["snapshot"] == second["snapshot"] == digest(original)
        seen = await call(client, "inspect_document", document=document)
        other_seen = await call(client, "inspect_document", document=other)
        anchor = seen["body"][0]["anchor"]
        assert anchor != other_seen["body"][0]["anchor"]
        edit = dict(kind="alignment", anchor=anchor, alignment="center")
        await failure(client, "preview_edits", "wrong_owner", document=other, edits=[edit])
        await failure(client, "preview_edits", "invalid_anchor", document=document,
                      edits=[dict(edit, anchor="never issued")])
        a = await call(client, "preview_edits", document=document, edits=[edit])
        b = await call(client, "preview_edits", document=document,
                       edits=[dict(edit, alignment="right")])
        assert a["preview"] != b["preview"] and a["output_snapshot"] != b["output_snapshot"]
        assert source.read_bytes() == original

        selected = dict(document=document, preview=a["preview"])
        await failure(client, "save_copy", "wrong_owner", document=other,
                      preview=a["preview"], output="foreign.docx")
        await failure(client, "save_copy", "invalid_preview", document=document,
                      preview="never issued", output="invalid.docx")
        await failure(client, "save_copy", "invalid_input", **selected, output="replay.docx",
                      edits=[edit])
        await failure(client, "save_copy", "io", **selected, output="missing/copy.docx")
        await failure(client, "save_copy", "source_path", **selected, output=source.name)
        await failure(client, "save_copy", "destination_exists", **selected, output=occupied.name)
        assert occupied.read_bytes() == b"sentinel bytes"
        assert {p.name for p in tmp_path.iterdir()} == {source.name, corrupt.name, occupied.name}
        assert source.read_bytes() == original

        # Even an absent source remains protected. Saving must not need to read it.
        source.unlink()
        await failure(client, "save_copy", "source_path", **selected, output=str(source))
        assert not source.exists()
        await call(client, "save_copy", **selected, output="a.docx")
        replacement = make_source(source, title="New on disk")
        assert await call(client, "inspect_document", document=document) == seen
        reopened = await call(client, "open_document", path=source.name)
        assert reopened["snapshot"] == digest(replacement)
        current = await call(client, "inspect_document", document=reopened["document"])
        assert current["body"][0]["text"] == "New on disk suffix"
        await call(client, "save_copy", document=document, preview=b["preview"], output="b.docx")
        await call(client, "save_copy", **selected, output="a-again.docx")
        a_bytes, b_bytes = ((tmp_path / name).read_bytes() for name in ("a.docx", "b.docx"))
        assert digest(a_bytes) == a["output_snapshot"]
        assert digest(b_bytes) == b["output_snapshot"]
        assert (tmp_path / "a-again.docx").read_bytes() == a_bytes
        assert docx.Document(tmp_path / "a.docx").paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.CENTER
        assert docx.Document(tmp_path / "b.docx").paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.RIGHT

        attempts = []

        async def race_save(preview):
            result = await client.call_tool("save_copy", {
                "document": document, "preview": preview["preview"], "output": "race.docx",
            })
            attempts.append((preview, result))

        async with anyio.create_task_group() as group:
            group.start_soon(race_save, a)
            group.start_soon(race_save, b)
        ((winner, success),) = [(p, r) for p, r in attempts if not r.isError]
        (collision,) = [r for _, r in attempts if r.isError]
        assert_tool_failure(collision, "destination_exists")
        assert success.structuredContent["output_snapshot"] == winner["output_snapshot"]
        assert (tmp_path / "race.docx").read_bytes() == (a_bytes if winner == a else b_bytes)
        assert source.read_bytes() == replacement

        await call(client, "close_document", document=document)
        await failure(client, "inspect_document", "invalid_document", document=document)
        await failure(client, "save_copy", "invalid_document", **selected, output="closed.docx")
        await failure(client, "preview_edits", "wrong_owner", document=reopened["document"],
                      edits=[edit])

    async with launch() as restarted:
        await failure(restarted, "inspect_document", "invalid_document", document=other)
        await failure(restarted, "save_copy", "invalid_document", **selected, output="expired.docx")
        new = await call(restarted, "open_document", path=source.name)
        assert new["document"] not in {document, other, reopened["document"]}
        await failure(restarted, "preview_edits", "wrong_owner", document=new["document"],
                      edits=[edit])
    assert not (tmp_path / "closed.docx").exists() and not (tmp_path / "expired.docx").exists()


async def test_capacity_rejects_without_eviction_and_close_releases(launch, tmp_path):
    # Public limits: 16 live documents and 64 retained previews per session.
    async with launch() as client:
        live = [await call(client, "create_document") for _ in range(16)]
        await failure(client, "create_document", "capacity")
        document = live[0]["document"]
        anchor = (await call(client, "inspect_document", document=document))["body"][0]["anchor"]
        edits = [dict(kind="insert-paragraph", anchor=anchor, text="Retained preview")]
        previews = [
            await call(client, "preview_edits", document=document, edits=edits) for _ in range(48)
        ]
        assert len({p["preview"] for p in previews}) == 48
        await failure(client, "preview_edits", "capacity", document=document, edits=edits)

        await call(client, "close_document", document=live[-1]["document"])
        # Refill the freed preview slot. Creating another document needs a preview slot too.
        await call(client, "preview_edits", document=document, edits=edits)
        await failure(client, "create_document", "capacity")
        for name, preview in [("oldest.docx", live[0]), ("first-edit.docx", previews[0])]:
            await call(client, "save_copy", document=document, preview=preview["preview"], output=name)
            assert digest((tmp_path / name).read_bytes()) == preview["output_snapshot"]
        await call(client, "close_document", document=document)
        created = await call(client, "create_document")
        assert created["document"] not in {d["document"] for d in live}
