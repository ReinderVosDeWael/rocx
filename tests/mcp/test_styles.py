"""Installed style catalog readback and native style transactions."""

import subprocess
from zipfile import ZipFile

import docx
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_fields import opened
from test_ranges import inspect, saved
from test_stdio import anyio_backend, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio
STYLE_PATH = "resources/selected-styles.xml"
DOCUMENT_RELS = "word/_rels/document.xml.rels"
REGION = dict(text=dict(bold=False, italic=True), alignment="center", background=[1, 2, 3])


def change(action, **fields):
    return dict(kind="style", edit=dict(kind=action, **fields))


def add(kind, name, **fields):
    return change("add", definition=dict(kind=kind, name=name, **fields))


def write(path, data):
    with ZipFile(path, "w") as archive:
        for name, content in data.items():
            archive.writestr(name, content)


def style_source(path):
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    memo = doc.styles.add_style("Memo Style", WD_STYLE_TYPE.PARAGRAPH)
    memo.font.bold = True
    doc.styles.add_style("Destination", WD_STYLE_TYPE.PARAGRAPH)
    doc.styles.add_style("Swap Style", WD_STYLE_TYPE.CHARACTER)
    doc.styles.add_style("Matrix", WD_STYLE_TYPE.TABLE)
    doc.styles.add_style("Spare", WD_STYLE_TYPE.TABLE)
    doc.add_paragraph("Body", memo)
    doc.sections[0].header.paragraphs[0].text = "Header stays"
    doc.save(path)
    entries = parts(path)
    entries[STYLE_PATH] = entries.pop("word/styles.xml")
    entries[DOCUMENT_RELS] = entries[DOCUMENT_RELS].replace(
        b'Target="styles.xml"', b'Target="../resources/selected-styles.xml"',
    )
    entries["[Content_Types].xml"] = entries["[Content_Types].xml"].replace(
        b"/word/styles.xml", b"/resources/selected-styles.xml",
    )
    write(path, entries)


async def catalog(client, document):
    result = await inspect(client, document, "styles", limit=100)
    rows = result["styles"]
    part = result["part_name"]
    while result["next_cursor"] is not None:
        result = await inspect(client, document, "styles", limit=100, cursor=result["next_cursor"])
        assert result["part_name"] == part
        rows.extend(result["styles"])
    return part, rows


async def preview(client, document, source, action, edits):
    result = await call(client, "preview_edits", document=document, edits=edits)
    output = source.parent / f"{action}.docx"
    expected = source.parent / f"native-{action}.docx"
    await saved(client, document, result, output)
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_style_oracle")
    subprocess.run([oracle, source, action, expected], check=True, timeout=20)
    assert output.read_bytes() == expected.read_bytes()
    return output


async def test_style_discovery(launch):
    async with launch() as client:
        schemas = {t.name: t.inputSchema for t in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    view = validator_for(schema)(schema)
    assert view.is_valid(dict(document="doc", view=dict(kind="styles", limit=100)))
    assert not view.is_valid(dict(document="doc", view=dict(kind="styles", limit=101)))
    schema = schemas["preview_edits"]
    valid = validator_for(schema)(schema).is_valid
    for item in [
        add("numbering", "Named List", numbering_id=1),
        add("character", "Marked", format=dict(italic=False)),
        add("table", "Topics", regions=[dict(region="firstRow", format=REGION)]),
        add("paragraph", "Topic", format=dict(line_height=dict(kind="exact", value=14))),
        change("field", anchor="a", field="rFonts.ascii", value=None),
        change("region", anchor="a", region="firstRow", value=None),
        change("retarget", anchor="a", replacement="b"),
    ]:
        assert valid(dict(document="doc", edits=[item]))
    for item in [
        add("table", "Topics", regions=[dict(region="bad", format=REGION)]),
        add("paragraph", "Bad", format=dict(heading=3)),
        change("field", anchor="a", field="jc"),
        change("region", anchor="a", region="firstRow"),
        change("retarget", anchor="a", replacement_id="b"),
    ]:
        assert not valid(dict(document="doc", edits=[item]))


async def test_styles_native_mixed_and_retarget(launch, tmp_path):
    source = tmp_path / "source.docx"
    style_source(source)
    original = source.read_bytes()
    async with launch() as client:
        document = await opened(client, source)
        head = await inspect(client, document, "styles", limit=1)
        assert len(head["styles"]) == 1 and head["part_name"] == STYLE_PATH
        cursor = head["next_cursor"]
        assert cursor is not None
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="controls", cursor=cursor))
        part, rows = await catalog(client, document)
        assert part == STYLE_PATH and rows[0] == head["styles"][0]
        styles = {r["id"]: r for r in rows}
        assert styles["MemoStyle"]["kind"] == "paragraph"
        assert styles["Heading1"]["linked_style"] == "Heading1Char"
        assert any(etree.fromstring(xml).find(qn("w:b")) is not None
                   for xml in styles["MemoStyle"]["property_xml"])
        target = {key: value["anchor"] for key, value in styles.items() if "anchor" in value}
        body = (await call(client, "inspect_document", document=document))["body"]
        other = await opened(client, source)
        _, foreign = await catalog(client, other)
        other_style = next(r["anchor"] for r in foreign if r["id"] == "MemoStyle")
        for error, edits in [
            ("wrong_owner", [change("retarget", anchor=target["MemoStyle"], replacement=other_style)]),
            ("wrong_anchor_type", [change("remove", anchor=body[0]["anchor"])]),
            ("invalid_anchor", [change("remove", anchor=f"{document}/style:NotIssued")]),
            ("rejected_batch", [add("paragraph", "Will Fail"), change("remove", anchor=target["MemoStyle"])]),
            ("rejected_batch", [change("retarget", anchor=target["MemoStyle"], replacement=target["SwapStyle"])]),
            ("rejected_batch", [add("table", "Duplicate Regions", regions=[
                dict(region="firstRow", format={}), dict(region="firstRow", format={}),
            ])]),
            ("rejected_batch", [change("field", anchor=target["Destination"], field="jc", value="unknown")]),
        ]:
            await failure(client, "preview_edits", error, document=document, edits=edits)
        mixed = await preview(client, document, source, "mixed", [
            add("paragraph", "Staged Paragraph", next_paragraph_style="Destination",
                format=dict(keep_with_next=True, line_height=dict(kind="exact", value=14)),
                text=dict(bold=False, color=[9, 8, 7])),
            add("table", "Staged Table", regions=[dict(region="band2Vert", format=REGION)]),
            add("numbering", "Staged List", numbering_id=1),
            change("replace", anchor=target["SwapStyle"],
                   definition=dict(kind="character", name="Swap Style", format=dict(bold=True, font="Arial"))),
            change("patch", anchor=target["MemoStyle"], bold=False, italic=True),
            change("field", anchor=target["Destination"], field="jc", value="center"),
            change("region", anchor=target["Matrix"], region="firstRow", value=REGION),
            change("remove", anchor=target["Spare"]),
            dict(kind="insert-rich-paragraph", anchor=body[0]["anchor"], paragraph=dict(
                inlines=[dict(kind="text", text="Staged body")], format=dict(style="Staged Paragraph"),
            )),
        ])
        doc = docx.Document(mixed)
        assert doc.paragraphs[1].style.name == "Staged Paragraph"
        assert doc.styles["Staged Paragraph"].next_paragraph_style.name == "Destination"
        assert doc.styles["Memo Style"].font.bold is False and doc.styles["Memo Style"].font.italic is True
        assert doc.styles["Swap Style"].font.name == "Arial" and doc.styles["Swap Style"].font.bold
        assert "Spare" not in doc.styles
        assert doc.styles["Matrix"].element.xpath(".//w:shd/@w:fill") == ["010203"]
        assert doc.styles["Staged Table"].element.xpath("w:tblStylePr/@w:type") == ["band2Vert"]
        reopened = await opened(client, mixed)
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="styles", cursor=cursor))
        await failure(client, "preview_edits", "wrong_owner", document=reopened,
                      edits=[change("remove", anchor=target["Matrix"])])
        _, rows = await catalog(client, reopened)
        entries = {row["id"]: row for row in rows}
        assert entries["StagedList"]["kind"] == "numbering"
        cleared = await preview(client, reopened, mixed, "clear", [
            change("region", anchor=entries["Matrix"]["anchor"], region="firstRow", value=None),
            change("field", anchor=entries["Destination"]["anchor"], field="jc", value=None),
        ])
        assert not docx.Document(cleared).styles["Matrix"].element.xpath("w:tblStylePr")
        assert docx.Document(cleared).styles["Destination"].paragraph_format.alignment is None
        reopened = await opened(client, cleared)
        _, rows = await catalog(client, reopened)
        anchors = {row["id"]: row["anchor"] for row in rows}
        migrated = await preview(client, reopened, cleared, "retarget", [
            change("retarget", anchor=anchors["MemoStyle"], replacement=anchors["Destination"]),
        ])
        final = docx.Document(migrated)
        assert final.paragraphs[0].style.name == "Destination"
        assert "Memo Style" not in final.styles

    assert source.read_bytes() == original
    for prior, output, changed in [
        (source, mixed, {STYLE_PATH, "word/document.xml", "word/numbering.xml"}),
        (mixed, cleared, {STYLE_PATH}),
        (cleared, migrated, {STYLE_PATH, "word/document.xml"}),
    ]:
        before, after = parts(prior), parts(output)
        assert before.keys() == after.keys()
        assert {name for name in before if before[name] != after[name]} == changed
    validate(source, mixed, cleared, migrated)


async def test_styles_absence_and_duplicate_ids(launch, tmp_path):
    source = tmp_path / "absent.docx"
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.add_paragraph("Without a styles relationship")
    doc.save(source)
    before = parts(source)
    root = etree.fromstring(before[DOCUMENT_RELS])
    for rel in root:
        if rel.get("Type").endswith("/styles"):
            root.remove(rel)
            break
    before[DOCUMENT_RELS] = etree.tostring(root)
    write(source, before)
    duplicate = tmp_path / "duplicate.docx"
    one = doc.styles.add_style("Clone One", WD_STYLE_TYPE.CHARACTER)
    two = doc.styles.add_style("Clone Two", WD_STYLE_TYPE.CHARACTER)
    two.element.set(qn("w:styleId"), one.style_id)
    doc.save(duplicate)
    async with launch() as client:
        document = await opened(client, source)
        assert await catalog(client, document) == (None, [])
        created = await preview(client, document, source, "create", [add("paragraph", "First Style")])
        reopened = await opened(client, created)
        part, rows = await catalog(client, reopened)
        assert part != "word/styles.xml"
        assert [row["id"] for row in rows] == ["FirstStyle"]
        assert parts(created)["word/styles.xml"] == before["word/styles.xml"]
        duplicate_handle = await opened(client, duplicate)
        _, rows = await catalog(client, duplicate_handle)
        duplicates = [row for row in rows if row["id"] == one.style_id]
        assert len(duplicates) == 2 and all("anchor" not in row for row in duplicates)
        await failure(client, "preview_edits", "invalid_anchor", document=duplicate_handle,
                      edits=[change("remove", anchor=f"{duplicate_handle}/style:{one.style_id}")])
    validate(created)
