"""Direct body paragraph and section layout through the installed stdio binary."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_TAB_ALIGNMENT, WD_TAB_LEADER
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Twips
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio

TAB = dict(position=720, alignment="right", leader="dot")
BORDER = dict(size=8, space=4, color="336699")

PARAGRAPH_SET = dict(
    styleId="Title",
    keepWithNext=False, keepTogether=True, pageBreakBefore=True, widowControl=False,
    suppressLineNumbers=True, paragraphBidi=True, snapToGrid=False, contextualSpacing=True,
    mirrorIndents=True, alignment="both", outlineLevel=4,
    spaceBefore=120, spaceAfter=240, lineHeight=300, lineRule="exact",
    leftIndent=360, rightIndent=-120, firstLineIndent=180,
    tabStops=[dict(position=480, alignment="left"), dict(position=960, alignment="decimal",
                                                         leader="hyphen")],
)
BREAK_SET = dict(heading=3)
INITIAL_SET = dict(sectionType="continuous", titlePage=False)
FINAL_SET = dict(
    pageWidth=16000, pageHeight=10000, orientation="landscape",
    marginTop=-600, marginBottom=-720, marginLeft=1000, marginRight=1100,
    marginHeader=400, marginFooter=500, marginGutter=120,
    columnCount=3, columnSpace=360, columnSeparator=True,
    gridMode="linesAndChars", gridPitch=320, gridCharSpace=-40,
    pageStart=7, pageFormat="lowerRoman",
    lineStart=6, lineCountBy=2, lineDistance=180, lineRestart="newSection",
    verticalAlign="center", bidi=True, rtlGutter=True,
    borderOffset="text", borderDisplay="firstPage", borderZOrder="back",
    borderTop=dict(size=12, color="auto"), borderLeft=BORDER, borderBottom={},
    borderRight=dict(space=6),
)


def make_source(path, *, guarded=False):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.add_table(1, 1).cell(0, 0).text = "inside the table"
    paragraph = document.add_paragraph("body text")
    paragraph.paragraph_format.widow_control = True
    paragraph.paragraph_format.space_after = Twips(180)
    document.sections[0].header.paragraphs[0].text = "unchanged header"
    document.sections[0].different_first_page_header_footer = True
    document.add_section()
    if guarded:
        paragraph.style = "Title"
        paragraph.paragraph_format.keep_with_next = True
        properties = paragraph._p.find(qn("w:pPr"))
        properties.find(qn("w:keepNext")).set(qn("w:val"), "true")
        properties.find(qn("w:spacing")).set(qn("w:beforeLines"), "150")

        section = document.sections[1]._sectPr
        columns = section.find(qn("w:cols"))
        columns.set(qn("w:equalWidth"), "0")
        columns.set(qn("w:num"), "2")
        for width in (3600, 4320):
            columns.append(OxmlElement("w:col", attrs={qn("w:w"): str(width)}))
        borders = OxmlElement("w:pgBorders")
        borders.append(OxmlElement("w:top", attrs={
            qn("w:val"): "single", qn("w:sz"): "8", qn("w:color"): "336699",
            qn("w:themeColor"): "accent1",
        }))
        section.insert_element_before(borders, "w:cols")
    document.save(path)
    return path.read_bytes()


async def page(client, document, **view):
    return await call(client, "inspect_document", document=document, view=dict(kind="layout", **view))


def operation(*edits):
    return dict(kind="edit-layout", edits=list(edits))


def edit(anchor, field, value):
    return dict(anchor=anchor, field=field, value=value)


def native_bytes(source, output, *args):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_layout_oracle")
    subprocess.run([oracle, source, output, *args], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


async def test_layout_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
        inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
        preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])
        assert inspect.is_valid(dict(document="doc", view=dict(kind="layout", limit=1)))
        assert not inspect.is_valid(dict(document="doc", view=dict(kind="layout", limit=0)))
        for scope in ("all", "paragraphs", "sections"):
            assert inspect.is_valid(dict(document="doc", view=dict(kind="layout", scope=scope)))
        for scope in (None, "section", []):
            assert not inspect.is_valid(dict(document="doc", view=dict(kind="layout", scope=scope)))

        def request(*edits):
            return dict(document="doc", operation=operation(*edits))

        assert preview.is_valid(request())
        for field, value in [
            *PARAGRAPH_SET.items(), *BREAK_SET.items(), *INITIAL_SET.items(), *FINAL_SET.items(),
            ("firstLineIndent", -240),
            ("margins", None),
            ("lineNumbering", None),
        ]:
            assert preview.is_valid(request(edit("target", field, value)))
        for invalid in [
            dict(anchor="target", field="alignment"),
            edit("target", "first_line_indent", -240),
            edit("target", "unknown", None),
            edit("target", "marginTop", 2**31),
            edit("target", "marginTop", 1.5),
            edit("target", "tabStops", [dict(alignment="right")]),
            edit("target", "tabStops", [dict(TAB, extra=True)]),
            edit("target", "tabStops", [dict(TAB, position=-(2**31) - 1)]),
            edit("target", "borderBottom", dict(BORDER, type="double")),
            edit("target", "borderBottom", dict(BORDER, size=2**31)),
        ]:
            assert not preview.is_valid(request(invalid)), invalid


async def test_layout_pages_mixed_transaction_and_native_bytes(launch, tmp_path):
    source, output, native = [tmp_path / name for name in ("source.docx", "copy.docx", "native.docx")]
    original = make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        first = await page(client, document, limit=1)
        second = await page(client, document, cursor=first["next_cursor"], limit=2)
        last = await page(client, document, cursor=second["next_cursor"], limit=1)
        assert last["next_cursor"] is None
        inspection = await page(client, document)
        rows = first["layout"] + second["layout"] + last["layout"]
        assert inspection["layout"] == rows
        assert await page(client, document, scope="all", limit=1) == first
        scoped = {scope: await page(client, document, scope=scope, limit=1)
                  for scope in ("paragraphs", "sections")}
        cursors = {"all": first["next_cursor"],
                   **{scope: result["next_cursor"] for scope, result in scoped.items()}}
        assert None not in cursors.values() and len(set(cursors.values())) == 3
        for scope, result in scoped.items():
            for other, cursor in cursors.items():
                if other != scope:
                    await failure(client, "inspect_document", "invalid_cursor", document=document,
                                  view=dict(kind="layout", scope=scope, cursor=cursor))
            final_page = await page(client, document, scope=scope, cursor=cursors[scope], limit=1)
            assert final_page["next_cursor"] is None
            assert result["layout"] + final_page["layout"] == [
                row for row in rows if row["kind"] == scope.removesuffix("s")
            ]
        for scope in scoped:
            await failure(client, "inspect_document", "invalid_cursor", document=document,
                          view=dict(kind="layout", cursor=cursors[scope]))
        assert [(row["kind"], row["index"]) for row in rows] == [
            ("paragraph", 1), ("paragraph", 2), ("section", 0), ("section", 1),
        ]
        first_paragraph, empty_paragraph, first_section, final_section = rows
        assert first_paragraph["layout"] == dict(widowControl=True, spaceAfter=180)
        assert empty_paragraph["layout"] == {}
        assert first_section["layout"]["titlePage"] is True
        assert "borderBottom" not in final_section["layout"]
        p, empty, initial, final = [row["anchor"] for row in rows]
        assert len({p, empty, initial, final}) == 4

        change = operation(
            edit(p, "alignment", "center"),
            edit(p, "firstLineIndent", -240),
            edit(p, "keepWithNext", False),
            edit(p, "widowControl", None),
            edit(p, "tabStops", [TAB]),
            edit(initial, "titlePage", None),
            edit(final, "marginTop", 900),
            edit(final, "borderBottom", BORDER),
        )
        draft = await call(client, "preview_edits", document=document, operation=change)
        assert draft["document"] == document and isinstance(draft["changes"], dict)
        assert list(tmp_path.iterdir()) == [source]
        saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())
        assert await page(client, document) == inspection

        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=change)
        new_p, new_empty, new_initial, new_final = (await page(client, reopened))["layout"]
        assert new_p["layout"] == dict(
            alignment="center", firstLineIndent=-240, keepWithNext=False, spaceAfter=180,
            tabStops=[TAB],
        )
        assert new_empty["layout"] == {}
        assert new_initial["layout"] == {k: v for k, v in first_section["layout"].items()
                                         if k != "titlePage"}
        assert new_final["layout"] == dict(final_section["layout"], marginTop=900, borderBottom=BORDER)

    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, native)
    before, after = parts(source), parts(output)
    assert after.keys() == before.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    body = ElementTree.fromstring(after["word/document.xml"]).find(qn("w:body"))
    borders = body.find(f"./{qn('w:sectPr')}/{qn('w:pgBorders')}/{qn('w:bottom')}")
    assert borders.attrib == {qn("w:val"): "single", qn("w:sz"): "8", qn("w:space"): "4",
                              qn("w:color"): "336699"}

    independent = docx.Document(output)
    assert [p.text for p in independent.paragraphs] == ["body text", ""]
    assert independent.tables[0].cell(0, 0).text == "inside the table"
    assert independent.sections[0].header.paragraphs[0].text == "unchanged header"
    assert not independent.sections[0].different_first_page_header_footer
    assert independent.sections[1].top_margin == Twips(900)
    p = independent.paragraphs[0].paragraph_format
    assert p.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert p.first_line_indent == Twips(-240)
    assert p.keep_with_next is False and p.widow_control is None
    assert p.space_after == Twips(180)
    tab, = p.tab_stops
    assert (tab.position, tab.alignment, tab.leader) == (
        Twips(720), WD_TAB_ALIGNMENT.RIGHT, WD_TAB_LEADER.DOTS,
    )
    validate(source, output)


async def test_layout_rejects_conflicts_and_mismatches_then_allows_retry(launch, tmp_path):
    source = tmp_path / "layout.docx"
    make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        inspection = await page(client, document)
        p, _, section, _ = [row["anchor"] for row in inspection["layout"]]
        body = (await call(client, "inspect_document", document=document))["body"]
        run = body[1]["runs"][0]["anchor"]
        for ops, category in [
            ([edit(p, "alignment", "center"), edit(p, "alignment", None)], "rejected_batch"),
            ([edit(section, "margins", None), edit(section, "marginTop", 900)], "rejected_batch"),
            ([edit(p, "marginTop", 900)], "rejected_batch"),
            ([edit(section, "alignment", "center")], "rejected_batch"),
            ([edit(p, "alignment", True)], "rejected_batch"),
            ([edit(section, "margins", True)], "rejected_batch"),
            ([edit(run, "alignment", "center")], "wrong_anchor_type"),
            ([dict(anchor=p, field="widowControl")], "invalid_input"),
        ]:
            await failure(client, "preview_edits", category, document=document, operation=operation(*ops))
        assert await page(client, document) == inspection
        assert list(tmp_path.iterdir()) == [source]

        empty = await call(client, "preview_edits", document=document, operation=operation())
        noop = await call(client, "preview_edits", document=document,
                          operation=operation(edit(p, "alignment", None)))
        saved = await call(client, "save_copy", document=document, preview=empty["preview"],
                           output="noop.docx")
        output = tmp_path / "noop.docx"
        assert saved["output_snapshot"] == empty["output_snapshot"] == noop["output_snapshot"]
    assert output.read_bytes() == native_bytes(source, tmp_path / "native.docx", "empty")
    assert saved["output_snapshot"] == digest(output.read_bytes())
    assert parts(output) == parts(source)


async def save_layout(client, document, output, edits):
    draft = await call(client, "preview_edits", document=document, operation=operation(*edits))
    saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                       output=output.name)
    assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())


def assert_main_only_changed(source, output):
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    return ElementTree.fromstring(after["word/document.xml"]).find(qn("w:body"))


async def test_layout_all_native_field_groups_set_and_clear(launch, tmp_path):
    source, output, cleared, native_set, native_clear = [
        tmp_path / f"{name}.docx" for name in ("source", "set", "clear", "native-set", "native-clear")
    ]
    original = make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        original_rows = (await page(client, document))["layout"]
        changes = [
            edit(row["anchor"], field, value)
            for row, fields in zip(original_rows, (PARAGRAPH_SET, BREAK_SET, INITIAL_SET, FINAL_SET),
                                   strict=True)
            for field, value in fields.items()
        ]
        await save_layout(client, document, output, changes)
        assert (await page(client, document))["layout"] == original_rows

        reopened = (await call(client, "open_document", path=output.name))["document"]
        rows = (await page(client, reopened))["layout"]
        expected = [
            dict(PARAGRAPH_SET, tabStops=[dict(leader=None, **PARAGRAPH_SET["tabStops"][0]),
                                          PARAGRAPH_SET["tabStops"][1]]),
            dict(styleId="Heading3", heading=3),
            dict(original_rows[2]["layout"], **INITIAL_SET),
            original_rows[3]["layout"] | FINAL_SET | dict(lineNumbering=True),
        ]
        for key in ("borderTop", "borderBottom", "borderRight"):
            expected[3][key] = dict.fromkeys(("size", "space", "color")) | FINAL_SET[key]
        for row, layout in zip(rows, expected, strict=True):
            assert row["layout"] == layout

        await failure(client, "preview_edits", "rejected_batch", document=reopened, operation=operation(
            edit(rows[3]["anchor"], "lineNumbering", None), edit(rows[3]["anchor"], "lineCountBy", 3),
        ))
        clear_requests = [
            ("lineRule", "rightIndent", "tabStops"), ("heading",), ("margins",),
            ("lineNumbering", "borderDisplay", "borderLeft"),
        ]
        await save_layout(client, reopened, cleared, [
            edit(row["anchor"], field, None)
            for row, fields in zip(rows, clear_requests, strict=True)
            for field in fields
        ])
        cleared_fields = [
            clear_requests[0], ("heading", "styleId"),
            ("margins", *(field for field in FINAL_SET if field.startswith("margin"))),
            ("lineNumbering", "lineStart", "lineCountBy", "lineDistance", "lineRestart",
             "borderDisplay", "borderLeft"),
        ]
        reopened_clear = (await call(client, "open_document", path=cleared.name))["document"]
        assert [row["layout"] for row in (await page(client, reopened_clear))["layout"]] == [
            {k: v for k, v in fields.items() if k not in removed}
            for fields, removed in zip(expected, cleared_fields, strict=True)
        ]

    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, native_set, "set")
    assert cleared.read_bytes() == native_bytes(native_set, native_clear, "clear")
    body = assert_main_only_changed(source, output)
    changed = assert_main_only_changed(output, cleared)
    final_props = body.find(qn("w:sectPr"))
    cleared_props = changed.find(qn("w:sectPr"))
    for child, attrs in [
        ("cols", dict(num="3", space="360", sep="1")),
        ("docGrid", dict(type="linesAndChars", linePitch="320", charSpace="-40")),
        ("pgNumType", dict(start="7", fmt="lowerRoman")),
        ("lnNumType", dict(start="6", countBy="2", distance="180", restart="newSection")),
    ]:
        element = final_props.find(qn(f"w:{child}"))
        assert {key: element.get(qn(f"w:{key}")) for key in attrs} == attrs
    assert cleared_props.find(qn("w:lnNumType")) is None
    borders = cleared_props.find(qn("w:pgBorders"))
    assert borders.attrib == {qn("w:offsetFrom"): "text", qn("w:zOrder"): "back"}
    assert {edge.tag: edge.attrib for edge in borders} == {
        edge.tag: edge.attrib for edge in final_props.find(qn("w:pgBorders")) if edge.tag != qn("w:left")
    }
    independent = docx.Document(output)
    assert [p.style.style_id for p in independent.paragraphs] == ["Title", "Heading3"]
    p = independent.paragraphs[0].paragraph_format
    assert (p.space_before, p.space_after, p.line_spacing) == tuple(map(Twips, (120, 240, 300)))
    assert independent.sections[1].page_width == Twips(16000)
    assert independent.sections[1].gutter == Twips(120)
    validate(source, output, cleared)


async def test_layout_guards_and_equivalent_imported_values_preserve_payloads(launch, tmp_path):
    source, output, native = [tmp_path / f"{name}.docx" for name in ("guarded", "noop", "native")]
    original = make_source(source, guarded=True)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        inspection = await page(client, document)
        p, _, _, final = [row["anchor"] for row in inspection["layout"]]
        themed_border = dict(size=8, space=None, color="336699")
        assert inspection["layout"][0]["layout"]["keepWithNext"] is True
        assert inspection["layout"][3]["layout"]["borderTop"] == themed_border

        for edits in [
            [edit(p, "keepWithNext", False), edit(final, "columnCount", 3)],
            [edit(p, "spaceBefore", 120)],  # Imported spacing is stored in lines.
            [edit(final, "borderTop", None)],  # Clearing the edge would discard its theme.
            [edit(final, "borderTop", dict(size=12, color="336699"))],
            [edit(final, "marginLeft", None)],  # The margin record requires all seven attributes.
            [edit(final, "borderRight", dict(size=1))],
            [edit(p, "tabStops", [])],
            [edit(p, "tabStops", [TAB, TAB])],
            [edit(p, "styleId", "Emphasis")],  # Character style, not a paragraph style.
            [edit(p, "styleId", "Title"), edit(p, "heading", 3)],
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          operation=operation(*edits))

        assert list(tmp_path.iterdir()) == [source]
        assert await page(client, document) == inspection
        await save_layout(client, document, output, [
            edit(p, "keepWithNext", True),
            edit(p, "heading", None),  # Title is not a heading; keep its style ID.
            edit(final, "borderTop", themed_border),
        ])

    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, native, "guarded-noop")
    assert parts(output) == parts(source)
    validate(source, output)


async def test_layout_paragraph_scope_survives_malformed_terminal_section(launch, tmp_path):
    source, output, native = [tmp_path / f"{name}.docx"
                              for name in ("malformed-section", "outline", "native-outline")]
    make_source(source)
    fixture = docx.Document(source)
    terminal = fixture.sections[-1]._sectPr
    for section_type in terminal.findall(qn("w:type")):
        terminal.remove(section_type)
    for section_type in ("continuous", "nextPage"):
        terminal.insert_element_before(
            OxmlElement("w:type", attrs={qn("w:val"): section_type}), "w:pgSz",
        )
    fixture.save(source)
    original = source.read_bytes()

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        for selector in ({}, {"scope": "all"}, {"scope": "sections"}):
            await failure(client, "inspect_document", "inspection_failed", document=document,
                          view=dict(kind="layout", **selector))
        paragraphs = await page(client, document, scope="paragraphs")
        assert paragraphs["next_cursor"] is None
        p, breaker = paragraphs["layout"]
        assert [(row["kind"], row["index"]) for row in (p, breaker)] == [
            ("paragraph", 1), ("paragraph", 2),
        ]
        assert "outlineLevel" not in p["layout"]
        change = edit(p["anchor"], "outlineLevel", 4)
        await save_layout(client, document, output, [change])
        assert await page(client, document, scope="paragraphs") == paragraphs
        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened,
                      operation=operation(change))
        after = (await page(client, reopened, scope="paragraphs"))["layout"]
        assert [row["layout"] for row in after] == [
            dict(p["layout"], outlineLevel=4), breaker["layout"],
        ]
        await failure(client, "inspect_document", "inspection_failed", document=reopened,
                      view=dict(kind="layout", scope="sections"))

    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, native, "outline")
    after = assert_main_only_changed(source, output)
    before = ElementTree.fromstring(parts(source)["word/document.xml"]).find(qn("w:body"))
    assert ElementTree.tostring(after.find(qn("w:sectPr"))) == ElementTree.tostring(
        before.find(qn("w:sectPr")),
    )
    properties = after.find(f"{qn('w:p')}/{qn('w:pPr')}")
    assert properties.find(qn("w:outlineLvl")).get(qn("w:val")) == "4"
