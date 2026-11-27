"""Installed MCP lookup of original native inline nodes through read-only handles."""

import docx
from docx.enum.text import WD_UNDERLINE
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import RGBColor
from jsonschema.validators import validator_for
import pytest

from test_block_readback import assert_inline_node, native_readback
from test_fields import opened, word
from test_stdio import PNG, anyio_backend, call, failure, launch, make_entry_source

pytestmark = pytest.mark.anyio


def add_link(paragraph, label):
    link = word("hyperlink", tooltip=label, history="0")
    link.set(qn("r:id"), paragraph.part.relate_to(
        f"https://example.test/{label}", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    child = paragraph.add_run(label)
    child.bold = False
    link.append(child._r)
    paragraph._p.append(link)
    return child


def add_revision(paragraph, tag, ident, text, **metadata):
    wrapper = word(tag, id=ident, **metadata)
    run = paragraph.add_run(text)
    if tag in {"del", "moveFrom"}:
        run._r[0].tag = qn("w:delText")
    wrapper.append(run._r)
    paragraph._p.append(wrapper)


@pytest.fixture
def source(tmp_path):
    path = tmp_path / "inline-at.docx"
    make_entry_source(path)
    document = docx.Document(path)
    paragraph = document.add_paragraph("Body root")
    rich = add_link(paragraph, "Body link")
    rich.italic = True
    rich.underline = WD_UNDERLINE.DOUBLE
    rich.font.strike = True
    rich.font.hidden = False
    rich.font.color.rgb = RGBColor(17, 34, 51)
    props = rich._r.get_or_add_rPr()
    props.append(word("lang", val="en-GB", eastAsia="ja-JP"))
    props.append(word("outline", val="1"))  # Unsupported direct formatting.
    formatted = paragraph.add_run("Property revision child")
    formatted.bold = True
    rich._r.addnext(formatted._r)
    change = word("rPrChange", id="0040", author="Previous author")
    previous = word("rPr")
    previous.append(word("b"))
    change.append(previous)
    formatted._r.get_or_add_rPr().append(change)
    add_revision(paragraph, "ins", "31", "Inserted child",
                 author="Native author", date="2025-03-01T00:00:00Z")
    add_revision(paragraph, "del", "0032", "Deleted child")
    add_revision(paragraph, "moveFrom", "33", "Move source", author="Mover")
    add_revision(paragraph, "moveTo", "34", "Move destination", author="Mover")

    math = OxmlElement("m:oMath")
    fraction = OxmlElement("m:f")
    math.append(fraction)
    for tag, text in (("num", "3"), ("den", "7")):
        operand, run, item = (OxmlElement(f"m:{name}") for name in (tag, "r", "t"))
        item.text = text
        run.append(item)
        operand.append(run)
        fraction.append(operand)
    paragraph._p.append(math)
    paragraph._p.append(word("fldSimple", instr=" PAGE "))
    picture = paragraph.add_run().add_picture(str(PNG))
    picture._inline.docPr.set("descr", "Readback image")
    picture._inline.docPr.set("title", "Drawing title")

    comment_paragraph = document.comments.get(0).paragraphs[0]
    add_revision(comment_paragraph, "ins", "0061", "Comment insertion", author="Comment reviewer")

    section = document.sections[0]
    add_link(section.header.paragraphs[0], "Header link")
    add_link(section.footer.paragraphs[0], "Footer link")
    add_link(document.add_table(1, 1).cell(0, 0).paragraphs[0], "Cell link")
    document.add_section()
    document.save(path)
    return path


async def inspect(client, document, kind, **selector):
    return await call(client, "inspect_document", document=document, view=dict(kind=kind, **selector))


async def test_inline_at_schema(launch):
    async with launch() as client:
        schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                      if tool.name == "inspect_document")
    valid = validator_for(schema)(schema).is_valid
    lookup = dict(kind="inline-at", inline_handle="issued")
    assert valid(dict(document="doc", view=lookup))
    for bad in (dict(kind="inline-at"), dict(lookup, inline_handle=None),
                dict(lookup, inline_handle=1), dict(lookup, anchor="edit"),
                dict(lookup, limit=1), dict(lookup, cursor="page")):
        assert not valid(dict(document="doc", view=bad))


async def test_inline_at_across_sources_and_nested_nodes(launch, source):
    native = native_readback(source, "inline")
    original = source.read_bytes()
    async with launch() as client:
        document = await opened(client, source)

        async def read(row, expected=None):
            result = await inspect(client, document, "inline-at", inline_handle=row["inline_handle"])
            assert result["document"] == document
            inline = result["inline"]
            assert "anchor" not in inline
            # An entry listing's top-level "id" names the owning entry, not
            # the revision. Lookup keys the row by its handle and returns the
            # original revision ID instead.
            entry_revision = "entry_index" in row and row["kind"] == "revision"
            assert all(row["revision_id" if entry_revision and key == "id" else key] == value
                       for key, value in inline.items())
            assert inline["inline_handle"] == row["inline_handle"]
            if expected is not None:
                assert_inline_node(inline, expected)
            return inline

        async def compare(row, expected, **coordinates):
            metadata = {"anchor", "owner", "note_type", *coordinates}
            payload = {key: value for key, value in row.items() if key not in metadata}
            if "entry_index" in coordinates:
                payload.pop("id")
                # An entry row may attach edit targets to hyperlink children.
                # A null slot on a non-container is not native inline content.
                if "children" not in expected and "children" in payload:
                    assert payload.pop("children") is None
                assert row["id"] == coordinates.pop("entry_id")
                if expected["kind"] == "revision":
                    payload["id"] = payload.pop("revision_id")
            assert {key: row[key] for key in coordinates} == coordinates
            assert_inline_node(payload, expected)
            if expected["kind"] != "opaque":
                inline = await read(row, expected)
                for child, child_expected in zip(inline.get("children", []),
                                                 expected.get("children", []), strict=True):
                    await compare(child, child_expected)

        first = await inspect(client, document, "body-inlines", limit=1)
        remaining = await inspect(client, document, "body-inlines", limit=100, cursor=first["next_cursor"])
        whole = await inspect(client, document, "body-inlines", limit=100)
        assert first["body_inlines"] + remaining["body_inlines"] == whole["body_inlines"]
        root = next(row for row in whole["body_inlines"] if row.get("text") == "Body root")
        run, link, revision, deleted, move_from, move_to, fraction, opaque, picture = [
            row for row in whole["body_inlines"] if row["body_index"] == root["body_index"]
        ]
        assert {row["body_index"] for row in whole["body_inlines"]} == {int(i) for i in native["body"]}
        for index, nodes in native["body"].items():
            rows = [row for row in whole["body_inlines"] if row["body_index"] == int(index)]
            for inline_index, (row, expected) in enumerate(zip(rows, nodes, strict=True)):
                await compare(row, expected, body_index=int(index), inline_index=inline_index)
        assert run["anchor"] != run["inline_handle"]
        assert (await read(run))["items"] == [dict(kind="text", text="Body root")]
        assert (await read(link))["tooltip"] == "Body link"
        child = await read(link["children"][0])
        assert child["text"] == "Body link" and child["format"]["bold"] is False
        assert (child["italic"], child["strike"], child["hidden"]) == (True, True, False)
        assert child["has_opaque_formatting"]
        property_run = await read(link["children"][1])
        assert property_run["has_opaque_formatting"] and property_run["bold"] is None
        assert property_run["property_revision"] == dict(
            kind="runProperties", id="0040", author="Previous author", date=None,
        )
        assert child["format"]["underline"] == "double"
        assert child["format"]["language"] == dict(latin="en-GB", east_asia="ja-JP", bidi=None)
        assert link["bookmark"] is None and link["history_xml"] == "0"
        tracked = await read(revision)
        assert (tracked["revision_kind"], tracked["id"], tracked["author"], tracked["date"]) == (
            "insertion", "31", "Native author", "2025-03-01T00:00:00Z",
        )
        assert (await read(tracked["children"][0]))["text"] == "Inserted child"
        assert [r["revision_kind"] for r in (deleted, move_from, move_to)] == [
            "deletion", "moveSource", "moveDestination",
        ]
        assert deleted["id"] == "0032" and deleted["author"] is None and deleted["date"] is None
        math = await read(fraction)
        assert (math["kind"], math["numerator"], math["denominator"]) == ("fraction", "3", "7")
        assert opaque["kind"] == "opaque" and "inline_handle" not in opaque
        assert picture["drawing"]["description"] == "Readback image"
        assert "drawing" not in run and run["property_revision"] is None

        entries = (await inspect(client, document, "entry-inlines", limit=100))["entry_inlines"]
        for row, expected in zip(entries, native["entries"], strict=True):
            await compare(row, expected["node"], **{key: value for key, value in expected.items() if key != "node"})
        footnote = [row for row in entries if row["entry_kind"] == "footnote" and row["id"] == "007"]
        mark, text = footnote
        assert "anchor" not in mark and "anchor" in text
        assert (await read(mark))["items"] == [dict(kind="opaque", name="w:footnoteRef")]
        assert (await read(text))["text"] == "footnote 007"
        endnote = next(row for row in entries if row.get("text") == "Original endnote tail")
        assert (await read(endnote))["text"] == "Original endnote tail"
        comment = next(row for row in entries if row.get("text") == "First comment")
        assert (await read(comment))["text"] == "First comment"

        headers = (await inspect(client, document, "default-headers"))["default_headers"]
        native_header = headers[0]["blocks"][0]["inlines"][0]
        assert (await read(native_header))["tooltip"] == "Header link"
        stories = (await inspect(client, document, "stories", limit=100))["stories"]
        tables = (await inspect(client, document, "body-tables"))["body_tables"]
        for kind in ("header", "footer"):
            specific = [row for row in stories if row["kind"] == kind and row["variant"] == "default"]
            direct, inherited = specific
            assert direct["owner"] == inherited["owner"]
            for slot in specific:
                (row,) = (await inspect(client, document, "story-inlines",
                                       anchor=slot["owner"]["anchor"], block_index=0))["inlines"]
                assert "anchor" not in row
                await compare(row, native[kind][0], inline_index=0)
                resolved = await read(row)
                assert resolved["tooltip"] == f"{kind.title()} link"
                assert (await read(resolved["children"][0]))["text"] == resolved["tooltip"]
                if kind == "header":
                    assert row["inline_handle"] == native_header["inline_handle"]
                if slot is direct:
                    physical = row["inline_handle"]
                else:
                    assert row["inline_handle"] == physical
        (cell,) = (await inspect(client, document, "table-cell-inlines", anchor=tables[0]["anchor"],
                                 row_index=0, cell_index=0, block_index=0))["inlines"]
        await compare(cell, native["cell"][0], inline_index=0)
        assert (await read(cell))["tooltip"] == "Cell link"
        assert (await read(cell["children"][0]))["text"] == "Cell link"

        handle = run["inline_handle"]
        for invalid in ("unknown", run["anchor"]):
            await failure(client, "inspect_document", "invalid_inline_handle", document=document,
                          view=dict(kind="inline-at", inline_handle=invalid))
        edit = dict(kind="inline", anchor=handle, edit=dict(kind="format", action=dict(kind="set-bold", value=True)))
        await failure(client, "preview_edits", "invalid_anchor", document=document, edits=[edit])
        preview = await call(client, "preview_edits", document=document,
                             edits=[dict(edit, anchor=run["anchor"])])
        assert preview["preview"]
        assert (await read(run))["format"]["bold"] is None

        reopened = await opened(client, source)
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="inline-at", inline_handle=handle))
        rewrapped = f"{reopened}/{handle.split('/', 1)[1]}"
        await failure(client, "inspect_document", "invalid_inline_handle", document=reopened,
                      view=dict(kind="inline-at", inline_handle=rewrapped))
        await call(client, "close_document", document=document)
        await failure(client, "inspect_document", "invalid_document", document=document,
                      view=dict(kind="inline-at", inline_handle=handle))
    assert source.read_bytes() == original
