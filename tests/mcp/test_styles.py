"""Installed style catalog readback and native style transactions."""

import json
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
from test_stdio import (
    anyio_backend, assert_tool_failure, call, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio
STYLE_PATH = "resources/selected-styles.xml"
DOCUMENT_RELS = "word/_rels/document.xml.rels"
REGION = dict(text=dict(bold=False, italic=True), alignment="center", background=[1, 2, 3])
REGIONS = (
    "wholeTable", "firstRow", "lastRow", "firstCol", "lastCol", "band1Horz", "band2Horz",
    "band1Vert", "band2Vert", "nwCell", "neCell", "swCell", "seCell",
)
TAB = dict(position=36, alignment="decimal", leader="dot")
RICH_REGION = dict(
    background=[1, 2, 3], alignment="center", bidi=False, spacing_before=5.5, spacing_after=2,
    line_height=dict(kind="at-least", value=12.5), tab_stops=[TAB],
    text=dict(
        bold=False, italic=True, underline=True, font="Arial", font_size=11.5, color=[10, 11, 12],
        language=dict(latin="en-GB", east_asia="ja-JP", bidi="ar-SA"),
        bold_complex_script=True, italic_complex_script=False, size_complex_script=13, rtl=False,
        complex_script=True, font_east_asia="MS Gothic", font_complex_script="Tahoma",
    ),
)


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
    stem = action.replace(":", "-")
    output = source.parent / f"{stem}.docx"
    expected = source.parent / f"native-{stem}.docx"
    await saved(client, document, result, output)
    native(source, action, expected)
    assert output.read_bytes() == expected.read_bytes()
    return output


def native(source, action, *args):
    return subprocess.check_output(native_command(source, action, *args), timeout=20)


def native_command(source, action, *args):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_style_oracle")
    return [oracle, source, action, *args]


def only_styles_changed(source, *outputs):
    before = parts(source)
    for output in outputs:
        after = parts(output)
        assert before.keys() == after.keys()
        assert {name for name in before if before[name] != after[name]} == {STYLE_PATH}


def attributes(node, path):
    leaf = node.find("/".join(qn(f"w:{tag}") for tag in path.split("/")))
    assert leaf is not None, path
    return {etree.QName(attr).localname: value for attr, value in leaf.attrib.items()}


def canonical_properties(row):
    # A catalog rewrite can change in-scope namespace declarations; compare
    # the property subtrees themselves, not unused namespace bindings.
    return [etree.tostring(etree.fromstring(xml), method="c14n", exclusive=True)
            for xml in row["property_xml"]]


def assert_region(node):
    assert attributes(node, "pPr/bidi") == {"val": "0"}
    assert attributes(node, "pPr/jc") == {"val": "center"}
    assert attributes(node, "pPr/spacing") == {
        "before": "110", "after": "40", "line": "250", "lineRule": "atLeast",
    }
    assert attributes(node, "pPr/tabs/tab") == {"pos": "720", "val": "decimal", "leader": "dot"}
    assert_rich_text(node)
    assert attributes(node, "tcPr/shd") == {"val": "clear", "color": "auto", "fill": "010203"}


def assert_rich_text(node):
    assert attributes(node, "rPr/rFonts") == {
        "ascii": "Arial", "hAnsi": "Arial", "eastAsia": "MS Gothic", "cs": "Tahoma",
    }
    assert attributes(node, "rPr/lang") == {"val": "en-GB", "eastAsia": "ja-JP", "bidi": "ar-SA"}
    for tag, value in (
        ("b", "0"), ("i", "1"), ("u", "single"), ("color", "0A0B0C"), ("sz", "23"),
        ("bCs", "1"), ("iCs", "0"), ("szCs", "26"), ("rtl", "0"), ("cs", "1"),
    ):
        assert attributes(node, f"rPr/{tag}") == {"val": value}


def paragraph_flags(positive):
    return (
        ("bidi", "bidi", positive),
        ("keep_together", "keepLines", positive),
        ("keep_with_next", "keepNext", not positive),
        ("page_break_before", "pageBreakBefore", positive),
        ("widow_control", "widowControl", not positive),
        ("suppress_line_numbers", "suppressLineNumbers", positive),
        ("contextual_spacing", "contextualSpacing", not positive),
        ("mirror_indents", "mirrorIndents", positive),
        ("snap_to_grid", "snapToGrid", not positive),
    )


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


@pytest.mark.parametrize("region", REGIONS)
async def test_table_style_region_lifecycle(launch, tmp_path, region):
    source = tmp_path / "regions.docx"
    style_source(source)
    async with launch() as client:
        document = await opened(client, source)
        _, rows = await catalog(client, document)
        anchor = next(row["anchor"] for row in rows if row["id"] == "Matrix")
        set_region = await preview(client, document, source, f"region-set:{region}", [
            add("table", "Regional Table", regions=[dict(region=region, format=RICH_REGION)]),
            change("region", anchor=anchor, region=region, value=RICH_REGION),
        ])
        document = await opened(client, set_region)
        _, rows = await catalog(client, document)
        matrix = next(row for row in rows if row["id"] == "Matrix")
        (wrapper,) = map(etree.fromstring, matrix["property_xml"])
        assert wrapper.tag == qn("w:tblStylePr") and wrapper.get(qn("w:type")) == region
        assert_region(wrapper)
        styles = docx.Document(set_region).styles
        assert_region(styles["Matrix"].element.find(qn("w:tblStylePr")))
        added = styles["Regional Table"].element
        if region != "wholeTable":
            assert added.xpath("w:tblStylePr/@w:type") == [region]
            added = added.find(qn("w:tblStylePr"))
        else:
            # Native definitions encode whole-table formatting directly; Region edits
            # address a physically stored wholeTable wrapper instead.
            assert added.find(qn("w:tblStylePr")) is None
        assert_region(added)

        anchor = matrix["anchor"]
        if region == "firstRow":
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=[
                change("field", anchor=anchor, region=region, field="tab.0720.leader", value=None),
            ])
        patched = await preview(client, document, set_region, f"region-fields:{region}", [
            change("field", anchor=anchor, region=region, field=field, value=value)
            for field, value in [
                ("rFonts.cs", "Arial Unicode MS"), ("tab.720.leader", None), ("shd.fill", "A0B1C2"),
            ]
        ])
        wrapper = docx.Document(patched).styles["Matrix"].element.find(qn("w:tblStylePr"))
        assert attributes(wrapper, "rPr/rFonts") == {
            "ascii": "Arial", "hAnsi": "Arial", "eastAsia": "MS Gothic", "cs": "Arial Unicode MS",
        }
        assert attributes(wrapper, "pPr/tabs/tab") == {"pos": "720", "val": "decimal"}
        assert attributes(wrapper, "tcPr/shd") == {"val": "clear", "color": "auto", "fill": "A0B1C2"}

        document = await opened(client, patched)
        _, rows = await catalog(client, document)
        anchor = next(row["anchor"] for row in rows if row["id"] == "Matrix")
        cleared = await preview(client, document, patched, f"region-clear:{region}", [
            change("region", anchor=anchor, region=region, value=None),
        ])
        styles = docx.Document(cleared).styles
        assert styles["Matrix"].element.find(qn("w:tblStylePr")) is None
        assert_region(styles["Regional Table"].element if region == "wholeTable"
                      else styles["Regional Table"].element.find(qn("w:tblStylePr")))
    only_styles_changed(source, set_region, patched, cleared)
    validate(set_region, patched, cleared)


@pytest.mark.parametrize("obstruction", ["duplicate", "row-properties"])
async def test_table_style_region_clear_guard(launch, tmp_path, obstruction):
    source = tmp_path / "guarded.docx"
    style_source(source)
    entries = parts(source)
    root = etree.fromstring(entries[STYLE_PATH])
    table = root.find(f"{qn('w:style')}[@{qn('w:styleId')}='Matrix']")
    region = etree.SubElement(table, qn("w:tblStylePr"), {qn("w:type"): "firstRow"})
    if obstruction == "duplicate":
        etree.SubElement(table, region.tag, dict(region.attrib))
    else:
        etree.SubElement(region, qn("w:trPr"))
    entries[STYLE_PATH] = etree.tostring(root)
    write(source, entries)
    rejected = tmp_path / "rejected.docx"
    result = subprocess.run(native_command(source, "region-clear:firstRow", rejected),
                            capture_output=True, text=True, timeout=20)
    diagnostic = "duplicate or unsupported table style region"
    assert result.returncode != 0 and diagnostic in result.stderr
    assert not rejected.exists()
    async with launch() as client:
        document = await opened(client, source)
        _, rows = await catalog(client, document)
        matrix = next(row for row in rows if row["id"] == "Matrix")
        result = await client.call_tool("preview_edits", dict(document=document, edits=[
            change("region", anchor=matrix["anchor"], region="firstRow", value=None),
        ]))
        assert_tool_failure(result, "rejected_batch")
        assert diagnostic in result.structuredContent["message"]
        output = await preview(client, document, source, "create", [add("paragraph", "First Style")])
        reopened = await opened(client, output)
        _, rows = await catalog(client, reopened)
        preserved = next(row for row in rows if row["id"] == "Matrix")
        assert canonical_properties(preserved) == canonical_properties(matrix)
    only_styles_changed(source, output)


@pytest.mark.parametrize(("mode", "value", "line", "rule", "positive"), [
    ("legacy", 1.5, "360", "auto", True),
    ("multiple", 1.25, "300", "auto", False),
    ("exact", 13.5, "270", "exact", True),
    ("at-least", 12.5, "250", "atLeast", False),
])
async def test_paragraph_style_format(launch, tmp_path, mode, value, line, rule, positive):
    source = tmp_path / "source.docx"
    style_source(source)
    flags = paragraph_flags(positive)
    fmt = dict(
        alignment="center", spacing_before=5.5, spacing_after=2,
        left_indent=0.5, right_indent=0.25, first_line_indent=0.125 if positive else -0.125,
        tab_stops=[TAB], **{field: enabled for field, _, enabled in flags},
    )
    if mode == "legacy":
        fmt["line_spacing"] = value
    else:
        fmt["line_height"] = dict(kind=mode, value=value)
    async with launch() as client:
        document = await opened(client, source)
        output = await preview(client, document, source, f"paragraph-format:{mode}", [
            add("paragraph", "Detailed Paragraph", base_style="Destination", format=fmt),
        ])
        document = await opened(client, output)
        _, rows = await catalog(client, document)
        row = next(row for row in rows if row["id"] == "DetailedParagraph")
        assert row["kind"] == "paragraph" and row["based_on"] == "Destination"
    style = docx.Document(output).styles["Detailed Paragraph"]
    assert style.base_style.name == "Destination"
    for _, tag, enabled in flags:
        assert attributes(style.element, f"pPr/{tag}") == {"val": str(int(enabled))}
    assert attributes(style.element, "pPr/jc") == {"val": "center"}
    assert attributes(style.element, "pPr/spacing") == {
        "before": "110", "after": "40", "line": line, "lineRule": rule,
    }
    assert attributes(style.element, "pPr/ind") == {
        "left": "720", "right": "360", ("firstLine" if positive else "hanging"): "180",
    }
    assert attributes(style.element, "pPr/tabs/tab") == {
        "pos": "720", "val": "decimal", "leader": "dot",
    }
    only_styles_changed(source, output)
    validate(output)


async def test_rich_style_replace_and_native_guards(launch, tmp_path):
    source = tmp_path / "source.docx"
    style_source(source)
    fixture = docx.Document(source)
    fixture.styles["Destination"].font.strike = True
    old_region = etree.SubElement(fixture.styles["Spare"].element, qn("w:tblStylePr"),
                                  {qn("w:type"): "lastRow"})
    etree.SubElement(etree.SubElement(old_region, qn("w:rPr")), qn("w:b"))
    fixture.save(source)
    original = source.read_bytes()

    flags = paragraph_flags(False)
    fmt = dict(
        alignment="center", spacing_before=5.5, spacing_after=2,
        line_height=dict(kind="at-least", value=12.5),
        left_indent=0.5, right_indent=0.25, first_line_indent=-0.125, tab_stops=[TAB],
        **{field: enabled for field, _, enabled in flags},
    )
    paragraph = dict(
        kind="paragraph", name="Destination", base_style="Normal", linked_style="Swap Style",
        next_paragraph_style="Memo Style", format=fmt, text=RICH_REGION["text"],
    )
    table = dict(
        kind="table", name="Spare", base_style="Matrix",
        regions=[dict(region=region, format=RICH_REGION) for region in ("wholeTable", "firstRow")],
    )
    async with launch() as client:
        document = await opened(client, source)
        _, rows = await catalog(client, document)
        anchors = {row["id"]: row["anchor"] for row in rows}
        conflict = paragraph | {"format": fmt | {"line_spacing": 1.5}}
        for action, edit, reason in [
            ("reject-line-spacing-conflict:add",
             change("add", definition=conflict | {"name": "Conflicting Paragraph"}),
             "line_height and line_spacing are mutually exclusive"),
            ("reject-line-spacing-conflict:replace",
             change("replace", anchor=anchors["Destination"], definition=conflict),
             "line_height and line_spacing are mutually exclusive"),
            ("reject-replacement-identity",
             change("replace", anchor=anchors["Destination"],
                    definition=paragraph | {"name": "Renamed Paragraph"}),
             "replacement must preserve style ID, name and type"),
            ("reject-live-replacement",
             change("replace", anchor=anchors["MemoStyle"],
                    definition=paragraph | {"name": "Memo Style", "next_paragraph_style": "Normal"}),
             "resource has live or uninspectable references"),
        ]:
            diagnostic = tmp_path / f"{action.replace(':', '-')}.txt"
            native(source, action, diagnostic)
            result = await client.call_tool("preview_edits", dict(document=document, edits=[edit]))
            assert_tool_failure(result, "rejected_batch")
            expected_error = diagnostic.read_text()
            assert reason in expected_error and expected_error in result.structuredContent["message"]

        output = await preview(client, document, source, "replace-rich", [
            change("replace", anchor=anchors["Destination"], definition=paragraph),
            change("replace", anchor=anchors["Spare"], definition=table),
        ])

    styles = docx.Document(output).styles
    replaced = styles["Destination"]
    assert (replaced.style_id, replaced.type) == ("Destination", WD_STYLE_TYPE.PARAGRAPH)
    assert replaced.base_style.name == "Normal"
    assert replaced.next_paragraph_style.name == "Memo Style"
    assert attributes(replaced.element, "link") == {"val": "SwapStyle"}
    assert replaced.font.strike is None  # Old direct formatting was replaced, not patched.
    assert_rich_text(replaced.element)
    for _, tag, enabled in flags:
        assert attributes(replaced.element, f"pPr/{tag}") == {"val": str(int(enabled))}
    assert attributes(replaced.element, "pPr/jc") == {"val": "center"}
    assert attributes(replaced.element, "pPr/spacing") == {
        "before": "110", "after": "40", "line": "250", "lineRule": "atLeast",
    }
    assert attributes(replaced.element, "pPr/ind") == {"left": "720", "right": "360", "hanging": "180"}
    assert attributes(replaced.element, "pPr/tabs/tab") == {"pos": "720", "val": "decimal", "leader": "dot"}
    replaced = styles["Spare"]
    assert (replaced.style_id, replaced.type) == ("Spare", WD_STYLE_TYPE.TABLE)
    assert replaced.base_style.name == "Matrix"
    assert replaced.element.xpath("w:tblStylePr/@w:type") == ["firstRow"]
    assert_region(replaced.element)
    assert_region(replaced.element.find(qn("w:tblStylePr")))
    assert source.read_bytes() == original
    only_styles_changed(source, output)
    validate(source, output)


async def test_style_direct_fields_set_and_clear(launch, tmp_path):
    source = tmp_path / "source.docx"
    style_source(source)
    # Stored Word XML values, not effective formatting. Put required values last
    # so a clear can remove the leaf only after removing its sibling attributes.
    run_flags = ("b", "i", "bCs", "iCs", "rtl", "cs", "strike", "caps", "smallCaps", "vanish")
    para_flags = ("keepNext", "keepLines", "pageBreakBefore", "widowControl", "bidi",
                  "contextualSpacing", "mirrorIndents", "snapToGrid", "suppressLineNumbers")
    memo = {
        **{tag: {"val": "Destination"} for tag in ("basedOn", "next")},
        "link": {"val": "SwapStyle"},
        "pPr/jc": {"val": "right"},
        "pPr/tabs/tab": {"leader": "middleDot", "val": "end"},
        **{f"rPr/{tag}": {"val": str(i % 2)} for i, tag in enumerate(run_flags)},
        **{f"pPr/{tag}": {"val": str(i % 2)} for i, tag in enumerate(para_flags)},
        "rPr/rFonts": dict(ascii="Arial", hAnsi="Calibri", eastAsia="MS Gothic", cs="Tahoma",
                          asciiTheme="majorAscii", hAnsiTheme="minorHAnsi",
                          eastAsiaTheme="majorEastAsia", cstheme="minorBidi"),
        "rPr/lang": dict(val="en-GB", eastAsia="ja-JP", bidi="ar-SA"),
        "rPr/sz": dict(val="23"),
        "rPr/szCs": dict(val="27"),
        "rPr/vertAlign": dict(val="superscript"),
        "rPr/snapToGrid": dict(val="0"),
        "rPr/color": dict(themeColor="accent2", themeTint="3A", themeShade="C4", val="012345"),
        "rPr/u": dict(color="654321", themeColor="accent5", themeTint="59", themeShade="A6",
                      val="wavyDouble"),
        "pPr/spacing": dict(before="110", after="40", beforeLines="50", afterLines="125",
                           beforeAutospacing="0", afterAutospacing="1", line="270", lineRule="exact"),
        "pPr/ind": dict(left="144", right="288", start="-72", end="36", leftChars="100",
                       rightChars="200", startChars="-50", endChars="25", firstLine="180",
                       firstLineChars="75"),
    }
    expected = {
        "MemoStyle": memo,
        "Destination": {"pPr/ind": dict(hanging="80", hangingChars="50")},
        "Matrix": {"tcPr/shd": dict(fill="102030", color="405060", themeColor="accent1",
                                   themeFill="accent6", themeTint="33", themeShade="CC",
                                   themeFillTint="44", themeFillShade="BB", val="pct25")},
    }
    special = {"pPr/tabs/tab": "tab.-40", "rPr/snapToGrid": "runSnapToGrid"}
    scalar_paths = {"basedOn", "next", "link", "pPr/jc"}
    scalar_paths.update(f"rPr/{tag}" for tag in run_flags)
    scalar_paths.update(f"pPr/{tag}" for tag in para_flags)

    def edits(rows, clear):
        anchors = {row["id"]: row.get("anchor") for row in rows}
        return [
            change("field", anchor=anchors[style_id],
                   field=path.rsplit("/", 1)[-1] if path in scalar_paths
                   else f"{special.get(path, path.rsplit('/', 1)[-1])}.{attr}",
                   value=None if clear else value)
            for style_id, properties in expected.items()
            for path, leaf in properties.items()
            for attr, value in leaf.items()
        ]

    async def reopen_and_check(client, output, *, clear):
        document = await opened(client, output)
        part, rows = await catalog(client, document)
        direct = json.loads(native(output, "inspect"))
        assert part == direct["part_name"] == STYLE_PATH
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in rows] == direct["styles"]
        styles = {row["id"]: row for row in rows}
        for style_id, properties in expected.items():
            row = styles[style_id]
            # Inspect the exact transported property subtrees independently of the
            # native JSON comparison, and verify all three preserved references.
            node = etree.Element(qn("w:style"))
            node.extend(map(etree.fromstring, row["property_xml"]))
            for path, attrs in properties.items():
                if "/" not in path:
                    key = {"basedOn": "based_on", "next": "next_style", "link": "linked_style"}[path]
                    assert row[key] == (None if clear else attrs["val"])
                elif clear:
                    assert node.find("/".join(qn(f"w:{tag}") for tag in path.split("/"))) is None, path
                else:
                    extra = {"pos": "-40"} if path == "pPr/tabs/tab" else {}
                    assert attributes(node, path) == attrs | extra
        return document, rows

    async with launch() as client:
        document = await opened(client, source)
        _, rows = await catalog(client, document)
        set_copy = await preview(client, document, source, "direct-fields-set", edits(rows, False))
        document, rows = await reopen_and_check(client, set_copy, clear=False)
        anchors = {row["id"]: row.get("anchor") for row in rows}
        for action, style_id, field, value, reason in [
            ("reject-style-cycle", "Destination", "basedOn", "MemoStyle", "style inheritance cycle"),
            ("reject-required-style-val", "MemoStyle", "color.val", None, "cannot clear required value"),
        ]:
            diagnostic = tmp_path / f"{action}.txt"
            native(set_copy, action, diagnostic)
            result = await client.call_tool("preview_edits", dict(document=document, edits=[
                change("field", anchor=anchors[style_id], field=field, value=value),
            ]))
            assert_tool_failure(result, "rejected_batch")
            expected_error = diagnostic.read_text()
            assert reason in expected_error and expected_error in result.structuredContent["message"]
        clear_copy = await preview(client, document, set_copy, "direct-fields-clear", edits(rows, True))
        await reopen_and_check(client, clear_copy, clear=True)
    only_styles_changed(source, set_copy, clear_copy)
    validate(set_copy, clear_copy)


async def test_style_definition_properties(launch, tmp_path):
    source = tmp_path / "source.docx"
    style_source(source)
    run = dict(
        bold=True, italic=False, underline="dash", font="Georgia", size=9.5,
        color=dict(theme="accent3", shade=164), superscript=True, strike=False, all_caps=True,
        small_caps=False, snap_to_grid=False, language=RICH_REGION["text"]["language"],
        bold_complex_script=False, italic_complex_script=True, size_complex_script=14,
        rtl=True, complex_script=False, font_east_asia="Yu Gothic", font_complex_script="Tahoma",
    )
    async with launch() as client:
        document = await opened(client, source)
        output = await preview(client, document, source, "style-definition-properties", [
            add("character", "Editorial Character", base_style="Default Paragraph Font",
                linked_style="Memo Style", format=run),
            add("paragraph", "Editorial Paragraph", linked_style="Swap Style", text=RICH_REGION["text"]),
            add("table", "Editorial Table", base_style="Matrix"),
        ])
        reopened = await opened(client, output)
        _, rows = await catalog(client, reopened)

    styles = {row["id"]: row for row in rows}
    character = styles["EditorialCharacter"]
    assert character["based_on"] == "DefaultParagraphFont"
    assert character["linked_style"] == "MemoStyle"
    assert styles["EditorialParagraph"]["linked_style"] == "SwapStyle"
    assert styles["EditorialTable"]["based_on"] == "Matrix"
    (rpr,) = map(etree.fromstring, character["property_xml"])
    assert rpr.tag == qn("w:rPr")
    assert {etree.QName(node).localname for node in rpr} == {
        "rFonts", "lang", "color", "vertAlign", "b", "i", "u", "sz", "strike", "caps",
        "smallCaps", "snapToGrid", "bCs", "iCs", "szCs", "rtl", "cs",
    }
    assert attributes(rpr, "rFonts") == {
        "ascii": "Georgia", "hAnsi": "Georgia", "eastAsia": "Yu Gothic", "cs": "Tahoma",
    }
    assert attributes(rpr, "lang") == {"val": "en-GB", "eastAsia": "ja-JP", "bidi": "ar-SA"}
    assert attributes(rpr, "color") == {"val": "auto", "themeColor": "accent3", "themeShade": "A4"}
    for tag, value in (
        ("vertAlign", "superscript"), ("b", "1"), ("i", "0"), ("u", "dash"), ("sz", "19"),
        ("strike", "0"), ("caps", "1"), ("smallCaps", "0"), ("snapToGrid", "0"), ("bCs", "0"),
        ("iCs", "1"), ("szCs", "28"), ("rtl", "1"), ("cs", "0"),
    ):
        assert attributes(rpr, tag) == {"val": value}
    paragraph = etree.Element(qn("w:style"))
    paragraph.extend(map(etree.fromstring, styles["EditorialParagraph"]["property_xml"]))
    assert_rich_text(paragraph)
    only_styles_changed(source, output)
    validate(output)


@pytest.mark.parametrize(("kind", "expected", "unsupported"), [
    (None, "missing", None),
    ("", "unsupported", ""),
    ("Paragraph", "unsupported", "Paragraph"),
    ("futureStyle", "unsupported", "futureStyle"),
])
async def test_style_catalog_unrecognized_kind(launch, tmp_path, kind, expected, unsupported):
    source = tmp_path / "unrecognized.docx"
    style_source(source)
    entries = parts(source)
    root = etree.fromstring(entries[STYLE_PATH])
    style = root.find(f"{qn('w:style')}[@{qn('w:styleId')}='MemoStyle']")
    if kind is None:
        del style.attrib[qn("w:type")]
    else:
        style.set(qn("w:type"), kind)
    entries[STYLE_PATH] = etree.tostring(root)
    write(source, entries)
    expected_catalog = json.loads(native(source, "inspect"))
    async with launch() as client:
        document = await opened(client, source)
        part, rows = await catalog(client, document)
        assert part == expected_catalog["part_name"] == STYLE_PATH
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in rows] == expected_catalog["styles"]
        row = next(row for row in rows if row["id"] == "MemoStyle")
        assert (row["kind"], row["unsupported_kind"], row["name"]) == (expected, unsupported, "Memo Style")
        assert row["anchor"]  # Native style_anchor is based on unique stored ID, not kind.
        assert etree.fromstring(row["property_xml"][0]).tag == qn("w:rPr")
        await failure(client, "preview_edits", "rejected_batch", document=document, edits=[
            change("patch", anchor=row["anchor"], italic=True),
        ])
        # Rejection does not prevent editing a different modeled style in the same catalog.
        output = await preview(client, document, source, "create", [
            add("paragraph", "First Style"),
        ])
        only_styles_changed(source, output)


@pytest.mark.parametrize("defect", ["root", "content-type", "xml"])
async def test_malformed_style_part(launch, tmp_path, defect):
    source = tmp_path / "malformed.docx"
    style_source(source)
    entries = parts(source)
    if defect == "content-type":
        entries["[Content_Types].xml"] = entries["[Content_Types].xml"].replace(
            b"wordprocessingml.styles+xml", b"wordprocessingml.settings+xml",
        )
    elif defect == "root":
        entries[STYLE_PATH] = entries[STYLE_PATH].replace(b"w:styles", b"w:catalog")
    else:
        entries[STYLE_PATH] = b"<"
    write(source, entries)
    expected_error = json.loads(native(source, "inspect"))["error"]
    async with launch() as client:
        document = await opened(client, source)
        for _ in range(2):
            result = await client.call_tool("inspect_document", dict(
                document=document, view=dict(kind="styles"),
            ))
            assert_tool_failure(result, "inspection_failed")
            assert expected_error in result.structuredContent["message"]
        assert len((await call(client, "inspect_document", document=document))["body"]) == 1
