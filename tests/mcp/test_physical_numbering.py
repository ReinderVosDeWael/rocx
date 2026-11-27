"""Snapshot-owned numbering in physical story, entry, and body-cell paragraphs."""

import subprocess
from zipfile import ZipFile

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_fields import opened, word
from test_numbering import edit, level
from test_ranges import inspect, saved
from test_stdio import anyio_backend, call, executable, failure, launch, make_entry_source, parts, validate

pytestmark = pytest.mark.anyio
HEADER = "word/stories/selected.xml"
TARGETS = {"Cell target", "Header target", "Footer target", "footnote 007", "endnote 007", "First comment"}
REFERENCE = dict(id=90, level=0)


def write(path, payloads):
    with ZipFile(path, "w") as output:
        for name, data in payloads.items():
            output.writestr(name, data)


def fixture(path):
    make_entry_source(path)
    doc = docx.Document(path)
    section = doc.sections[0]
    section.header.paragraphs[0].text = "Header target"
    section.footer.paragraphs[0].text = "Footer target"
    start = section.header.add_paragraph("Protected start")
    end = section.header.add_paragraph("Protected end")
    start._p.append(word("bookmarkStart", id="42", name="AcrossParagraphs"))
    end._p.append(word("bookmarkEnd", id="42"))
    doc.add_table(1, 1).cell(0, 0).text = "Removed table"
    table = doc.add_table(2, 2)
    target = table.cell(0, 0)
    target.text = "Cell lead"
    target.add_table(1, 1).cell(0, 0).text = "Nested"
    target.paragraphs[-1].text = "Cell target"
    table.cell(0, 1).merge(table.cell(1, 1)).text = "Merge start"
    doc.add_section()
    doc.save(path)
    payloads = parts(path)
    payloads[HEADER] = payloads.pop("word/header1.xml")
    rels = "word/_rels/document.xml.rels"
    payloads[rels] = payloads[rels].replace(b'Target="header1.xml"', b'Target="stories/selected.xml"')
    types = "[Content_Types].xml"
    payloads[types] = payloads[types].replace(b"/word/header1.xml", b"/word/stories/selected.xml")
    write(path, payloads)


async def view(client, document, scope, **fields):
    return await inspect(client, document, "numbering-paragraphs", scope=scope, **fields)


async def all_rows(client, document, scope):
    result = await view(client, document, scope, limit=2)
    rows = result["paragraphs"]
    while result["next_cursor"] is not None:
        result = await view(client, document, scope, limit=2, cursor=result["next_cursor"])
        rows.extend(result["paragraphs"])
    return rows


async def targets(client, document):
    discovered = {}
    for scope in ("cells", "stories", "entries"):
        for row in await all_rows(client, document, scope):
            if row["text"] in TARGETS:
                assert row["text"] not in discovered
                discovered[row["text"]] = row
    assert discovered.keys() == TARGETS
    return discovered


async def output(client, document, source, action, edits):
    preview = await call(client, "preview_edits", document=document, edits=edits)
    dest = source.parent / f"{action}.docx"
    oracle_dest = source.parent / f"native-{action}.docx"
    await saved(client, document, preview, dest)
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_physical_numbering_oracle")
    subprocess.run([oracle, source, action, oracle_dest], check=True, timeout=20)
    assert dest.read_bytes() == oracle_dest.read_bytes()
    return dest


async def test_physical_numbering_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    valid = validator_for(schema)(schema).is_valid
    for scope in ("cells", "stories", "entries"):
        assert valid(dict(document="d", view=dict(kind="numbering-paragraphs", scope=scope, limit=100)))
    for fields in [{}, dict(scope="all"), dict(scope="cells", limit=101), dict(scope="cells", cursor=None)]:
        assert not valid(dict(document="d", view=dict(kind="numbering-paragraphs", **fields)))
    schema = schemas["preview_edits"]
    valid = validator_for(schema)(schema).is_valid
    for value in (REFERENCE, None):
        assert valid(dict(document="d", edits=[edit("physical-paragraph", anchor="a", value=value)]))
    for fields in [dict(anchor="a"), dict(value=None), dict(anchor="a", value=dict(id=90, level=9))]:
        assert not valid(dict(document="d", edits=[edit("physical-paragraph", **fields)]))


async def test_physical_numbering_mixed_native_bytes(launch, tmp_path):
    source = tmp_path / "source.docx"
    fixture(source)
    original = source.read_bytes()
    async with launch() as client:
        document = await opened(client, source)
        chosen = await targets(client, document)
        assert all(row["value"] is None for row in chosen.values())
        assert chosen["Header target"]["part_name"] == HEADER
        assert chosen["footnote 007"]["part_name"] == "word/notes/footnotes.xml"
        assert chosen["endnote 007"]["id"] == "007"
        assert chosen["Cell target"]["body_index"] == 2 and chosen["Cell target"]["block_index"] == 2
        head = await view(client, document, "stories", limit=1)
        cursor = head["next_cursor"]
        assert cursor is not None
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="numbering-paragraphs", scope="entries", cursor=cursor))
        story_rows = await all_rows(client, document, "stories")
        assert len(story_rows) == 4  # Two sections share the same physical parts.
        blocked = [row for row in story_rows if row["text"].startswith("Protected")]
        assert len(blocked) == 2 and all("unavailable" in row and "anchor" not in row for row in blocked)
        cells = await all_rows(client, document, "cells")
        assert not any(row["text"] == "Nested" for row in cells)
        continuation = next(row for row in cells if
                            (row["body_index"], row["row_index"], row["cell_index"]) == (2, 1, 1))
        assert "unavailable" in continuation and "anchor" not in continuation
        reserved = [row for row in await all_rows(client, document, "entries") if row["note_type"] == "separator"]
        assert len(reserved) == 2 and all("unavailable" in row and "anchor" not in row for row in reserved)
        body = (await call(client, "inspect_document", document=document))["body"]
        tables = (await inspect(client, document, "body-tables"))["body_tables"]
        by_index = {row["body_index"]: row["anchor"] for row in tables}
        other = await opened(client, source)
        foreign = (await view(client, other, "stories", limit=1))["paragraphs"][0]["anchor"]
        cell_anchor = chosen["Cell target"]["anchor"]
        for error, anchor in [
            ("wrong_anchor_type", body[0]["anchor"]),
            ("wrong_anchor_type", by_index[2]),
            ("wrong_owner", foreign),
            ("invalid_anchor", f"{document}/numbering-cell:2:1:1:0"),
        ]:
            await failure(client, "preview_edits", error, document=document,
                          edits=[edit("physical-paragraph", anchor=anchor, value=None)])
        await failure(client, "inspect_document", "wrong_anchor_type", document=document,
                      view=dict(kind="paragraph-numbering", anchor=cell_anchor))
        assert not (tmp_path / "assign.docx").exists()
        assign = [edit("physical-paragraph", anchor=row["anchor"], value=REFERENCE)
                  for row in chosen.values()] + [
            edit("add-instance", num_id=90, abstract_id=80),
            edit("add-abstract", abstract_id=80, levels=[level(0)]),
        ]
        for bad in [
            edit("physical-paragraph", anchor=cell_anchor, value=None),
            dict(kind="table-structure", anchor=by_index[2], edit=dict(kind="remove-table")),
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=assign + [bad])
        assigned = await output(client, document, source, "assign", assign + [
            dict(kind="table-structure", anchor=by_index[1], edit=dict(kind="remove-table")),
        ])
        reopened = await opened(client, assigned)
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="numbering-paragraphs", scope="stories", cursor=cursor))
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[assign[0]])
        readback = await targets(client, reopened)
        assert readback["Cell target"]["body_index"] == 1
        assert all(row["value"] == REFERENCE for row in readback.values())
        for live in ("Header target", "footnote 007", "Cell target"):
            await failure(client, "preview_edits", "rejected_batch", document=reopened, edits=[
                edit("remove-instance", num_id=90),
                *[edit("physical-paragraph", anchor=row["anchor"], value=None)
                  for text, row in readback.items() if text != live],
            ])
        cleared = await output(client, reopened, assigned, "clear", [
            edit("remove-abstract", abstract_id=80), edit("remove-instance", num_id=90),
            *[edit("physical-paragraph", anchor=row["anchor"], value=None) for row in readback.values()],
        ])
        reopened = await opened(client, cleared)
        assert all(row["value"] is None for row in (await targets(client, reopened)).values())

    independent = docx.Document(assigned)
    assert len(independent.tables) == 1
    assert independent.tables[0].cell(0, 0).paragraphs[-1]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["90"]
    assert independent.sections[0].header.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["90"]
    assert independent.sections[1].header.is_linked_to_previous
    assert source.read_bytes() == original
    for prior, after in [(source, assigned), (assigned, cleared)]:
        before, result = parts(prior), parts(after)
        assert before.keys() == result.keys()
        assert {name for name in before if before[name] != result[name]} == {
            "word/document.xml", "word/numbering.xml", HEADER, "word/footer1.xml",
            "word/notes/footnotes.xml", "word/notes/endnotes.xml", "word/comments.xml",
        }
    validate(source, assigned, cleared)


async def test_physical_numbering_isolates_unavailable_rows(launch, tmp_path):
    source = tmp_path / "malformed.docx"
    fixture(source)
    payloads = parts(source)
    footnotes = "word/notes/footnotes.xml"
    notes = etree.fromstring(payloads[footnotes])
    selected = next(node for node in notes if node.get(qn("w:id")) == "8")
    selected.set(qn("w:id"), "7")  # Same decimal ID as the existing '007'.
    payloads[footnotes] = etree.tostring(notes)
    body = etree.fromstring(payloads["word/document.xml"])
    lead = next(n for n in body.iter(qn("w:t")) if n.text == "Cell lead")
    props, number = word("pPr"), word("numPr")
    number.append(word("numId", val="1"))  # Valid source markup; not a complete direct reference.
    props.append(number)
    lead.getparent().getparent().insert(0, props)
    payloads["word/document.xml"] = etree.tostring(body)
    write(source, payloads)
    async with launch() as client:
        document = await opened(client, source)
        rows = await all_rows(client, document, "entries")
        ambiguous = [row for row in rows if row["kind"] == "footnote" and row["id"] in ("7", "007")]
        assert len(ambiguous) == 2 and all("unavailable" in row and "anchor" not in row for row in ambiguous)
        assert next(row for row in rows if row["text"] == "endnote 007")["value"] is None
        cells = {row["text"]: row for row in await all_rows(client, document, "cells")}
        assert "incomplete direct numbering reference" in cells["Cell lead"]["unavailable"]
        assert "value" not in cells["Cell lead"] and "anchor" not in cells["Cell lead"]
        assert cells["Cell target"]["value"] is None
