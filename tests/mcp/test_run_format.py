"""Installed MCP run-format inspection and edits over native physical owners."""

import subprocess
from xml.etree import ElementTree
from zipfile import ZipFile

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import (
    anyio_backend, call, executable, failure, launch, make_entry_source, parts, validate,
)

pytestmark = pytest.mark.anyio
MAIN = "word/document.xml"
CLEAR_FIELDS = [
    "style", "font", "fontEastAsia", "fontComplexScript", "bold", "boldComplexScript",
    "italic", "italicComplexScript", "allCaps", "smallCaps", "strike", "snapToGrid",
    "hidden", "color", "size", "sizeComplexScript", "underline", "verticalAlignment",
    "rtl", "complexScript", "languageLatin", "languageEastAsia", "languageBidi",
]
BODY_FORMAT = {
    "style": "Emphasis", "font": "Finn", "size": 13.5, "size_complex_script": 14.0,
    "italic_complex_script": False, "bold_complex_script": True,
    "font_east_asia": "Ming", "font_complex_script": "Naskh",
    "all_caps": False, "small_caps": True, "strike": False, "snap_to_grid": False,
    "underline": "wave", "color": {"theme": "accent2", "tint": 17},
    "subscript": True, "rtl": False, "complex_script": True,
    "language": {"latin": "en-GB", "east_asia": "ja-JP", "bidi": "ar"},
}


def element(tag, text=None):
    node = OxmlElement(f"w:{tag}")
    if text is not None:
        node.text = text
    return node


def add_link(part, paragraph, text):
    link = element("hyperlink")
    link.set(qn("r:id"), part.relate_to(
        "https://example.org/format", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    run = element("r")
    fmt = element("rPr")
    fmt.extend([element("b"), element("vanish")])
    run.extend([fmt, element("t", text)])
    paragraph.append(link)
    link.append(run)


@pytest.fixture
def format_source(tmp_path):
    source = tmp_path / "source.docx"
    make_entry_source(source)
    doc = docx.Document(source)
    p = doc.add_paragraph()
    target = p.add_run("Body target")
    target.bold = target.italic = True
    target._r.rPr.append(element("outline"))
    add_link(doc.part, p._p, "Body link")
    cell = doc.add_table(1, 1).cell(0, 0).paragraphs[0]
    add_link(doc.part, cell._p, "Cell link")
    for kind in ("header", "footer"):
        story = getattr(doc.sections[0], kind)
        p = story.paragraphs[0]
        add_link(story.part, p._p, f"{kind} link")
    doc.add_section()  # Both occupied slots inherit their physical part.
    comment_p = doc.comments.get(0).paragraphs[0]
    add_link(comment_p.part, comment_p._p, "comment link")
    for kind in ("footnote", "endnote"):
        part = doc.part.part_related_by(getattr(RELATIONSHIP_TYPE, kind.upper() + "S"))
        root = etree.fromstring(part.blob)
        for note in root:
            if note.get(qn("w:type")) is None:
                add_link(part, note[0], f"{kind} link {note.get(qn('w:id'))}")
        part._blob = etree.tostring(root, encoding="UTF-8", xml_declaration=True)
    doc.save(source)
    return source


def operation(owner, *edits):
    return dict(kind="edit-run-format", anchor=owner, edits=list(edits))


def change(anchor, kind, **fields):
    return dict(anchor=anchor, edit=dict(kind=kind, **fields))


async def page(client, document, **view):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="format-runs", **view))


def native_bytes(source, output, kind):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_run_format_oracle")
    subprocess.run([oracle, source, output, kind], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


async def test_run_format_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
        inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
        preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])

        def accepts(edit):
            return preview.is_valid(dict(document="doc", operation=operation("owner", edit)))

        assert inspect.is_valid(dict(document="doc", view=dict(kind="format-runs", limit=1)))
        assert not inspect.is_valid(dict(document="doc", view=dict(kind="format-runs", limit=0)))
        assert accepts(change("run", "set-bold", value=False))
        assert accepts(change("run", "patch-format", clear=CLEAR_FIELDS, set=BODY_FORMAT))
        for toggle in ("bold", "italic", "strike", "hidden"):
            for value in (True, False, None):
                assert accepts(change("run", "set-toggle", toggle=toggle, value=value))
        for invalid in [
            change("run", "set-toggle", toggle="caps", value=False),
            change("run", "set-toggle", toggle="hidden"),
            change("run", "set-bold", value=None),
            change("run", "patch-format", clear=["size_complex_script"]),
            change("run", "patch-format", set={"fontTheme": "minor"}),
            change("run", "patch-format", extra=True),
        ]:
            assert not accepts(invalid), invalid


@pytest.mark.parametrize("kind", ["body", "header", "footer", "footnote", "endnote", "comment"])
async def test_physical_run_format_native_bytes(launch, format_source, kind):
    output = format_source.with_name("copy.docx")
    original = format_source.read_bytes()
    expected = native_bytes(format_source, output.with_name("native.docx"), kind)
    async with launch() as client:
        document = (await call(client, "open_document", path=format_source.name))["document"]
        all_rows = (await page(client, document, limit=100))["format_runs"]
        if kind == "body":
            paged = await page(client, document, limit=2)
            rows = paged["format_runs"]
            await failure(client, "inspect_document", "invalid_cursor", document=document,
                          view={"kind": "entries", "cursor": paged["next_cursor"]})
            while paged["next_cursor"]:
                paged = await page(client, document, limit=3, cursor=paged["next_cursor"])
                rows += paged["format_runs"]
            assert rows == all_rows
            reserved = [row for row in rows if row.get("note_type") == "separator"]
            assert len(reserved) == 2
            assert all(row["read_only"] and "anchor" not in row for row in reserved)

        links = [row for row in all_rows
                 if row["kind"] in ({"body", "body-cell"} if kind == "body" else {kind})
                 and "child_index" in row]
        assert len(links) == (2 if kind in {"body", "footnote"} else 1)
        owner = links[0]["owner"]
        assert all(row["owner"] == owner for row in links)
        if kind == "body":
            cell, = [row for row in links if row["kind"] == "body-cell"]
            assert (cell["body_index"], cell["row_index"], cell["cell_index"]) == (2, 0, 0)
        edits = []
        for row in links:
            assert row["format"]["bold"] is True and row["hidden"] is True
            edits += [
                change(row["anchor"], "set-toggle", toggle="hidden", value=None),
                change(row["anchor"], "patch-format",
                       set={"italic": False, "color": [17, 51, 119]}, clear=["bold"]),
            ]
        if kind == "body":
            body, = [row for row in all_rows if row["text"] == "Body target"]
            assert body["owner"] == owner and body["has_opaque_formatting"]
            assert body["format"]["italic"] is True
            edits += [
                change(body["anchor"], "set-bold", value=False),
                change(body["anchor"], "set-toggle", toggle="hidden", value=True),
                change(body["anchor"], "patch-format", set=BODY_FORMAT, clear=["italic"]),
            ]
        op = operation(owner["anchor"], *edits)
        foreign = next(row for row in all_rows if "child_index" in row and row["owner"] != owner)
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=operation(foreign["owner"]["anchor"], *edits))
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=operation(owner["anchor"], *edits, edits[0]))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      operation=operation(links[0]["anchor"], *edits))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      operation=operation(owner["anchor"], change(
                          owner["anchor"], "set-bold", value=True,
                      )))
        preview = await call(client, "preview_edits", document=document, operation=op)
        assert preview["changes"] is None
        await call(client, "save_copy", document=document, preview=preview["preview"],
                   output=output.name)
        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=op)
        after_rows = (await page(client, reopened, limit=100))["format_runs"]
        for original_link in links:
            row, = [row for row in after_rows if row["text"] == original_link["text"]]
            assert row["format"]["bold"] is None and row["hidden"] is None
            assert row["format"]["italic"] is False and row["format"]["color"] == [17, 51, 119]
        if kind == "body":
            row, = [row for row in after_rows if row["text"] == "Body target"]
            assert row["format"]["italic"] is None and row["hidden"] is True
            assert row["format"]["bold"] is False and row["has_opaque_formatting"]
            for key, value in BODY_FORMAT.items():
                if key == "color":
                    value = dict(value, shade=None)
                assert row["format"][key] == value, key
            cleared = output.with_name("cleared.docx")
            clear_preview = await call(client, "preview_edits", document=reopened,
                                       operation=operation(row["owner"]["anchor"], change(
                                           row["anchor"], "patch-format", clear=CLEAR_FIELDS,
                                       )))
            await call(client, "save_copy", document=reopened, preview=clear_preview["preview"],
                       output=cleared.name)
            assert cleared.read_bytes() == native_bytes(
                output, output.with_name("native-clear.docx"), "body-clear",
            )
            cleared_doc = (await call(client, "open_document", path=cleared.name))["document"]
            restored, = [row for row in (await page(client, cleared_doc, limit=100))["format_runs"]
                         if row["text"] == "Body target"]
            assert restored["hidden"] is None and restored["has_opaque_formatting"]
            assert all(value is None for value in restored["format"].values())

    assert format_source.read_bytes() == original
    assert output.read_bytes() == expected
    before, after = parts(format_source), parts(output)
    part = owner.get("part_name", MAIN)
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {part}
    target = ElementTree.fromstring(after[part])
    for link in target.iter(qn("w:hyperlink")):
        run = link.find(qn("w:r"))
        props = run.find(qn("w:rPr"))
        assert props.find(qn("w:b")) is None and props.find(qn("w:vanish")) is None
        assert props.find(qn("w:color")).get(qn("w:val")) == "113377"
    readback = docx.Document(output)
    if kind in {"header", "footer"}:
        for section in readback.sections:
            link, = getattr(section, kind).paragraphs[0].hyperlinks
            assert link.url == "https://example.org/format"
            assert link.runs[0].italic is False and link.runs[0].bold is None
    elif kind == "body":
        assert readback.paragraphs[1].runs[0].font.size.pt == 13.5
    validate(output)


@pytest.mark.parametrize("bad_part,catalog", [
    ("word/notes/footnotes.xml", "entries"),
    ("word/header1.xml", "stories"),
])
async def test_broken_unrelated_catalog_does_not_block_body(
    launch, format_source, bad_part, catalog,
):
    source = format_source.with_name("broken.docx")
    payloads = parts(format_source)
    assert bad_part in payloads
    payloads[bad_part] = b"<bad/>"
    with ZipFile(source, "w") as archive:
        for name, data in payloads.items():
            archive.writestr(name, data)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        rows = (await page(client, document, limit=100))["format_runs"]
        diagnostic, = [row for row in rows if row["kind"] == "unavailable"]
        assert diagnostic["catalog"] == catalog and diagnostic["error"]
        assert "anchor" not in diagnostic and "owner" not in diagnostic
        assert any(row["kind"] == ("footnote" if catalog == "stories" else "header")
                   for row in rows)
        body, = [row for row in rows if row.get("text") == "Body target"]
        preview = await call(client, "preview_edits", document=document, operation=operation(
            body["owner"]["anchor"], change(body["anchor"], "set-bold", value=False),
        ))
        assert preview["preview"]
