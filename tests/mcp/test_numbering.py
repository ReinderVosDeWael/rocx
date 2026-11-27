"""Native list definitions, overrides, and direct body numbering over MCP."""

import json
from runpy import run_path
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
    ROOT, anyio_backend, assert_tool_failure, call, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio
NUMBERS = "resources/lists.xml"


def edit(action, **fields):
    return dict(kind="numbering", edit=dict(kind=action, **fields))


def level(index, start=1, **fields):
    return dict(level=index, start=start, format="decimal", text=f"%{index + 1}.") | fields


RICH = level(0, label_format=dict(bold=False, size=12), paragraph_format=dict(
    left_indent=12, tab_stops=[dict(position=36, alignment="left")],
), alignment="left", suffix="space", is_legal=False, paragraph_style="Normal")
OVERRIDE = level(1, 3, format="lowerRoman", label_format=dict(italic=True))


def source_doc(path, *, malformed=False):
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.add_paragraph("Number this")
    doc.add_paragraph("Unrelated")
    doc.sections[0].footer.paragraphs[0].text = "Keep this footer"
    if malformed:
        style = doc.styles.add_style("Duplicate", WD_STYLE_TYPE.PARAGRAPH)
        style.element.set(qn("w:styleId"), "Normal")
    doc.save(path)
    entries = parts(path)
    entries[NUMBERS] = entries.pop("word/numbering.xml")
    rel = "word/_rels/document.xml.rels"
    entries[rel] = entries[rel].replace(b'Target="numbering.xml"', b'Target="../resources/lists.xml"')
    entries["[Content_Types].xml"] = entries["[Content_Types].xml"].replace(
        b"/word/numbering.xml", b"/resources/lists.xml",
    )
    with ZipFile(path, "w") as output:
        for name, data in entries.items():
            output.writestr(name, data)


def native(source, command, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_numbering_oracle")
    subprocess.run([oracle, source, command, output], check=True, timeout=20)


async def preview(client, document, source, command, edits):
    result = await call(client, "preview_edits", document=document, edits=edits)
    out = source.parent / f"{command}.docx"
    expect = source.parent / f"native-{command}.docx"
    await saved(client, document, result, out)
    native(source, command, expect)
    assert out.read_bytes() == expect.read_bytes()
    return out


async def catalogs(client, document):
    values = {}
    for key in ("abstracts", "instances"):
        head = await inspect(client, document, "numbering-" + key, limit=1)
        values[key] = head[key]
        while head["next_cursor"] is not None:
            head = await inspect(client, document, "numbering-" + key, cursor=head["next_cursor"], limit=2)
            values[key].extend(head[key])
    return values


async def value(client, document, kind):
    return (await inspect(client, document, "resource-value", query={"kind":kind}))["value"]


async def test_numbering_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    view = validator_for(schema)(schema).is_valid
    for item in [
        dict(kind="numbering-abstracts", limit=100),
        dict(kind="numbering-instances", cursor="c"),
        dict(kind="paragraph-numbering", anchor="a"),
        dict(kind="resource-value", query=dict(kind="abstract-starts")),
    ]:
        assert view(dict(document="doc", view=item))
    assert not view(dict(document="doc", view=dict(kind="paragraph-numbering")))
    assert not view(dict(document="doc", view=dict(kind="numbering-instances", limit=101)))
    schema = schemas["preview_edits"]
    valid = validator_for(schema)(schema).is_valid
    for item in [
        edit("add-abstract", abstract_id=80, levels=[RICH]),
        edit("abstract-start", abstract_id=80, level=0, value=None),
        edit("instance-level", num_id=90, level=8, value=level(8)),
        edit("replace-instance", definition=dict(id=90, abstract_id=80)),
        edit("paragraph", anchor="a", value=dict(id=90, level=0)),
    ]:
        assert valid(dict(document="doc", edits=[item]))
    for item in [
        edit("instance-start", num_id=90, level=0),
        edit("abstract-level", abstract_id=80, level=9, value=None),
        edit("add-abstract", abstract_id=80, levels=[level(9)]),
        edit("add-abstract", abstract_id=80, levels=[level(0, extra=True)]),
        edit("paragraph", anchor="a", value=dict(id=90, level=-1)),
    ]:
        assert not valid(dict(document="doc", edits=[item]))


async def test_numbering_native_lifecycle(launch, tmp_path):
    source = tmp_path / "source.docx"
    source_doc(source)
    original = source.read_bytes()
    async with launch() as client:
        document = await opened(client, source)
        page = await inspect(client, document, "numbering-abstracts", limit=1)
        cursor = page["next_cursor"]
        assert cursor is not None
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="numbering-instances", cursor=cursor))
        body = (await call(client, "inspect_document", document=document))["body"]
        para = body[0]["anchor"]
        run = body[1]["runs"][0]["anchor"]
        assert (await inspect(client, document, "paragraph-numbering", anchor=para))["value"] is None
        foreign = await opened(client, source)
        other = (await call(client, "inspect_document", document=foreign))["body"][0]["anchor"]
        for error, anchor in [
            ("invalid_anchor", f"{document}/unknown"),
            ("wrong_anchor_type", run),
            ("wrong_owner", other),
        ]:
            await failure(client, "inspect_document", error, document=document,
                          view=dict(kind="paragraph-numbering", anchor=anchor))
            await failure(client, "preview_edits", error, document=document,
                          edits=[edit("paragraph", anchor=anchor, value=None)])
        create = [
            edit("instance-start", num_id=90, level=1, value=6),
            edit("paragraph", anchor=para, value=dict(id=90, level=1)),
            edit("instance-level", num_id=90, level=1, value=OVERRIDE),
            edit("add-instance", num_id=90, abstract_id=80),
            edit("add-abstract", abstract_id=80, levels=[RICH, level(1)]),
            dict(kind="run-text", anchor=run, text="Unrelated edited"),
        ]
        for invalid in [
            create + [edit("remove-abstract", abstract_id=80)],
            create + [edit("instance-start", num_id=90, level=2, value=2)],
            create + [edit("paragraph", anchor=para, value=None)],
            [edit("paragraph", anchor=para, value=dict(id=90, level=9))],
        ]:
            error = "invalid_input" if len(invalid) == 1 else "rejected_batch"
            await failure(client, "preview_edits", error, document=document, edits=invalid)
        created = await preview(client, document, source, "create", create)
        imported = await opened(client, created)
        await failure(client, "inspect_document", "wrong_owner", document=imported,
                      view=dict(kind="numbering-abstracts", cursor=cursor))
        again = (await call(client, "inspect_document", document=imported))["body"][0]["anchor"]
        assert (await inspect(client, imported, "paragraph-numbering", anchor=again))["value"] == dict(id=90, level=1)
        rich = await catalogs(client, imported)
        expected = tmp_path / "catalog.json"
        native(created, "inspect", expected)
        assert rich == json.loads(expected.read_bytes())
        abstract = next(row for row in rich["abstracts"] if row["abstract_id"] == 80)
        assert abstract["levels"][0]["label_format"] == dict(bold=False, size=12)
        assert abstract["levels"][0]["paragraph_format"]["left_indent"] == 12
        assert abstract["levels"][0]["suffix"] == "space" and abstract["levels"][0]["is_legal"] is False
        assert next(row for row in rich["instances"] if row["num_id"] == 90)["level_overrides"][0]["format"] == "lowerRoman"
        for invalid in [
            edit("remove-instance", num_id=90),
            edit("remove-abstract", abstract_id=80),
            edit("abstract-level", abstract_id=80, level=1, value=None),
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=imported, edits=[invalid])
        relinked = await preview(client, imported, created, "relink", [
            edit("relink-instance", num_id=90, abstract_id=81),
            edit("replace-abstract", abstract_id=80, levels=[level(0, 7)]),
            edit("add-abstract", abstract_id=81, levels=[level(0), level(1)]),
            edit("instance-level", num_id=90, level=1, value=None),
        ])
        imported = await opened(client, relinked)
        rows = await value(client, imported, "numbering-instances")
        expected_instances = tmp_path / "native-instances.json"
        native(relinked, "inspect-instances", expected_instances)
        assert rows == json.loads(expected_instances.read_bytes())
        inst = next(row for row in rows if row["num_id"] == 90)
        assert inst == dict(num_id=90, abstract_id=81, start_overrides=[dict(level=1, start=6)], replacement_levels=[])
        replaced = await preview(client, imported, relinked, "replace", [
            edit("abstract-level", abstract_id=80, level=1, value=level(1, 8)),
            edit("replace-instance", definition=dict(id=90, abstract_id=80,
                 start_overrides=[dict(level=0, start=2)], level_overrides=[OVERRIDE])),
            edit("abstract-start", abstract_id=81, level=0, value=None),
        ])
        imported = await opened(client, replaced)
        rows = await value(client, imported, "numbering-instances")
        native(replaced, "inspect-instances", expected_instances)
        assert rows == json.loads(expected_instances.read_bytes())
        inst = next(row for row in rows if row["num_id"] == 90)
        assert inst == dict(num_id=90, abstract_id=80, start_overrides=[dict(level=0, start=2)],
                            replacement_levels=[1])
        starts = {(r["abstract_id"], r["level"]): r["start"]
                  for r in await value(client, imported, "abstract-starts")}
        assert starts[81, 0] is None and starts[80, 1] == 8
        cleared = await preview(client, imported, replaced, "replace-empty", [
            edit("replace-instance", definition=dict(id=90, abstract_id=80)),
        ])
        imported = await opened(client, cleared)
        inst = next(row for row in await value(client, imported, "numbering-instances") if row["num_id"] == 90)
        assert inst == dict(num_id=90, abstract_id=80, start_overrides=[], replacement_levels=[])
        target = (await call(client, "inspect_document", document=imported))["body"][0]["anchor"]
        removed = await preview(client, imported, cleared, "remove", [
            edit("remove-abstract", abstract_id=80), edit("remove-abstract", abstract_id=81),
            edit("remove-instance", num_id=90), edit("paragraph", anchor=target, value=None),
        ])

    word = docx.Document(replaced)
    assert word.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:ilvl/@w:val") == ["1"]
    assert word.paragraphs[1].text == "Unrelated edited"
    nums = word.part.numbering_part.element
    assert nums.xpath("w:num[@w:numId='90']/w:lvlOverride/w:lvl/w:numFmt/@w:val") == ["lowerRoman"]
    after = docx.Document(removed)
    assert not after.paragraphs[0]._p.xpath("w:pPr/w:numPr")
    assert not after.part.numbering_part.element.xpath("w:num[@w:numId='90']")
    assert source.read_bytes() == original
    for before, after, changed in [
        (source, created, {NUMBERS, "word/document.xml"}),
        (created, relinked, {NUMBERS}),
        (relinked, replaced, {NUMBERS}),
        (replaced, cleared, {NUMBERS}),
        (cleared, removed, {NUMBERS, "word/document.xml"}),
    ]:
        a, b = parts(before), parts(after)
        assert a.keys() == b.keys()
        assert {k for k in a if a[k] != b[k]} == changed
    validate(source, created, relinked, replaced, cleared, removed)


async def test_numbering_level_fields_set_and_clear(launch, tmp_path):
    source = tmp_path / "source.docx"
    source_doc(source)
    legacy = dict(line_spacing=1.25, tab_stops=[dict(position=18, alignment="end")])
    abstract_levels = [
        level(0, alignment="left", suffix="tab"),
        level(1, 2, alignment="center", suffix="space", restart=0, is_legal=True),
        level(2, 3, alignment="right", suffix="nothing", restart=1, is_legal=False),
        level(3, 4, alignment="start", paragraph_format=legacy),
    ]
    unreferenced_levels = [level(0), level(8, 5, alignment="end")]
    label = dict(
        bold=False, italic=True, font="Arial", size=11.5, color=dict(theme="accent4", tint=36),
        underline="wave", subscript=True, strike=False, all_caps=False, small_caps=True,
        snap_to_grid=False, bold_complex_script=True, italic_complex_script=False,
        size_complex_script=13, rtl=False, complex_script=True, font_east_asia="MS Gothic",
        font_complex_script="Tahoma", language=dict(latin="en-GB", east_asia="ja-JP", bidi="ar-SA"),
    )
    paragraph = dict(
        alignment="justify-high", bidi=True, spacing_before=5.5, spacing_after=2,
        line_height=dict(kind="exact", value=12.5), left_indent=24, right_indent=6,
        first_line_indent=-12, keep_together=False, keep_with_next=True, page_break_before=False,
        widow_control=False, suppress_line_numbers=True, contextual_spacing=False,
        mirror_indents=True, snap_to_grid=False,
        tab_stops=[dict(position=36, alignment="decimal", leader="middleDot")],
    )
    override = level(2, 10, format="upperLetter", text="§ %3)", alignment="end", suffix="nothing",
                     restart=2, is_legal=True, paragraph_style="heading 1",
                     label_format=label, paragraph_format=paragraph)
    simple_override = level(3, 20, label_format=dict(superscript=True, color="auto"))

    def instance_level(index, value):
        return edit("instance-level", num_id=90, level=index, value=value)

    def instance_start(index, value):
        return edit("instance-start", num_id=90, level=index, value=value)

    async def reopen_and_check(client, output, *, clear):
        document = await opened(client, output)
        expected = tmp_path / "catalog.json"
        native(output, "inspect", expected)
        observed = await catalogs(client, document)
        assert observed == json.loads(expected.read_bytes())
        abstracts = {row["abstract_id"]: row for row in observed["abstracts"]}
        instance = next(row for row in observed["instances"] if row["num_id"] == 90)
        read_levels = [row | dict(has_opaque_content=False) for row in abstract_levels]
        read_levels[0]["start"] = 9 if clear else 1
        read_levels[3]["paragraph_format"] = dict(
            line_height=dict(kind="multiple", value=1.25), tab_stops=legacy["tab_stops"],
        )
        assert abstracts[80] == dict(abstract_id=80, levels=read_levels)
        assert abstracts[81] == dict(abstract_id=81, levels=[
            row | dict(has_opaque_content=False)
            for row in (unreferenced_levels[:1] if clear else unreferenced_levels)
        ])
        detailed = override | dict(has_opaque_content=False, paragraph_style="Heading1",
                                   paragraph_format=paragraph | dict(alignment="highKashida"))
        assert instance == dict(
            num_id=90, abstract_id=80, style_ids=[],
            start_overrides=([dict(level=3, start=14)] if clear else
                             [dict(level=2, start=12), dict(level=3, start=14)]),
            level_overrides=([detailed] if clear else
                             [detailed, simple_override | dict(has_opaque_content=False)]),
        )
        return document

    async with launch() as client:
        document = await opened(client, source)
        set_copy = await preview(client, document, source, "level-fields-set", [
            edit("add-abstract", abstract_id=80, levels=abstract_levels),
            edit("add-abstract", abstract_id=81, levels=unreferenced_levels),
            edit("add-instance", num_id=90, abstract_id=80),
            instance_start(2, 12), instance_start(3, 14),
            instance_level(2, override), instance_level(3, simple_override),
        ])
        document = await reopen_and_check(client, set_copy, clear=False)
        for action, invalid, reason in [
            ("reject-override-level-index", level(3), "mismatched replacement level"),
            ("reject-override-run-style", level(2, label_format=dict(style="DefaultParagraphFont")),
             "cannot contain run style references"),
            ("reject-override-paragraph-heading", level(2, paragraph_format=dict(heading=1)),
             "cannot contain style references or headings"),
        ]:
            diagnostic = tmp_path / f"{action}.txt"
            native(set_copy, action, diagnostic)
            result = await client.call_tool("preview_edits", dict(
                document=document, edits=[instance_level(2, invalid)],
            ))
            assert_tool_failure(result, "rejected_batch")
            expected_error = diagnostic.read_text()
            assert reason in expected_error and expected_error in result.structuredContent["message"]

        cleared = await preview(client, document, set_copy, "level-fields-clear", [
            edit("abstract-level", abstract_id=81, level=8, value=None),
            edit("abstract-start", abstract_id=80, level=0, value=9),
            instance_start(2, None), instance_level(3, None),
        ])
        await reopen_and_check(client, cleared, clear=True)

    before = parts(source)
    for output in (set_copy, cleared):
        after = parts(output)
        assert before.keys() == after.keys()
        assert {name for name in before if before[name] != after[name]} == {NUMBERS}
    nums = docx.Document(cleared).part.numbering_part.element
    assert nums.xpath("w:abstractNum[@w:abstractNumId='80']/w:lvl[@w:ilvl='0']/w:start/@w:val") == ["9"]
    overrides = nums.xpath("w:num[@w:numId='90']/w:lvlOverride")
    assert [(item.get(qn("w:ilvl")), [etree.QName(n).localname for n in item])
            for item in overrides] == [("2", ["lvl"]), ("3", ["startOverride"])]
    validate(set_copy, cleared)


async def test_numbering_independent_readers_ignore_bad_style_catalog(launch, tmp_path):
    source = tmp_path / "malformed.docx"
    source_doc(source, malformed=True)
    async with launch() as client:
        document = await opened(client, source)
        for kind in ("numbering-abstracts", "numbering-instances"):
            await failure(client, "inspect_document", "inspection_failed", document=document,
                          view=dict(kind=kind))
        assert await value(client, document, "abstract-starts")
        assert await value(client, document, "numbering-instances")


@pytest.mark.parametrize("opaque", [False, True], ids=["modeled", "opaque"])
async def test_numbering_level_free_alias_and_opaque_readback(launch, tmp_path, opaque):
    fixture = run_path(str(ROOT / "tests/fixtures/generate_numbering_catalog.py"))
    source = tmp_path / "linked.docx"
    source.write_bytes(fixture["source"]("opaque" if opaque else None))
    expected = tmp_path / "catalog.json"
    native(source, "inspect", expected)
    catalog = json.loads(expected.read_bytes())

    async with launch() as client:
        document = await opened(client, source)
        observed = await catalogs(client, document)
        assert observed == catalog
        abstracts = {row["abstract_id"]: row for row in observed["abstracts"]}
        instances = {row["num_id"]: row for row in observed["instances"]}
        assert abstracts[711] == dict(abstract_id=711, num_style_link="FixtureList", levels=[])
        assert abstracts[710]["style_link"] == "FixtureList"
        assert instances[712]["style_ids"] == ["FixtureList"]
        assert instances[713] == dict(num_id=713, abstract_id=711, style_ids=[],
                                      start_overrides=[], level_overrides=[])
        owner_level, = abstracts[710]["levels"]
        override, = instances[712]["level_overrides"]
        assert owner_level["has_opaque_content"] is opaque
        assert override["has_opaque_content"] is False
        assert owner_level["label_format"]["bold"] is False
        assert owner_level["format"] == "lowerRoman"

        body = (await call(client, "inspect_document", document=document))["body"]
        anchor = body[0]["anchor"]
        guards = [
            ("reject-alias-level", edit("abstract-level", abstract_id=711, level=0, value=level(0)),
             "inherited abstract has no direct editable level"),
            ("reject-owning-relink", edit("relink-instance", num_id=712, abstract_id=711),
             "cannot move a numbering style's owning instance"),
        ]
        if opaque:
            guards.extend([
                ("reject-opaque-level", edit("abstract-level", abstract_id=710, level=0, value=level(0)),
                 "cannot discard unmodeled abstract level markup"),
                ("reject-opaque-abstract", edit("replace-abstract", abstract_id=710, levels=[level(0)]),
                 "cannot discard unmodeled abstract level markup"),
            ])
        else:
            guards.append((
                "reject-lost-abstract-level", edit("replace-abstract", abstract_id=710, levels=[level(1)]),
                "would lose numbering levels of a surviving instance",
            ))
        for command, request, reason in guards:
            diagnostic = tmp_path / f"{command}.txt"
            native(source, command, diagnostic)
            native_message = diagnostic.read_text()
            assert reason in native_message
            rejected = await client.call_tool("preview_edits", {
                "document": document,
                "edits": [request],
            })
            assert_tool_failure(rejected, "rejected_batch")
            if rejected.structuredContent is not None:
                message = rejected.structuredContent["message"]
            else:
                message = rejected.content[0].text
            assert native_message in message

        if not opaque:
            updated = await preview(client, document, source, "edit-owner-level", [
                edit("abstract-level", abstract_id=710, level=0, value=level(0, 9)),
            ])
            changed = await catalogs(client, await opened(client, updated))
            owner = next(row for row in changed["abstracts"] if row["abstract_id"] == 710)
            assert owner["style_link"] == "FixtureList" and owner["levels"][0]["start"] == 9
            assert next(row for row in changed["abstracts"] if row["abstract_id"] == 711) == abstracts[711]
            assert changed["instances"] == catalog["instances"]
            updated_parts = parts(updated)
            original_parts = parts(source)
            assert updated_parts.keys() == original_parts.keys()
            (numbering_part,) = [p for p in original_parts if p.endswith("numbering.xml")]
            assert {p for p in original_parts if original_parts[p] != updated_parts[p]} == {numbering_part}
            validate(updated)

        assignment = dict(id=713, level=0)
        output = await preview(client, document, source, "assign-alias", [
            edit("paragraph", anchor=anchor, value=assignment),
        ])
        reopened = await opened(client, output)
        new_body = (await call(client, "inspect_document", document=reopened))["body"]
        stored = await inspect(client, reopened, "paragraph-numbering", anchor=new_body[0]["anchor"])
        assert stored["value"] == assignment
        assert await catalogs(client, reopened) == catalog

    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    assert docx.Document(output).paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["713"]
    validate(source, output)


async def test_numbering_creates_part_with_reciprocal_style_link(launch, tmp_path):
    source = tmp_path / "missing.docx"
    source_doc(source)
    entries = parts(source)
    relationships = "word/_rels/document.xml.rels"
    root = etree.fromstring(entries[relationships])
    relationship, = [r for r in root if r.get("Type").endswith("/numbering")]
    root.remove(relationship)
    entries[relationships] = etree.tostring(root)
    with ZipFile(source, "w") as output:
        for name, data in entries.items():
            output.writestr(name, data)
    async with launch() as client:
        document = await opened(client, source)
        assert await catalogs(client, document) == dict(abstracts=[], instances=[])
        assert await value(client, document, "abstract-starts") == []
        assert await value(client, document, "numbering-instances") == []
        body = (await call(client, "inspect_document", document=document))["body"]
        output = await preview(client, document, source, "create-part", [
            edit("add-instance", num_id=90, abstract_id=80),
            edit("add-abstract", abstract_id=80, levels=[level(0)]),
            edit("paragraph", anchor=body[0]["anchor"], value=dict(id=90, level=0)),
            dict(kind="style", edit=dict(kind="add", definition=dict(
                kind="numbering", name="First List Style", numbering_id=90,
            ))),
        ])
        reopened = await opened(client, output)
        catalog = await catalogs(client, reopened)
        assert catalog["abstracts"][0]["style_link"] == "FirstListStyle"
        assert catalog["instances"][0]["style_ids"] == ["FirstListStyle"]
        page = await inspect(client, reopened, "styles", limit=100)
        while not any(row["id"] == "FirstListStyle" for row in page["styles"]):
            assert page["next_cursor"] is not None
            page = await inspect(client, reopened, "styles", limit=100, cursor=page["next_cursor"])
        style = next(row for row in page["styles"] if row["id"] == "FirstListStyle")
        diagnostic = tmp_path / "native-replace-numbering-style.txt"
        native(output, "reject-replace-numbering-style", diagnostic)
        rejected = await client.call_tool("preview_edits", dict(document=reopened, edits=[
            dict(kind="style", edit=dict(kind="replace", anchor=style["anchor"],
                 definition=dict(kind="numbering", name="First List Style", numbering_id=90))),
        ]))
        assert_tool_failure(rejected, "rejected_batch")
        assert "cannot replace a linked numbering style" in diagnostic.read_text()
        assert diagnostic.read_text() in rejected.structuredContent["message"]
        assert await catalogs(client, reopened) == catalog
    result = parts(output)
    assert result[NUMBERS] == entries[NUMBERS]
    assert str(docx.Document(output).part.numbering_part.partname) != "/" + NUMBERS
    assert {name for name in entries if entries[name] != result[name]} == {
        relationships, "word/document.xml", "word/styles.xml", "[Content_Types].xml",
    }
    validate(output)
