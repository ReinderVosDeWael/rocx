"""Paired ranges use original body sites and preserve unrelated package parts."""

import json
import subprocess

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_fields import opened, word
from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio


def make_source(path):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.sections[0].footer.paragraphs[0].text = "untouched footer"
    document.add_table(1, 1).cell(0, 0).text = "untouched table"
    old = document.add_paragraph("old bookmark")
    old.paragraph_format.keep_with_next = True
    old.runs[0]._r.addprevious(word("bookmarkStart", id="+007", name="OldPlace"))
    old._p.append(word("bookmarkEnd", id="0007"))
    start = document.add_paragraph("before link ")
    link = word("hyperlink")
    link.set(qn("r:id"), start.part.relate_to(
        "https://example.test/range", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    run = word("r")
    run.append(word("t", "linked start"))
    link.append(run)
    start._p.append(link)
    document.add_paragraph("cross-paragraph end")
    comment = document.add_paragraph("old comment")
    document.add_comment(comment.runs, text="definition stays", author="Author", initials="A")
    document.add_paragraph("new comment target")
    document.add_paragraph("ordinary")
    proof = document.add_paragraph("proofing range")
    proof.runs[0]._r.addprevious(word("proofErr", type="spellStart"))
    proof._p.append(word("proofErr", type="spellEnd"))
    document.save(path)


def edit(id, bookmark=None, endpoints=None):
    result = dict(kind="paired-range", id=id)
    if bookmark is not None:
        result["bookmark"] = bookmark
    if endpoints is not None:
        result["endpoints"] = dict(first=endpoints[0], last=endpoints[1])
    return result


async def inspect(client, document, kind="range-markers", **view):
    return await call(client, "inspect_document", document=document, view=dict(kind=kind, **view))


async def anchors(client, document):
    return {
        (row["body_index"], row["inline_index"]): row["anchor"]
        for row in (await inspect(client, document, "body-inlines"))["body_inlines"]
        if "anchor" in row
    }


def native(source, action, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_range_oracle")
    result = subprocess.run(
        [oracle, source, action, output], check=True, cwd=source.parent, timeout=20,
        capture_output=True,
    )
    return json.loads(result.stdout) if result.stdout else None


async def saved(client, document, draft, output):
    result = await call(client, "save_copy", document=document, preview=draft["preview"],
                        output=output.name)
    assert result["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())


async def test_paired_range_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    inspect_schema = schemas["inspect_document"]
    view = validator_for(inspect_schema)(inspect_schema)
    assert view.is_valid(dict(document="doc", view=dict(kind="range-markers", limit=1)))
    assert not view.is_valid(dict(document="doc", view=dict(kind="range-markers", limit=101)))
    preview_schema = schemas["preview_edits"]
    preview = validator_for(preview_schema)(preview_schema)
    for change in [
        edit(0), edit(7, "OldPlace"), edit(7, "OldPlace", ("a", "b")),
        edit(0, endpoints=("a", "a")), edit(2**32 - 1),
    ]:
        assert preview.is_valid(dict(document="doc", edits=[change]))
    for change in [
        dict(kind="paired-range"), edit(-1), edit(2**32), edit("7"),
        dict(edit(7), bookmark=None), dict(edit(7), endpoints=None),
        dict(edit(7), endpoints=dict(first="a")),
        dict(edit(7), endpoints=dict(first="a", last="b", extra="c")),
    ]:
        assert not preview.is_valid(dict(document="doc", edits=[change]))


async def test_range_markers_preserve_ambiguous_source(launch, tmp_path):
    source = tmp_path / "ambiguous.docx"
    document = docx.Document()
    paragraph = document.add_paragraph()
    for i in range(50):
        paragraph._p.append(word("bookmarkStart", id=str(i), name=f"Range{i}"))
        paragraph._p.append(word("bookmarkEnd", id=str(i)))
    paragraph._p.append(word("bookmarkStart", id="+049"))  # Alias and missing name.
    paragraph._p.append(word("bookmarkEnd"))  # Missing ID.
    document.save(source)
    native_rows = tmp_path / "markers.json"
    native(source, "inspect", native_rows)
    async with launch() as client:
        handle = await opened(client, source)
        page = await inspect(client, handle, limit=100)
        assert len(page["range_markers"]) == 100
        tail = await inspect(client, handle, cursor=page["next_cursor"])
        assert tail["next_cursor"] is None
        keys = ["body_index", "inline_index", "kind", "id", "name"]
        rows = [[row[k] for k in keys] for row in page["range_markers"] + tail["range_markers"]]
        assert rows == json.loads(native_rows.read_bytes())
        assert rows[-2:] == [[0, 100, "bookmarkStart", "+049", None],
                             [0, 101, "bookmarkEnd", None, None]]
        await failure(client, "preview_edits", "rejected_batch", document=handle,
                      edits=[edit(49, "Range49")])


async def test_paired_range_lifecycle(launch, tmp_path):
    source, moved, removed, created = [tmp_path / f"{name}.docx" for name in (
        "source", "moved", "removed", "created",
    )]
    make_source(source)
    original = source.read_bytes()
    native_rows = tmp_path / "markers.json"
    native(source, "inspect", native_rows)
    async with launch() as client:
        document = await opened(client, source)
        full = await inspect(client, document)
        page = await inspect(client, document, limit=2)
        cursor = page["next_cursor"]
        rows = page["range_markers"]
        while page["next_cursor"]:
            page = await inspect(client, document, limit=2, cursor=page["next_cursor"])
            rows.extend(page["range_markers"])
        assert rows == full["range_markers"]
        keys = ["body_index", "inline_index", "kind", "id", "name"]
        assert [list(row[k] for k in keys) for row in rows] == json.loads(native_rows.read_bytes()) == [
            [1, 0, "bookmarkStart", "+007", "OldPlace"],
            [1, 2, "bookmarkEnd", "0007", None],
            [4, 0, "commentRangeStart", "0", None],
            [4, 2, "commentRangeEnd", "0", None],
            [4, 3, "commentReference", "0", None],
        ]
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="body-inlines", cursor=cursor))
        body = (await call(client, "inspect_document", document=document))["body"]
        a = await anchors(client, document)
        foreign = await opened(client, source)
        other = await anchors(client, foreign)
        first, last = a[2, 1], a[3, 0]
        move = edit(7, "OldPlace", (first, last))
        remove = [edit(7, "OldPlace"), edit(0)]
        for error, changes in [
            ("wrong_owner", [edit(7, "OldPlace", (first, other[3, 0]))]),
            ("wrong_anchor_type", [edit(7, "OldPlace", (first, body[3]["anchor"]))]),
            ("invalid_anchor", [edit(7, "OldPlace", (first, f"{document}/unissued"))]),
            ("invalid_input", [dict(edit(7), endpoints=dict(first=first))]),
            ("invalid_input", [dict(edit(7), endpoints=None)]),
            ("rejected_batch", [edit(7, "OldPlace", (last, first))]),
            ("rejected_batch", [edit(7, "DifferentName", (first, last))]),
            ("rejected_batch", [edit(99, endpoints=(first, last))]),
            ("rejected_batch", [edit(7, "OldPlace", (a[7, 1], a[7, 1]))]),
            ("rejected_batch", [move, move]),
            ("rejected_batch", [
                remove[0],
                dict(kind="inline", anchor=a[6, 0], edit=dict(
                    kind="insert-after", inline=dict(
                        kind="hyperlink", target=dict(kind="bookmark", name="OldPlace"),
                        children=[dict(kind="text", text="live reference")],
                    ),
                )),
            ]),
        ]:
            await failure(client, "preview_edits", error, document=document, edits=changes)
        assert await inspect(client, document) == full
        assert not moved.exists()
        draft = await call(client, "preview_edits", document=document, edits=[
            move, edit(0, endpoints=(a[5, 0], a[5, 0])),
            dict(kind="run-text", anchor=body[6]["runs"][0]["anchor"], text="ordinary edited"),
        ])
        alternative = await call(client, "preview_edits", document=document, edits=remove)
        assert draft["preview"] != alternative["preview"]
        await saved(client, document, draft, moved)
        assert await inspect(client, document) == full

        reopened = await opened(client, moved)
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[move])
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="range-markers", cursor=cursor))
        moved_rows = (await inspect(client, reopened))["range_markers"]
        assert [(r["body_index"], r["inline_index"], r["id"]) for r in moved_rows] == [
            (2, 1, "+007"), (3, 1, "0007"), (5, 0, "0"), (5, 2, "0"), (5, 3, "0"),
        ]
        deletion = await call(client, "preview_edits", document=reopened, edits=remove)
        await saved(client, reopened, deletion, removed)
        empty = await opened(client, removed)
        assert (await inspect(client, empty))["range_markers"] == []
        clean = await anchors(client, empty)
        creation = await call(client, "preview_edits", document=empty, edits=[
            edit(8, "NewPlace", (clean[1, 0], clean[1, 0])),
            edit(0, endpoints=(clean[4, 0], clean[4, 0])),
        ])
        await saved(client, empty, creation, created)

    for before, after, action, preview in [
        (source, moved, "move", draft),
        (moved, removed, "remove", deletion),
        (removed, created, "create", creation),
    ]:
        oracle = tmp_path / f"native-{action}.docx"
        assert preview["changes"]["inline_changes"] == native(before, action, oracle)
        assert after.read_bytes() == oracle.read_bytes()
        old, new = parts(before), parts(after)
        assert old.keys() == new.keys()
        assert {key for key in old if old[key] != new[key]} == {"word/document.xml"}
    assert source.read_bytes() == original
    validate(source, moved, removed, created)
    independent = docx.Document(created)
    assert independent.paragraphs[1].hyperlinks[0].url == "https://example.test/range"
    assert independent.paragraphs[5].text == "ordinary edited"
    assert independent.comments.get(0).text == "definition stays"
    bookmark = independent.paragraphs[0]._p.find(qn("w:bookmarkStart"))
    assert (bookmark.get(qn("w:id")), bookmark.get(qn("w:name"))) == ("8", "NewPlace")
    comment = independent.paragraphs[3]._p
    assert [child.tag for child in comment] == [
        qn("w:commentRangeStart"), qn("w:r"), qn("w:commentRangeEnd"), qn("w:r"),
    ]
    assert comment[-1].find(qn("w:commentReference")).get(qn("w:id")) == "0"
