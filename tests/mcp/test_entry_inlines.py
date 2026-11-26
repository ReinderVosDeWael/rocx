"""Installed MCP entry-inline lifecycle and physical relationship ownership."""

import posixpath
import subprocess

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_drawings import IMAGES
from test_fields import opened
from test_ranges import inspect, saved
from test_stdio import anyio_backend, call, executable, failure, launch, make_entry_source, parts, validate

pytestmark = pytest.mark.anyio


def edit(anchor, kind, **fields):
    return dict(anchor=anchor, edit=dict(kind=kind, **fields))


def operation(owner, *edits):
    return dict(kind="edit-entry-inlines", anchor=owner, edits=list(edits))


def image(name="sample.png", **fields):
    return dict(kind="image", bytes=list((IMAGES / name).read_bytes()), **fields)


def rels_path(owner):
    base, name = posixpath.split(owner)
    return posixpath.join(base, "_rels", name + ".rels")


def relation(path, owner, rid):
    data = parts(path)
    rels = etree.fromstring(data[rels_path(owner)])
    item, = [item for item in rels if item.get("Id") == rid]
    target = item.get("Target")
    if item.get("TargetMode") == "External":
        return target
    part = posixpath.normpath(posixpath.join(posixpath.dirname(owner), target)).lstrip("/")
    return data[part]


async def preview(client, document, source, kind, command, op):
    result = await call(client, "preview_edits", document=document, operation=op)
    assert result["changes"] is None
    output = source.parent / f"{command}.docx"
    native = source.parent / f"native-{command}.docx"
    await saved(client, document, result, output)
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_entry_inline_oracle")
    subprocess.run([oracle, source, kind, command, native], check=True, timeout=20)
    assert output.read_bytes() == native.read_bytes()
    return output


async def test_entry_inline_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    view = validator_for(schema)(schema)
    assert view.is_valid(dict(document="doc", view=dict(kind="entry-inlines", limit=1)))
    assert not view.is_valid(dict(document="doc", view=dict(kind="entry-inlines", cursor=None)))
    schema = schemas["preview_edits"]
    candidate = validator_for(schema)(schema)
    for value in [
        operation("owner", edit("a", "insert-after", inline=image())),
        operation("owner", edit("a", "drawing-property", property="crop", inline=image())),
        operation("owner", edit("a", "drawing-description", value=None)),
        operation("owner", edit("a", "format", action=dict(kind="set-bold", value=True))),
    ]:
        assert candidate.is_valid(dict(document="doc", operation=value))
        assert not candidate.is_valid(dict(document="doc", operation=value, edits=[]))
    for value in [
        operation("owner"),
        operation("owner", edit("a", "drawing-description")),
        operation("owner", edit("a", "hyperlink-tooltip", value="unavailable")),
        operation("owner", edit("a", "drawing-property", property="chart-series", inline=image())),
    ]:
        assert not candidate.is_valid(dict(document="doc", operation=value))


@pytest.mark.parametrize("kind", ["footnote", "endnote", "comment"])
async def test_entry_picture_lifecycle_native_bytes(launch, tmp_path, kind):
    source = tmp_path / "source.docx"
    make_entry_source(source)
    doc = docx.Document(source)
    doc.paragraphs[0].add_run().add_picture(str(IMAGES / "sample.png"))
    doc.save(source)
    original = source.read_bytes()
    ident = "0" if kind == "comment" else "007"
    text_index = 1  # The first run is the note/comment reference mark.

    async def rows(client, document):
        return (await inspect(client, document, "entry-inlines", limit=100))["entry_inlines"]

    def selected(all_rows):
        return {r["inline_index"]: r for r in all_rows
                if (r["entry_kind"], r["id"], r["block_index"]) == (kind, ident, 0)}

    outputs = []
    async with launch() as client:
        document = await opened(client, source)
        all_rows = await rows(client, document)
        first = await inspect(client, document, "entry-inlines", limit=1)
        cursor = first["next_cursor"]
        remaining = await inspect(client, document, "entry-inlines", cursor=cursor, limit=100)
        assert first["entry_inlines"] + remaining["entry_inlines"] == all_rows
        assert remaining["next_cursor"] is None
        assert first["entry_inlines"][0]["note_type"] == "separator"
        assert "anchor" not in first["entry_inlines"][0]
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="entry-runs", cursor=cursor))
        entry_rows = selected(all_rows)
        assert "anchor" not in entry_rows[0]  # Note/comment reference mark.
        original_row = entry_rows[text_index]
        owner = original_row["owner"]
        assert owner["part_name"] == ("word/comments.xml" if kind == "comment"
                                      else f"word/notes/{kind}s.xml")
        text = original_row["anchor"]
        valid = edit(text, "insert-after", inline=image())
        foreign_row = next(row for row in all_rows if row["entry_kind"] != kind and "anchor" in row)
        other_handle = await opened(client, source)
        other_text = selected(await rows(client, other_handle))[text_index]["anchor"]
        for error, op in [
            ("invalid_batch", operation(owner["anchor"])),
            ("wrong_anchor_type", operation(text, valid)),
            ("wrong_anchor_type", operation(owner["anchor"], edit(owner["anchor"], "remove-object"))),
            ("invalid_anchor", operation(owner["anchor"], edit(f"{document}/entry-inline:44:0:0", "remove-object"))),
            ("wrong_owner", operation(owner["anchor"], edit(other_text, "remove-object"))),
            ("rejected_batch", operation(owner["anchor"], valid, edit(foreign_row["anchor"], "insert-after", inline=image()))),
            ("rejected_batch", operation(owner["anchor"], valid, valid)),
            ("rejected_batch", operation(owner["anchor"], edit(text, "insert-after",
                       inline=dict(kind="column-chart", width=72, height=72, categories=["a"], values=[1])))),
        ]:
            await failure(client, "preview_edits", error, document=document, operation=op)
        current = await preview(client, document, source, kind, "insert", operation(owner["anchor"], valid))
        outputs.append(current)
        link = await preview(client, document, source, kind, "link", operation(owner["anchor"],
            edit(text, "insert-after", inline=dict(kind="hyperlink",
                target=dict(kind="url", url="https://example.test/entry"),
                children=[dict(kind="text", text="entry link")],
            )),
        ))
        outputs.append(link)
        link_doc = await opened(client, link)
        link_row = selected(await rows(client, link_doc))[text_index + 1]
        assert link_row["kind"] == "hyperlink" and link_row["children"][0]["text"] == "entry link"
        assert relation(link, owner["part_name"], link_row["relationship_id"]) == "https://example.test/entry"

        new_doc = await opened(client, current)
        await failure(client, "preview_edits", "wrong_owner", document=new_doc,
                      operation=operation(owner["anchor"], valid))
        inserted = selected(await rows(client, new_doc))[text_index + 1]
        drawing = inserted["drawing"]
        initial_id = drawing["relationship_id"]
        assert drawing["kind"] == "image"
        assert relation(current, owner["part_name"], initial_id) == (IMAGES / "sample.png").read_bytes()
        for command in ("patch", "redirect", "replace", "description", "remove"):
            selection = selected(await rows(client, new_doc))
            text = selection[text_index]["anchor"]
            picture = selection[text_index + 1]["anchor"]
            owner = selection[text_index + 1]["owner"]
            if command == "patch":
                edits = [
                    edit(text, "format", action=dict(kind="set-bold", value=True)),
                    edit(picture, "drawing-property", property="transform", inline=image(
                        "sample.jpg", transformation=dict(rotation=90, flip_vertical=False),
                    )),
                ]
                await failure(client, "preview_edits", "rejected_batch", document=new_doc,
                              operation=operation(owner["anchor"], edit(picture, "drawing-property",
                                  property="size", inline=dict(kind="text", text="not a picture"))))
            elif command == "redirect":
                edits = [edit(picture, "image-asset", bytes=image("sample.jpg")["bytes"])]
            elif command == "replace":
                edits = [edit(picture, "replace", inline=image(alt_text=dict(description="Replacement")))]
            elif command == "description":
                edits = [edit(picture, "drawing-description", value=None)]
            else:
                edits = [edit(picture, "remove-object")]
            out = await preview(client, new_doc, current, kind, command, operation(owner["anchor"], *edits))
            before, after = parts(current), parts(out)
            if command in ("patch", "description", "remove"):
                assert before.keys() == after.keys()
                assert {key for key in before if before[key] != after[key]} == {owner["part_name"]}
            outputs.append(out)
            current, new_doc = out, await opened(client, out)
            updated = selected(await rows(client, new_doc)).get(text_index + 1)
            if command == "remove":
                assert updated is None
                continue
            current_drawing = updated["drawing"]
            if command in ("patch", "redirect"):
                assert (current_drawing["rotation"], current_drawing["flip_vertical"]) == (5400000, False)
            if command == "patch":
                assert current_drawing["relationship_id"] == initial_id
            if command == "redirect":
                assert current_drawing["relationship_id"] != initial_id
                assert relation(out, owner["part_name"], initial_id) == (IMAGES / "sample.png").read_bytes()
                assert relation(out, owner["part_name"], current_drawing["relationship_id"]) == (
                    IMAGES / "sample.jpg").read_bytes()
            if command in ("replace", "description"):
                assert current_drawing["description"] == ("Replacement" if command == "replace" else None)

    assert source.read_bytes() == original
    before = parts(source)
    for output in outputs:
        after = parts(output)
        assert set(before) <= set(after)
        assert all(after[name] == before[name] for name in before
                   if name not in {owner["part_name"], "[Content_Types].xml"})
    removed_owner = etree.fromstring(parts(current)[owner["part_name"]])
    entry, = [item for item in removed_owner if item.get(qn("w:id")) == ident]
    text_run = entry.find(qn("w:p")).findall(qn("w:r"))[text_index]
    assert text_run.find(f"{qn('w:rPr')}/{qn('w:b')}") is not None
    mark = "annotationRef" if kind == "comment" else kind + "Ref"
    assert entry.find(f"{qn('w:p')}/{qn('w:r')}/{qn('w:' + mark)}") is not None
    validate(source, *outputs)
