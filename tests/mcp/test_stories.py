"""Installed SDK checks for logical story slots and their physical owners."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.section import WD_HEADER_FOOTER
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts


pytestmark = pytest.mark.anyio

HEADER = "word/custom/relocated-header.xml"
HEADER_RELS = "word/custom/_rels/relocated-header.xml.rels"
MAIN = "word/document.xml"
MAIN_RELS = "word/_rels/document.xml.rels"


@pytest.fixture
def story_source(tmp_path):
    path = tmp_path / "stories.docx"
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.add_paragraph("Untouched body")
    first = document.sections[0]
    header = first.header
    paragraph = header.paragraphs[0]
    paragraph.add_run("Original direct").bold = True
    header.part.partname = PackURI(f"/{HEADER}")
    default_ref = first._sectPr.get_headerReference(WD_HEADER_FOOTER.PRIMARY)
    first._sectPr.add_headerReference(WD_HEADER_FOOTER.FIRST_PAGE, default_ref.rId)

    link = OxmlElement("w:hyperlink")
    link.set(qn("r:id"), header.part.relate_to(
        "https://example.org/header", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    run, text = OxmlElement("w:r"), OxmlElement("w:t")
    text.text = "Original link"
    run.append(text)
    link.append(run)
    paragraph._p.append(link)
    symbol = OxmlElement("w:sym")
    symbol.set(qn("w:font"), "Symbol")
    symbol.set(qn("w:char"), "F041")
    paragraph.add_run()._r.append(symbol)
    header.add_table(1, 1, Inches(2)).cell(0, 0).text = "Opaque table"
    first.footer.paragraphs[0].text = "Prior footer"

    second = document.add_section()
    second.footer.is_linked_to_previous = False
    second.footer.paragraphs[0].text = "Explicit footer override"
    document.save(path)
    return path


async def open_document(client, path):
    return (await call(client, "open_document", path=path.name))["document"]


async def story_page(client, document, **view):
    result = await call(client, "inspect_document", document=document, view=dict(kind="stories", **view))
    assert result["document"] == document
    return result["stories"], result["next_cursor"]


def story_oracle(source, case):
    output = source.with_name(f"native-{case}.docx")
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_story_oracle")
    subprocess.run([binary, case, output, source], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


async def save_preview(client, document, source, case, operation):
    preview = await call(client, "preview_edits", document=document, operation=operation)
    assert preview["changes"] is None
    output = source.with_name(f"{case}.docx")
    await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)
    data = output.read_bytes()
    assert digest(data) == preview["output_snapshot"]
    assert data == story_oracle(source, case)
    return output


async def test_story_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
    preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])

    view = dict(kind="stories", limit=1, cursor="next")
    assert inspect.is_valid(dict(document="doc", view=view))
    assert not inspect.is_valid(dict(document="doc", view=dict(view, limit=0)))
    assert not inspect.is_valid(dict(document="doc", view=dict(view, cursor=None)))

    edit = dict(anchor="inline", text="plain")
    text = dict(kind="edit-story-text", anchor="physical", edits=[edit])
    assert preview.is_valid(dict(document="doc", operation=text))
    assert not preview.is_valid(dict(document="doc", operation=dict(text, edits=[dict(edit, run=0)])))
    reference = dict(kind="edit-story-reference", anchor="slot")
    for change in (dict(kind="create", paragraphs=[]), dict(kind="link", anchor="physical"),
                   dict(kind="remove")):
        assert preview.is_valid(dict(document="doc", operation=dict(reference, edit=change)))
    for change in (dict(kind="create", paragraphs=[{"text": "rich"}]), dict(kind="link", path=HEADER),
                   dict(kind="remove", anchor="physical")):
        assert not preview.is_valid(dict(document="doc", operation=dict(reference, edit=change)))


async def test_story_slot_pages_include_missing_and_share_owner_across_aliases(launch, story_source):
    async with launch() as client:
        document = await open_document(client, story_source)
        first, cursor = await story_page(client, document, limit=1)
        next_rows, cursor = await story_page(client, document, limit=6, cursor=cursor)
        final, cursor = await story_page(client, document, limit=6, cursor=cursor)
        rows = first + next_rows + final
        assert cursor is None
        assert [(s["section_index"], s["kind"], s["variant"]) for s in rows] == [
            (section, kind, variant)
            for section in range(2)
            for kind in ("header", "footer")
            for variant in ("default", "first", "even")
        ]
        assert len({row["anchor"] for row in rows}) == 12
        assert rows[0]["owner"]["part_name"] == HEADER
        assert all(rows[i]["owner"] == rows[0]["owner"] for i in (1, 6, 7))
        assert all(not rows[i]["inherited"] for i in (0, 1))
        assert all(rows[i]["inherited"] for i in (6, 7))
        assert all(rows[i]["owner"] is None and rows[i]["blocks"] == [] for i in (2, 4, 5, 8, 10, 11))
        assert rows[3]["owner"] != rows[9]["owner"]
        blocks = rows[0]["blocks"]
        assert blocks == rows[7]["blocks"]
        paragraph, table = blocks
        assert paragraph["text"] == "Original directOriginal link"
        assert paragraph["has_opaque_content"]
        assert table == dict(type="opaque", index=1, name="w:tbl")
        direct, hyperlink, symbol = paragraph["inlines"]
        assert direct["type"] == "run" and direct["text"] == "Original direct"
        assert hyperlink["type"] == "hyperlink"
        assert hyperlink["children"][0]["text"] == "Original link"
        assert "anchor" not in hyperlink["children"][0]
        assert symbol["has_opaque_content"] and "anchor" not in symbol


async def test_story_text_scoped_targets_and_native_bytes(launch, story_source):
    source_data = story_source.read_bytes()
    async with launch() as client:
        document = await open_document(client, story_source)
        all_rows, _ = await story_page(client, document)
        alias = all_rows[7]
        direct, _, _ = alias["blocks"][0]["inlines"]
        edit = dict(anchor=direct["anchor"], text=" Direct 😀 ")
        operation = dict(kind="edit-story-text", anchor=alias["owner"]["anchor"], edits=[edit])
        footer_run = all_rows[3]["blocks"][0]["inlines"][0]
        wrong = dict(operation, edits=[edit, dict(anchor=footer_run["anchor"], text="Rejected")])
        await failure(client, "preview_edits", "rejected_batch", document=document, operation=wrong)
        for wrong in (
            dict(operation, anchor=direct["anchor"]),
            dict(operation, edits=[dict(edit, anchor=alias["owner"]["anchor"])]),
            dict(operation, anchor=alias["anchor"]),
        ):
            await failure(client, "preview_edits", "wrong_anchor_type", document=document, operation=wrong)
        output = await save_preview(client, document, story_source, "text", operation)

    assert story_source.read_bytes() == source_data
    old, new = parts(story_source), parts(output)
    assert new.keys() == old.keys()
    assert {name for name in old if old[name] != new[name]} == {HEADER}
    assert new[HEADER_RELS] == old[HEADER_RELS]
    root = ElementTree.fromstring(new[HEADER])
    assert root.find(f".//{qn('w:sym')}").get(qn("w:char")) == "F041"
    before = ElementTree.fromstring(old[HEADER]).find(f".//{qn('w:hyperlink')}")
    after = root.find(f".//{qn('w:hyperlink')}")
    assert ElementTree.tostring(before) == ElementTree.tostring(after)
    header = docx.Document(output).sections[0].header
    assert header.paragraphs[0].runs[0].bold
    assert header.paragraphs[0].text == " Direct 😀 Original link"
    assert header.tables[0].cell(0, 0).text == "Opaque table"


@pytest.mark.parametrize("case", ["create", "link", "remove"])
async def test_story_reference_lifecycle_and_native_bytes(launch, story_source, case):
    source_data = story_source.read_bytes()
    old = parts(story_source)
    async with launch() as client:
        document = await open_document(client, story_source)
        rows, _ = await story_page(client, document)
        if case == "create":
            slot = rows[5]
            edit = dict(kind="create", paragraphs=["New even footer", "Second paragraph & 😀"])
        elif case == "link":
            slot = rows[8]
            edit = dict(kind="link", anchor=rows[0]["owner"]["anchor"])
            wrong = dict(kind="link", anchor=rows[3]["owner"]["anchor"])
            await failure(client, "preview_edits", "rejected_batch", document=document, operation={
                "kind": "edit-story-reference", "anchor": slot["anchor"], "edit": wrong,
            })
            wrong = dict(kind="link", anchor=f"{document}/{HEADER}")
            await failure(client, "preview_edits", "invalid_anchor", document=document, operation={
                "kind": "edit-story-reference", "anchor": slot["anchor"], "edit": wrong,
            })
        else:
            slot = rows[9]
            edit = dict(kind="remove")
        operation = dict(kind="edit-story-reference", anchor=slot["anchor"], edit=edit)
        output = await save_preview(client, document, story_source, case, operation)
        reopened = await open_document(client, output)
        changed, _ = await story_page(client, reopened)
        if case == "create":
            assert not changed[5]["inherited"] and changed[11]["inherited"]
            assert changed[5]["owner"] == changed[11]["owner"]
            assert [p["text"] for p in changed[11]["blocks"]] == edit["paragraphs"]
        elif case == "link":
            assert not changed[8]["inherited"] and changed[8]["owner"] == changed[0]["owner"]
        else:
            assert changed[9]["inherited"] and changed[9]["owner"] == changed[3]["owner"]
            assert changed[9]["blocks"][0]["text"] == "Prior footer"

    assert story_source.read_bytes() == source_data
    new = parts(output)
    expected = {MAIN, MAIN_RELS, "[Content_Types].xml"} if case == "create" else {MAIN}
    assert old.keys() <= new.keys()
    assert {name for name in old if old[name] != new[name]} == expected
    assert len(new.keys() - old.keys()) == (1 if case == "create" else 0)
