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

from test_stdio import anyio_backend, assert_tool_failure, call, digest, executable, failure, launch, parts, validate


pytestmark = pytest.mark.anyio

HEADER = "word/custom/relocated-header.xml"
HEADER_RELS = "word/custom/_rels/relocated-header.xml.rels"
MAIN = "word/document.xml"
MAIN_RELS = "word/_rels/document.xml.rels"

RICH = {
    "format": {"spacing_after": 9.0},
    "inlines": [
        {"kind": "text", "text": "Styled 😀", "format": {"bold": True, "italic": False, "size": 14.0}},
        {"kind": "text", "text": " tail"},
        {"kind": "hyperlink", "target": {"kind": "url", "url": "https://example.org/rich"},
         "children": [{"kind": "text", "text": " source"}]},
    ],
}


def assert_rich(paragraph):
    assert paragraph.text == "Styled 😀 tail source"
    assert paragraph.runs[0].bold is True and paragraph.runs[0].italic is False
    assert paragraph.runs[0].font.size.pt == 14
    assert paragraph.paragraph_format.space_after.pt == 9
    assert paragraph.hyperlinks[0].url == "https://example.org/rich"


@pytest.mark.parametrize("case", [
    "create-rich", "rich-splice-shared", "rich-splice-local", "rich-splice-footer",
])
async def test_rich_story_blocks_native_bytes_and_preservation(launch, story_source, case):
    if case == "rich-splice-local":
        fixture = docx.Document(story_source)
        fixture.sections[1].footer.is_linked_to_previous = True
        fixture.save(story_source)
    source = story_source.read_bytes()
    blocks = [{"kind": "paragraph", "paragraph": RICH}]
    async with launch() as client:
        document = await open_document(client, story_source)
        rows, _ = await story_page(client, document)
        if case == "create-rich":
            operation = {"kind": "edit-story-reference", "anchor": rows[5]["anchor"],
                         "edit": {"kind": "create-rich", "blocks": blocks}}
        elif case == "rich-splice-footer":
            operation = {"kind": "edit-story-blocks", "anchor": rows[9]["anchor"],
                         "edit": {"mode": "shared", "index": 0, "remove": 1, "blocks": blocks}}
        elif case == "rich-splice-local":
            operation = {"kind": "edit-story-blocks", "anchor": rows[6]["anchor"],
                         "edit": {"mode": "section-local", "index": 2, "remove": 0, "blocks": blocks}}
            # The original header's symbol/table markup prevents a local fork.
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          operation=operation)
            operation = {**operation, "anchor": rows[9]["anchor"],
                         "edit": {**operation["edit"], "index": 1}}
        else:
            operation = {
                "kind": "edit-story-blocks", "anchor": rows[6]["anchor"],
                "edit": {"mode": "shared" if case.endswith("shared") else "section-local",
                         "index": 2, "remove": 0, "blocks": blocks},
            }
            await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                          operation={**operation, "anchor": rows[6]["owner"]["anchor"]})
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          operation={**operation, "edit": {**operation["edit"], "index": 0, "remove": 1}})
        output = await save_preview(client, document, story_source, case, operation)

    assert story_source.read_bytes() == source
    document = docx.Document(output)
    if case == "create-rich":
        assert_rich(document.sections[0].even_page_footer.paragraphs[0])
    elif case == "rich-splice-footer":
        assert len(document.sections[1].footer.paragraphs) == 1
        assert_rich(document.sections[1].footer.paragraphs[0])
        assert document.sections[0].footer.paragraphs[0].text == "Prior footer"
    elif case == "rich-splice-local":
        assert_rich(document.sections[1].footer.paragraphs[-1])
        assert [p.text for p in document.sections[0].footer.paragraphs] == ["Prior footer"]
        assert document.sections[1].footer.paragraphs[0].text == "Prior footer"
    else:
        assert_rich(document.sections[1].header.paragraphs[-1])
        assert (document.sections[0].header.paragraphs[-1].text == "Styled 😀 tail source") == case.endswith("shared")
        assert document.sections[1].header.tables[0].cell(0, 0).text == "Opaque table"
    before, after = parts(story_source), parts(output)
    if not case.endswith("shared"):
        assert before[HEADER_RELS] == after[HEADER_RELS]
        assert before[HEADER] == after[HEADER]
    else:
        assert b"https://example.org/header" in after[HEADER_RELS]
        assert b"https://example.org/rich" in after[HEADER_RELS]
    validate(output)


async def test_insert_rich_paragraph_native_bytes(launch, story_source):
    async with launch() as client:
        document = await open_document(client, story_source)
        body = (await call(client, "inspect_document", document=document))["body"]
        edit = {"kind": "insert-rich-paragraph", "anchor": body[0]["anchor"], "paragraph": RICH}
        await failure(client, "preview_edits", "invalid_input", document=document,
                      edits=[{**edit, "paragraph": {**RICH, "extra": True}}])
        preview = await call(client, "preview_edits", document=document, edits=[edit])
        assert preview["changes"]["paragraph_insertions"] == [0]
        output = story_source.with_name("rich-insert.docx")
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)
    assert output.read_bytes() == story_oracle(story_source, "rich-insert")
    assert_rich(docx.Document(output).paragraphs[1])
    before, after = parts(story_source), parts(output)
    assert {k for k, v in before.items() if after[k] != v} == {MAIN, MAIN_RELS}
    assert b"https://example.org/rich" in after[MAIN_RELS]
    validate(output)


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


async def test_shared_first_footer_ignores_duplicate_section_type(launch, tmp_path):
    source = tmp_path / "duplicate-section-type.docx"
    fixture = docx.Document()
    fixture.add_paragraph("Untouched body")
    fixture.sections[0].first_page_footer.paragraphs[0].add_run("First footer").bold = True
    fixture.add_section()
    sect_pr = fixture.sections[0]._sectPr
    for value in ("nextPage", "continuous"):
        section_type = OxmlElement("w:type")
        section_type.set(qn("w:val"), value)
        sect_pr.insert_element_before(section_type, "w:pgSz")
    fixture.save(source)
    original = source.read_bytes()
    before = parts(source)
    section_types = ElementTree.fromstring(before[MAIN]).findall(
        f".//{qn('w:sectPr')}/{qn('w:type')}"
    )
    assert [node.get(qn("w:val")) for node in section_types] == ["nextPage", "continuous"]
    empty_source = tmp_path / "empty-stories-duplicate-type.docx"
    footer_reference = sect_pr.find(qn("w:footerReference"))
    sect_pr.remove(footer_reference)
    fixture.save(empty_source)

    async with launch() as client:
        empty = await open_document(client, empty_source)
        await failure(client, "inspect_document", "inspection_failed", document=empty,
                      view=dict(kind="sections"))
        empty_slots, cursor = await story_page(client, empty, limit=12)
        assert cursor is None and len(empty_slots) == 12
        assert [row["section_index"] for row in empty_slots] == [0] * 6 + [1] * 6
        assert all(row["owner"] is None and row["blocks"] == [] for row in empty_slots)
        document = await open_document(client, source)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="sections"))
        first, cursor = await story_page(client, document, limit=6)
        second, cursor = await story_page(client, document, limit=6, cursor=cursor)
        assert len(first) == len(second) == 6 and cursor is None
        explicit, inherited = first[4], second[4]
        assert [(row["section_index"], row["kind"], row["variant"]) for row in (explicit, inherited)] == [
            (0, "footer", "first"), (1, "footer", "first"),
        ]
        assert not explicit["inherited"] and inherited["inherited"]
        assert explicit["owner"] == inherited["owner"]
        assert second[5]["owner"] is None and second[5]["blocks"] == []
        owner = inherited["owner"]
        inlines = await call(client, "inspect_document", document=document, view=dict(
            kind="story-inlines", anchor=owner["anchor"], block_index=0,
        ))
        (run,) = inlines["inlines"]
        assert run["items"] == [dict(kind="text", text="First footer")] and run["bold"] is True

        text = dict(kind="edit-story-text", anchor=owner["anchor"], edits=[
            dict(anchor=inherited["blocks"][0]["inlines"][0]["anchor"], text=" Direct 😀 "),
        ])
        block_edit = dict(mode="shared", index=1, remove=0, blocks=[
            dict(kind="paragraph", paragraph=dict(inlines=[dict(kind="text", text="New footer 😀")])),
        ])
        blocks = dict(kind="edit-story-blocks", anchor=inherited["anchor"], edit=block_edit)
        local = dict(blocks, edit=dict(block_edit, mode="section-local"))
        rejected = await client.call_tool("preview_edits", dict(document=document, operation=local))
        assert_tool_failure(rejected, "rejected_batch")
        assert rejected.structuredContent["detail"] == "ambiguous section structure"

        text_output = await save_preview(client, document, source, "first-footer-text", text)
        blocks_output = await save_preview(client, document, source, "first-footer-blocks", blocks)

    assert source.read_bytes() == original
    for output in (text_output, blocks_output):
        after = parts(output)
        assert after.keys() == before.keys()
        assert {name for name in before if before[name] != after[name]} == {owner["part_name"]}
        assert after[MAIN] == before[MAIN]
    footer = docx.Document(text_output).sections[1].first_page_footer
    assert footer.paragraphs[0].text == " Direct 😀 " and footer.paragraphs[0].runs[0].bold
    footer = docx.Document(blocks_output).sections[1].first_page_footer
    assert [paragraph.text for paragraph in footer.paragraphs] == ["First footer", "New footer 😀"]


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
