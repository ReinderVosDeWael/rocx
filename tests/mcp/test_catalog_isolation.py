"""Owner-scoped installed-client discovery works despite unrelated malformed catalogs."""

from copy import deepcopy
import json
import subprocess
from zipfile import ZipFile

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.oxml import OxmlElement, parse_xml
from docx.oxml.ns import qn
from docx.shared import Inches
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_physical_tables import RICH, edit, native_oracle, open_document, operation, page, save
from test_stdio import (
    anyio_backend, assert_tool_failure, call, executable, failure, launch, make_entry_source, parts,
)

pytestmark = pytest.mark.anyio
MAIN = "word/relocated-main.xml"
HEADER = "word/stories/relocated-header.xml"
FOOTNOTES = "word/notes/footnotes.xml"
MAIN_SCOPE = dict(kind="main")


@pytest.fixture
def catalog_source(tmp_path):
    clean = tmp_path / "clean.docx"
    make_entry_source(clean)
    document = docx.Document(clean)
    document.paragraphs[0].insert_paragraph_before("Editable body")
    document.part.partname = PackURI(f"/{MAIN}")
    outer = document.add_table(1, 1)
    outer.cell(0, 0).add_table(1, 1).cell(0, 0).text = "nested"
    header = document.sections[0].header
    header.part.partname = PackURI(f"/{HEADER}")
    header.add_table(1, 1, Inches(2)).cell(0, 0).text = "header"
    document.add_section()
    for relation in (RELATIONSHIP_TYPE.FOOTNOTES, RELATIONSHIP_TYPE.ENDNOTES):
        part = document.part.part_related_by(relation)
        notes = parse_xml(part.blob)
        for note in notes:
            if note.get(qn("w:id")) in ("007", "-1"):
                # The native reader exposes both tables in ordinary notes and skips reserved ones.
                note.append(deepcopy(outer._tbl))
        part._blob = etree.tostring(notes)
    document.save(clean)

    def corrupt(part):
        source = tmp_path / f"broken-{part.rsplit('/', 1)[1]}.docx"
        with ZipFile(source, "w") as archive:
            for name, data in parts(clean).items():
                # Valid XML in a relationship-selected part, but not the native story root.
                archive.writestr(name, etree.tostring(OxmlElement("w:p")) if name == part else data)
        return source

    return clean, corrupt(HEADER), corrupt(FOOTNOTES)


async def body(client, document):
    return await call(client, "inspect_document", document=document, view=dict(kind="body"))


def compare(source, output, case, changed):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_catalog_isolation_oracle")
    native = output.with_name(f"{case}-native.docx")
    subprocess.run([oracle, source, native, case], check=True, cwd=source.parent, timeout=20)
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {changed}


async def test_catalog_isolation_discovery(launch, catalog_source):
    clean, _, _ = catalog_source
    async with launch() as client:
        tool = next(tool for tool in (await client.list_tools()).tools
                    if tool.name == "inspect_document")
        schema = validator_for(tool.inputSchema)(tool.inputSchema)

        def accepts(kind, **options):
            return schema.is_valid(dict(document="doc", view=dict(kind=kind, **options)))

        assert accepts("body")
        assert not accepts("body", limit=1)
        assert accepts("physical-tables")
        assert accepts("physical-tables", scope=MAIN_SCOPE)
        assert accepts("physical-tables", scope=dict(kind="story", anchor="owner"))
        assert not accepts("physical-tables", scope=dict(kind="story"))
        assert accepts("physical-tables", scope=dict(kind="entry", anchor="owner"))
        assert not accepts("physical-tables", scope=dict(kind="entry"))
        assert not accepts("physical-tables", scope=None)
        assert not accepts("physical-tables", scope=dict(kind="main", anchor="owner"))

        document = await open_document(client, clean)
        isolated = await body(client, document)
        compact = await call(client, "inspect_document", document=document)
        assert isolated == {"document": document, "body": compact["body"]}
        assert compact["headers"]
        aggregate = await page(client, document, limit=1)
        scoped = await page(client, document, scope=MAIN_SCOPE, limit=1)
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="physical-tables", scope=MAIN_SCOPE,
                                cursor=aggregate["next_cursor"]))
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="physical-tables", cursor=scoped["next_cursor"]))


async def test_body_and_main_tables_ignore_malformed_header(launch, catalog_source):
    _, source, _ = catalog_source
    body_output = source.with_name("body-edit.docx")
    table_output = source.with_name("main-edit.docx")
    async with launch() as client:
        document = await open_document(client, source)
        await failure(client, "inspect_document", "inspection_failed", document=document)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="physical-tables"))
        entry_runs = await call(client, "inspect_document", document=document,
                                view=dict(kind="entry-runs"))
        entry_owner = entry_runs["entry_runs"][0]["owner"]["anchor"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="physical-tables", scope=dict(kind="entry", anchor=entry_owner)))
        edits = [dict(kind="run-text", anchor=f"{document}/run:0:0", text="Edited body 😀")]
        await failure(client, "preview_edits", "invalid_anchor", document=document, edits=edits)
        isolated = await body(client, document)
        assert set(isolated) == {"body", "document"}
        edits[0]["anchor"] = isolated["body"][0]["runs"][0]["anchor"]
        preview = await call(client, "preview_edits", document=document, edits=edits)
        await call(client, "save_copy", document=document, preview=preview["preview"],
                   output=body_output.name)

        first = await page(client, document, scope=MAIN_SCOPE, limit=1)
        (outer,) = first["physical_tables"]
        second = await page(client, document, scope=MAIN_SCOPE, cursor=first["next_cursor"])
        (nested,) = second["physical_tables"]
        assert second["next_cursor"] is None
        assert nested["depth"] == 1 and nested["path"][:-3] == outer["path"]
        owner = outer["owner"]
        assert owner == nested["owner"]
        assert owner["part_name"] == MAIN and owner["kind"] == "body"
        outer_edit, nested_edit = (edit(row, "layout", value="fixed") for row in (outer, nested))
        rejected = await client.call_tool("preview_edits", dict(
            document=document, operation=operation(owner, outer_edit, nested_edit),
        ))
        assert_tool_failure(rejected, "rejected_batch")
        assert "overlapping physical table targets" in rejected.structuredContent["message"]
        await save(client, document, table_output, operation(owner, nested_edit))

    compare(source, body_output, "body", MAIN)
    assert docx.Document(body_output).paragraphs[0].runs[0].text == "Edited body 😀"
    compare(source, table_output, "main", MAIN)
    assert not docx.Document(table_output).tables[0].cell(0, 0).tables[0].autofit


async def test_story_tables_ignore_malformed_entries(launch, catalog_source):
    _, _, source = catalog_source
    output = source.with_name("story-edit.docx")
    async with launch() as client:
        document = await open_document(client, source)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="entries"))
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="physical-tables"))
        stories = await call(client, "inspect_document", document=document,
                             view=dict(kind="stories"))
        header = stories["stories"][0]
        scope = dict(kind="story", anchor=header["anchor"])
        await failure(client, "inspect_document", "wrong_anchor_type", document=document,
                      view=dict(kind="physical-tables", scope=scope))
        scope["anchor"] = header["owner"]["anchor"]
        (table,) = (await page(client, document, scope=scope))["physical_tables"]
        assert table["owner"]["part_name"] == HEADER
        assert [slot["inherited"] for slot in table["owner"]["slots"]] == [False, True]

        main_owner = (await page(client, document, scope=MAIN_SCOPE))["physical_tables"][0]["owner"]
        change = edit(table, "layout", value="fixed")
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=operation(main_owner, change))
        await save(client, document, output, operation(table["owner"], change))

    compare(source, output, "story", HEADER)
    assert not docx.Document(output).sections[0].header.tables[0].autofit


async def test_entry_table_scopes_match_native(launch, catalog_source):
    source, _, _ = catalog_source
    native_output = source.with_name("entry-tables.json")
    async with launch() as client:
        document = await open_document(client, source)
        runs = await call(client, "inspect_document", document=document,
                          view=dict(kind="entry-runs"))
        inlines = await call(client, "inspect_document", document=document,
                             view=dict(kind="entry-inlines", limit=100))
        run_owners = {row["kind"]: row["owner"] for row in runs["entry_runs"]}
        inline_owners = {row["entry_kind"]: row["owner"] for row in inlines["entry_inlines"]}
        assert run_owners.keys() == inline_owners.keys() == {"comment", "footnote", "endnote"}

        entries = await call(client, "inspect_document", document=document,
                             view=dict(kind="entries"))
        definition = next(row["anchor"] for row in entries["entries"] if "anchor" in row)
        await failure(client, "inspect_document", "wrong_anchor_type", document=document,
                      view=dict(kind="physical-tables",
                                scope=dict(kind="entry", anchor=definition)))

        for kind, owner in run_owners.items():
            scope = dict(kind="entry", anchor=owner["anchor"])
            inline_scope = dict(kind="entry", anchor=inline_owners[kind]["anchor"])
            first = await page(client, document, scope=scope, limit=1)
            rows = first["physical_tables"]
            if first["next_cursor"] is not None:
                other = run_owners["endnote" if kind == "footnote" else "footnote"]
                await failure(client, "inspect_document", "invalid_cursor", document=document,
                              view=dict(kind="physical-tables", cursor=first["next_cursor"],
                                        scope=dict(kind="entry", anchor=other["anchor"])))
                # Both views identify the same physical owner and pagination position.
                rest = await page(client, document, scope=inline_scope, cursor=first["next_cursor"])
                rows += rest["physical_tables"]
                assert rest["next_cursor"] is None
            assert rows == (await page(client, document, scope=inline_scope))["physical_tables"]
            assert len(rows) == (1 if kind == "comment" else 2)
            part = owner["part_name"]
            assert all(row["owner"]["kind"] == kind and row["owner"]["part_name"] == part
                       for row in rows)
            projected = [
                dict(part=part, **{key: value for key, value in row.items()
                                   if key not in {"owner", "anchor"}})
                for row in rows
            ]
            assert projected == json.loads(native_oracle(source, native_output, "inspect", part))
            if kind == "comment":
                (table,) = rows
                output = source.with_name("comment-table-edit.docx")
                change = edit(table, "replace-cell", row=0, cell=0, blocks=RICH)
                await save(client, document, output, operation(table["owner"], change))
                assert output.read_bytes() == native_oracle(
                    source, source.with_name("comment-table-native.docx"), "rich", part,
                )
