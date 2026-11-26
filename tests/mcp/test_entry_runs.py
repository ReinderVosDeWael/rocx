"""Native physical text edits inside notes and comments."""

from xml.etree import ElementTree

import docx
from docx.oxml.ns import qn
import pytest

from test_stdio import (
    anyio_backend, call, failure, launch, make_entry_source, oracle, parts, pytestmark, validate,
)


@pytest.mark.parametrize("kind", ["footnote", "endnote", "comment"])
async def test_entry_story_text_in_one_physical_owner(launch, tmp_path, kind):
    source = tmp_path / "source.docx"
    output = tmp_path / "edited.docx"
    make_entry_source(source)
    expected = oracle(tmp_path / "native.docx", "entry-runs", source, kind)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        page = await call(client, "inspect_document", document=document,
                          view={"kind": "entry-runs", "limit": 1})
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view={"kind": "entries", "cursor": page["next_cursor"]})
        rows = page["entry_runs"]
        assert rows[0]["note_type"] == "separator"
        assert all("anchor" not in run for run in rows[0]["runs"])
        rows += (await call(client, "inspect_document", document=document, view={
            "kind": "entry-runs", "cursor": page["next_cursor"],
        }))["entry_runs"]
        selected = [row for row in rows if row["kind"] == kind
                    and row["note_type"] != "separator"]
        owner = selected[0]["owner"]
        assert all(row["owner"] == owner for row in selected)

        def first_text(row):
            return next(run["anchor"] for run in row["runs"] if "anchor" in run)

        edits = [{"anchor": first_text(row), "text": f"Revised {kind} {row['id']}"}
                 for row in selected]
        operation = {"kind": "edit-story-text", "anchor": owner["anchor"], "edits": edits}
        foreign = next(row for row in rows if row["kind"] != kind
                       and row["note_type"] != "separator")
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation={**operation, "edits": edits + [
                          {"anchor": first_text(foreign), "text": "Wrong part"},
                      ]})
        preview = await call(client, "preview_edits", document=document, operation=operation)
        assert preview["changes"] is None
        await call(client, "save_copy", document=document, preview=preview["preview"],
                   output=output.name)

    assert output.read_bytes() == expected
    before, after = parts(source), parts(output)
    changed = owner["part_name"]
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {changed}
    target = ElementTree.fromstring(after[changed])
    for row in selected:
        (entry,) = (entry for entry in target if entry.get(qn("w:id")) == row["id"])
        assert next(entry.iter(qn("w:t"))).text == f"Revised {kind} {row['id']}"
    readback = docx.Document(output)
    assert readback.paragraphs[0].text == "Body unchanged"
    if kind == "comment":
        assert readback.comments.get(0).paragraphs[0].text == "Revised comment 0"
    validate(output)
