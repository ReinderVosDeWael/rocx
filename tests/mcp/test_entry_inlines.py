"""Installed MCP entry-inline lifecycle and physical relationship ownership."""

import posixpath
import subprocess
from zipfile import ZipFile

import docx
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_drawings import IMAGES
from test_fields import opened
from test_ranges import inspect, saved
from test_stdio import (
    anyio_backend, assert_tool_failure, call, executable, failure, launch, make_entry_source,
    parts, validate,
)

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


def assert_only_owner_changed(source, output, owner):
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {owner}


def native_oracle(source, kind, command, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_entry_inline_oracle")
    subprocess.run([oracle, source, kind, command, output], check=True, timeout=20)


async def preview(client, document, source, kind, command, op):
    result = await call(client, "preview_edits", document=document, operation=op)
    assert result["changes"] is None
    output = source.parent / f"{command}.docx"
    native = source.parent / f"native-{command}.docx"
    await saved(client, document, result, output)
    native_oracle(source, kind, command, native)
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
                children=[dict(kind="text", text="first child"), dict(kind="text", text="entry link")],
            )),
        ))
        outputs.append(link)
        link_doc = await opened(client, link)
        link_rows = await rows(client, link_doc)
        link_entry = selected(link_rows)
        link_row = link_entry[text_index + 1]
        assert link_row["kind"] == "hyperlink"
        first_child, child = link_row["children"]
        assert [c["text"] for c in link_row["children"]] == ["first child", "entry link"]
        assert child["kind"] == "run" and child["bold"] is not True
        assert relation(link, owner["part_name"], link_row["relationship_id"]) == "https://example.test/entry"
        link_owner = link_row["owner"]["anchor"]
        format_child = edit(child["anchor"], "format", action=dict(kind="set-bold", value=True))
        format_direct = edit(link_entry[text_index]["anchor"], "format",
                             action=dict(kind="set-toggle", toggle="italic", value=True))
        foreign_owner = next(r["owner"]["anchor"] for r in link_rows if r["entry_kind"] != kind)
        for error, op in [
            ("rejected_batch", operation(link_owner, edit(child["anchor"], "replace",
                                                         inline=dict(kind="text", text="blocked")))),
            ("rejected_batch", operation(link_owner, edit(link_entry[text_index]["anchor"],
                                                         "insert-after", inline=dict(kind="text", text="blocked")),
                                         format_child)),
            ("rejected_batch", operation(foreign_owner, format_child)),
            ("wrong_owner", operation(link_owner, valid)),
        ]:
            await failure(client, "preview_edits", error, document=link_doc, operation=op)
        formatted = await preview(client, link_doc, link, kind, "format-link",
                                  operation(link_owner, format_direct, format_child))
        outputs.append(formatted)
        assert_only_owner_changed(link, formatted, owner["part_name"])
        formatted_doc = await opened(client, formatted)
        reread = selected(await rows(client, formatted_doc))
        assert reread[text_index]["italic"] is True
        updated_link = reread[text_index + 1]
        assert updated_link["relationship_id"] == link_row["relationship_id"]
        assert updated_link["children"][0]["bold"] == first_child["bold"]
        assert updated_link["children"][1]["bold"] is True
        assert updated_link["children"][1]["text"] == "entry link"
        await failure(client, "preview_edits", "wrong_owner", document=formatted_doc,
                      operation=operation(updated_link["owner"]["anchor"], format_child))

        new_doc = await opened(client, current)
        await failure(client, "preview_edits", "wrong_owner", document=new_doc,
                      operation=operation(owner["anchor"], valid))
        inserted = selected(await rows(client, new_doc))[text_index + 1]
        drawing = inserted["drawing"]
        initial_id = drawing["relationship_id"]
        assert drawing["kind"] == "image"
        assert relation(current, owner["part_name"], initial_id) == (IMAGES / "sample.png").read_bytes()
        donor = image(
            "sample.jpg",
            transformation=dict(width=30, height=20, crop=dict(left=15),
                                anchor=dict(x=10, wrap="square", wrap_side="left", allow_overlap=False)),
            alt_text=dict(title="A title", description="Accessible"),
        )
        groups = {
            "size": dict(width_emu=381000, height_emu=254000),
            "crop": dict(crop=[15000, 0, 0, 0]),
            "accessibility": dict(title="A title", description="Accessible"),
            "placement": dict(placement="floating"),
        }
        for command in ("patch", *groups, "redirect", "replace", "description", "remove"):
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
            elif command in groups:
                edits = [edit(picture, "drawing-property", property=command, inline=donor)]
            elif command == "redirect":
                edits = [edit(picture, "image-asset", bytes=image("sample.jpg")["bytes"])]
            elif command == "replace":
                edits = [edit(picture, "replace", inline=image(alt_text=dict(description="Replacement")))]
            elif command == "description":
                edits = [edit(picture, "drawing-description", value=None)]
            else:
                edits = [edit(picture, "remove-object")]
            out = await preview(client, new_doc, current, kind, command, operation(owner["anchor"], *edits))
            if command in ("patch", *groups, "description", "remove"):
                assert_only_owner_changed(current, out, owner["part_name"])
            outputs.append(out)
            current, new_doc = out, await opened(client, out)
            updated = selected(await rows(client, new_doc)).get(text_index + 1)
            if command == "remove":
                assert updated is None
                continue
            current_drawing = updated["drawing"]
            if command in ("patch", "redirect"):
                assert (current_drawing["rotation"], current_drawing["flip_vertical"]) == (5400000, False)
            if command in ("patch", *groups):
                assert current_drawing["relationship_id"] == initial_id
            if command in groups:
                assert {key: current_drawing[key] for key in groups[command]} == groups[command]
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


def write_parts(path, data):
    with ZipFile(path, "w") as archive:
        for part, payload in data.items():
            archive.writestr(part, payload)


def make_entry_drawing_source(source, owner):
    base = source.with_name("base.docx")
    seed = source.with_name("seed.docx")
    make_entry_source(base)
    base_doc = docx.Document(base)
    base_doc.add_paragraph("shape donor").add_run("chart donor")
    base_doc.save(base)
    native_oracle(base, "footnote", "seed-drawings", seed)

    # Native entry insertion is picture-only. Import actual shape/chart markup
    # by moving it from the body and rebasing the chart relationship to the note.
    document_part = "word/document.xml"
    data = parts(seed)
    body = etree.fromstring(data[document_part])
    notes = etree.fromstring(data[owner])
    note, = [n for n in notes if n.get(qn("w:id")) == "007"]
    paragraph = note.find(qn("w:p"))
    drawings = [r for r in body.findall(f".//{qn('w:r')}") if r.find(qn("w:drawing")) is not None]
    assert len(drawings) == 2
    for run in drawings:
        paragraph.append(run)

    body_rels = etree.fromstring(data[rels_path(document_part)])
    chart, = [r for r in body_rels if r.get("Type").endswith("/chart")]
    target = posixpath.normpath(posixpath.join(posixpath.dirname(document_part), chart.get("Target")))
    chart.set("Target", posixpath.relpath(target, posixpath.dirname(owner)))
    entry_rels = etree.Element(body_rels.tag, nsmap=body_rels.nsmap)
    entry_rels.append(chart)
    for part, root in ((document_part, body), (owner, notes), (rels_path(owner), entry_rels)):
        data[part] = etree.tostring(root)
    write_parts(source, data)


async def test_imported_entry_shape_and_chart_format_only_native_bytes(launch, tmp_path):
    source = tmp_path / "source.docx"
    owner_part = "word/notes/footnotes.xml"
    make_entry_drawing_source(source, owner_part)

    async def entry(client, document):
        page = await inspect(client, document, "entry-inlines", limit=100)
        return [r for r in page["entry_inlines"]
                if (r["entry_kind"], r["id"], r["block_index"]) == ("footnote", "007", 0)]

    async with launch() as client:
        document = await opened(client, source)
        _, text, shape, chart = await entry(client, document)
        assert [r["drawing"]["kind"] for r in (shape, chart)] == ["shape", "chart"]
        owner = text["owner"]["anchor"]
        formats = []
        for row in (shape, chart):
            assert row["bold"] is not True
            fmt = edit(row["anchor"], "format", action=dict(kind="set-bold", value=True))
            formats.append(fmt)
            for edits in [
                [edit(row["anchor"], "remove-object")],
                [fmt, edit(text["anchor"], "insert-after", inline=dict(kind="text", text="blocked"))],
            ]:
                result = await client.call_tool("preview_edits", dict(
                    document=document, operation=operation(owner, *edits),
                ))
                assert_tool_failure(result, "rejected_batch")
                assert result.structuredContent["detail"] == (
                    "only picture occurrences are editable in entry stories"
                )
        output = await preview(client, document, source, "footnote", "format-drawings",
                               operation(owner, *formats))
        reopened = await opened(client, output)
        _, _, updated_shape, updated_chart = await entry(client, reopened)
        for original, updated in ((shape, updated_shape), (chart, updated_chart)):
            assert updated["bold"] is True
            assert updated["drawing"] == original["drawing"]

    assert_only_owner_changed(source, output, owner_part)
    validate(source, output)


async def test_top_level_entry_revision_keeps_lexical_id_or_null(launch, tmp_path):
    source = tmp_path / "source.docx"
    make_entry_source(source)
    data = parts(source)
    owner = "word/notes/footnotes.xml"
    notes = etree.fromstring(data[owner])
    note, = [n for n in notes if n.get(qn("w:id")) == "007"]
    paragraph = note.find(qn("w:p"))
    # A missing required w:id is an inspection fixture, not an authorable revision.
    for ident in ("00042", None):
        revision = OxmlElement("w:ins")
        revision.set(qn("w:author"), "Original")
        if ident is not None:
            revision.set(qn("w:id"), ident)
        run = OxmlElement("w:r")
        text = OxmlElement("w:t")
        text.text = "Tracked"
        run.append(text)
        revision.append(run)
        paragraph.append(revision)
    data[owner] = etree.tostring(notes)
    write_parts(source, data)

    async with launch() as client:
        document = await opened(client, source)
        page = await inspect(client, document, "entry-inlines", limit=100)
        rows = page["entry_inlines"]
        assert all("revision_id" not in row for row in rows if row["kind"] != "revision")
        revisions = [row for row in rows if row["kind"] == "revision"]
        assert [(row["id"], row["revision_id"]) for row in revisions] == [
            ("007", "00042"), ("007", None),
        ]
        for row in revisions:
            native = await inspect(client, document, "inline-at", inline_handle=row["inline_handle"])
            assert native["inline"]["id"] == row["revision_id"]
