"""Snapshot-bound content controls over installed MCP stdio."""

import json
import subprocess

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_fields import opened, simple, word
from test_ranges import inspect, saved
from test_stdio import (
    anyio_backend, assert_tool_failure, call, executable, failure, launch, make_entry_source,
    parts, validate,
)

pytestmark = pytest.mark.anyio


def wrap(node, tag, *properties):
    control = word("sdt")
    props = word("sdtPr")
    props.append(word("tag", val=tag))
    props.extend(properties)
    control.append(props)
    payload = word("sdtContent")
    control.append(payload)
    node.addprevious(control)
    payload.append(node)
    return control


def make_source(path):
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    wrap(doc.add_paragraph("old plain")._p, "plain", word("id", val="+017"),
         word("text", multiLine="0"))
    paragraph = doc.add_paragraph()
    wrap(paragraph.add_run("old inline")._r, "inline", word("richText"))
    dropdown = word("dropDownList", lastValue="Alpha")
    dropdown.append(word("listItem", displayText="Alpha", value="Alpha"))
    dropdown.append(word("listItem", displayText="Beta", value="Beta"))
    wrap(doc.add_paragraph("Alpha")._p, "choice", dropdown)
    paragraph = doc.add_paragraph()
    paragraph.add_run("promoted").italic = True
    wrap(paragraph._p, "unwrap", word("richText"))
    wrap(doc.add_paragraph("old block")._p, "block", word("richText"))
    wrap(doc.add_paragraph("existing cache")._p, "define", word("richText"))
    locked = wrap(doc.add_paragraph("locked payload")._p, "locked",
                  word("lock", val="sdtContentLocked"), word("richText"))
    locked.find(qn("w:sdtPr")).insert(0, word("alias", val="Locked & named"))
    doc.add_paragraph("block first").runs[0].bold = True
    doc.add_paragraph("block last")
    doc.add_table(1, 1).cell(0, 0).text = "table text"
    doc.add_paragraph("outside")
    comment = doc.add_comment(doc.add_paragraph("comment Target").runs, text="comment payload")
    wrap(comment.paragraphs[0].add_run("within comment")._r, "comment-control", word("richText"))
    footer = doc.sections[0].footer.paragraphs[0]
    wrap(footer.add_run("footer payload")._r, "footer-control", word("richText"))
    doc.add_section()  # Inherits the same footer part.
    doc.save(path)


def change(action, **fields):
    return dict(kind="control", edit=dict(kind=action, **fields))


def native(source, action, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_control_oracle")
    subprocess.run([oracle, source, action, output], check=True, timeout=20, cwd=source.parent)
    return output.read_bytes()


async def test_control_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    view = validator_for(schema)(schema)
    for kind in ("controls", "control-sites", "story-controls"):
        assert view.is_valid(dict(document="doc", view=dict(kind=kind, limit=1)))
        assert not view.is_valid(dict(document="doc", view=dict(kind=kind, limit=101)))
    schema = schemas["preview_edits"]
    preview = validator_for(schema)(schema)
    for request in [
        change("wrap", first="first", last="last", tag="tag"),
        change("patch", anchor="a", text="value"),
        change("unwrap", anchor="a"),
        change("define", anchor="a", text=dict(kind="plain", multi_line=False)),
        change("choice", anchor="a", selected="code"),
        change("replace-inlines", anchor="a", inlines=[dict(kind="text", text="x")]),
        change("replace-blocks", anchor="a", blocks=[dict(kind="paragraph", paragraph=dict(inlines=[]))]),
    ]:
        assert preview.is_valid(dict(document="doc", edits=[request]))
    for request in [
        change("wrap", first="a", last=None, tag="tag"),
        change("wrap", first="a"),
        change("unwrap", anchor="a", tag="extra"),
        change("define", anchor="a", text=dict(kind="unknown")),
        change("define", anchor="a", text=dict(kind="rich"), id=-1),
        change("choice", anchor="a", selected="x", items=[dict(value="x")]),
    ]:
        assert not preview.is_valid(dict(document="doc", edits=[request]))


async def test_control_lifecycle(launch, tmp_path):
    source = tmp_path / "source.docx"
    make_source(source)
    original = source.read_bytes()
    outputs = {}
    async with launch() as client:
        document = await opened(client, source)
        result = await inspect(client, document, "controls")
        controls = result["controls"]
        anchors = {row["tag"]: row["anchor"] for row in controls}
        assert list(anchors) == ["plain", "inline", "choice", "unwrap", "block", "define", "locked"]
        assert {k: controls[0][k] for k in ("id", "kind", "context", "multi_line", "bound")} == {
            "id": "+017", "kind": "text", "context": "block", "multi_line": "0", "bound": False,
        }
        assert controls[2]["selected"] == "Alpha"
        assert controls[2]["choices"] == [
            dict(display_text="Alpha", value="Alpha"), dict(display_text="Beta", value="Beta"),
        ]
        head = await inspect(client, document, "controls", limit=1)
        tail = await inspect(client, document, "controls", cursor=head["next_cursor"])
        assert head["controls"] + tail["controls"] == controls
        assert tail["next_cursor"] is None
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="control-sites", cursor=head["next_cursor"]))
        stories = (await inspect(client, document, "story-controls"))["story_controls"]
        assert [(r["tag"], r["owner_part"], r["entry_id"], r["entry_index"]) for r in stories] == [
            ("footer-control", "word/footer1.xml", None, None),
            ("comment-control", "word/comments.xml", "0", 0),
        ]
        assert all("anchor" not in row for row in stories)
        sites = (await inspect(client, document, "control-sites"))["control_sites"]
        expected = json.loads(native(source, "inspect", tmp_path / "native-readback.json"))
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in controls] == expected["controls"]
        assert stories == expected["story_controls"]
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in sites] == expected["control_sites"]
        first_page = await inspect(client, document, "control-sites", limit=4)
        second_page = await inspect(client, document, "control-sites", cursor=first_page["next_cursor"])
        assert first_page["control_sites"] + second_page["control_sites"] == sites
        assert second_page["next_cursor"] is None
        sites = {(row["context"], row["text"]): row["anchor"] for row in sites}
        first, last = (sites["block", text] for text in ("block first", "block last"))
        body = (await call(client, "inspect_document", document=document))["body"]
        assert [row for row in body if row["type"] == "content-control"] == expected["body_controls"]
        other = await opened(client, source)
        foreign = (await inspect(client, other, "control-sites"))["control_sites"][0]["anchor"]
        patch = change("patch", anchor=anchors["plain"], tag="patched", alias="New alias",
                       text="updated plain")
        for error, request in [
            ("wrong_owner", [change("wrap", first=first, last=foreign, tag="bad")]),
            ("wrong_anchor_type", [change("wrap", first=anchors["plain"], tag="bad")]),
            ("wrong_anchor_type", [change("unwrap", anchor=first)]),
            ("invalid_anchor", [change("unwrap", anchor=f"{document}/control:99")]),
            ("rejected_batch", [change("wrap", first=last, last=first, tag="bad")]),
            ("rejected_batch", [change("wrap", first=first, last=sites["inline", "block last"], tag="bad")]),
            ("rejected_batch", [change("unwrap", anchor=anchors["locked"])]),
            ("rejected_batch", [change("choice", anchor=anchors["choice"], selected="missing")]),
            ("rejected_batch", [change("replace-inlines", anchor=anchors["block"],
                                       inlines=[dict(kind="text", text="wrong context")])]),
            ("rejected_batch", [patch, change("unwrap", anchor=anchors["plain"])]),
        ]:
            await failure(client, "preview_edits", error, document=document, edits=request)
        assert await inspect(client, document, "controls") == result
        for action, edits in {
            "mixed": [
                patch,
                change("replace-inlines", anchor=anchors["inline"],
                       inlines=[dict(kind="text", text="new rich text", format=dict(bold=True))]),
                change("choice", anchor=anchors["choice"], selected="Gamma",
                       items=[dict(display_text=s, value=s) for s in ("Alpha", "Gamma")]),
                change("unwrap", anchor=anchors["unwrap"]),
                change("replace-blocks", anchor=anchors["block"], blocks=[
                    dict(kind="paragraph", paragraph=dict(inlines=[dict(kind="text", text="new block")])),
                ]),
                change("define", anchor=anchors["define"], id=42, text=dict(kind="plain", multi_line=False)),
                dict(kind="run-text", anchor=body[10]["runs"][0]["anchor"], text="edited outside"),
            ],
            "block": [change("wrap", first=first, last=last, tag="wrapped")],
            "inline": [change("wrap", first=sites["inline", "block first"], tag="wrapped")],
            "row": [change("wrap", first=sites["row", "table text"], tag="wrapped")],
            "cell": [change("wrap", first=sites["cell", "table text"], tag="wrapped")],
        }.items():
            output = tmp_path / f"{action}.docx"
            draft = await call(client, "preview_edits", document=document, edits=edits)
            await saved(client, document, draft, output)
            outputs[action] = output
        reopened = await opened(client, outputs["mixed"])
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[patch])
        rows = (await inspect(client, reopened, "controls"))["controls"]
        assert [r["tag"] for r in rows] == ["patched", "inline", "choice", "block", None, "locked"]
        assert rows[2]["selected"] == "Gamma"
        assert (rows[4]["id"], rows[4]["kind"], rows[4]["multi_line"]) == ("42", "text", "0")

    for action, path in outputs.items():
        expected = tmp_path / f"native-{action}.docx"
        assert path.read_bytes() == native(source, action, expected)
        old, new = parts(source), parts(path)
        assert old.keys() == new.keys()
        assert {name for name in old if old[name] != new[name]} == {"word/document.xml"}
        if action != "mixed":
            root = etree.fromstring(new["word/document.xml"])
            ns = {"w": qn("w:p").split("}")[0][1:]}
            expected_parent = {"block": "body", "inline": "p", "row": "tbl", "cell": "tr"}
            expected_payload = {"block": ["p", "p"], "inline": ["r"], "row": ["tr"], "cell": ["tc"]}
            wrapper, = root.xpath(".//w:sdt[w:sdtPr/w:tag/@w:val='wrapped']", namespaces=ns)
            assert etree.QName(wrapper.getparent()).localname == expected_parent[action]
            assert [etree.QName(child).localname for child in wrapper.find(qn("w:sdtContent"))] == expected_payload[action]
    readback = docx.Document(outputs["mixed"])
    promoted = next(p for p in readback.paragraphs if p.text == "promoted")
    assert promoted.runs[0].italic
    assert "edited outside" in [p.text for p in readback.paragraphs]
    root = etree.fromstring(parts(outputs["mixed"])["word/document.xml"])
    ns = {"w": qn("w:p").split("}")[0][1:]}
    def control_xpath(tag, path):
        return root.xpath(f".//w:sdt[w:sdtPr/w:tag/@w:val='{tag}']/{path}", namespaces=ns)
    assert control_xpath("patched", "w:sdtContent//w:t/text()") == ["updated plain"]
    assert control_xpath("patched", "w:sdtPr/w:alias/@w:val") == ["New alias"]
    assert control_xpath("inline", "w:sdtContent/w:r/w:rPr/w:b")
    assert control_xpath("inline", "w:sdtContent//w:t/text()") == ["new rich text"]
    assert control_xpath("choice", "w:sdtContent//w:t/text()") == ["Gamma"]
    assert control_xpath("block", "w:sdtContent//w:t/text()") == ["new block"]
    assert source.read_bytes() == original
    validate(source, *outputs.values())


def make_wrapper_variants(path):
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    paragraph = doc.add_paragraph("before ")
    run = paragraph.add_run("promoted inline")
    run.italic = True
    wrap(run._r, "inline", word("richText"))
    paragraph.add_run(" after")
    # Row and cell targets live in separate original body blocks.
    table = doc.add_table(2, 1)
    table.cell(0, 0).text = "promoted row"
    table.cell(1, 0).text = "next row"
    wrap(table.rows[0]._tr, "row", word("richText"))
    table = doc.add_table(1, 2)
    table.cell(0, 0).text = "promoted cell"
    table.cell(0, 1).text = "next cell"
    wrap(table.cell(0, 0)._tc, "cell", word("richText"))
    plain = wrap(doc.add_paragraph("keep this cache")._p, "plain", word("id", val="+042"),
                 word("text", multiLine="1"))
    plain.find(qn("w:sdtPr")).insert(0, word("alias", val="Old alias"))
    wrap(doc.add_paragraph("First label")._p, "define-choice", word("richText"))
    combo = word("comboBox", lastValue="combo-code")
    combo.append(word("listItem", displayText="Combo label", value="combo-code"))
    for tag, prop, text in [
        ("combo", combo, "Combo label"),
        ("date", word("date", fullDate="2026-11-27T00:00:00Z"), "2026-11-27"),
        ("picture", word("picture"), ""),
        ("opaque", word("group"), "group payload"),
        ("implicit", None, "no explicit type"),
    ]:
        wrap(doc.add_paragraph(text)._p, tag, *([prop] if prop is not None else []))
    for i in range(3):
        wrap(doc.add_paragraph(f"lock cache {i}")._p, f"lock-{i}", word("richText"))
    doc.sections[0].header.paragraphs[0].text = "untouched header"
    doc.save(path)


async def test_control_wrapper_variants_native_bytes(launch, tmp_path):
    source = tmp_path / "variants.docx"
    make_wrapper_variants(source)
    original = source.read_bytes()
    items = [dict(display_text="First label", value="one"),
             dict(display_text="Second label", value="two")]

    async def findings(client, document, path):
        controls = (await inspect(client, document, "controls"))["controls"]
        expected = json.loads(native(path, "inspect", tmp_path / "native-controls.json"))
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in controls] == expected["controls"]
        assert len({row["anchor"] for row in controls}) == len(controls)
        return {row["tag"]: row for row in controls}

    async def preview(client, document, path, name, action, edits):
        result = await call(client, "preview_edits", document=document, edits=edits)
        output = tmp_path / f"{name}.docx"
        await saved(client, document, result, output)
        assert output.read_bytes() == native(path, action, tmp_path / f"native-{name}.docx")
        return output

    async with launch() as client:
        document = await opened(client, source)
        rows = await findings(client, document, source)
        assert [(row["context"], row["kind"]) for row in rows.values()] == [
            ("inline", "richText"), ("row", "richText"), ("cell", "richText"),
            ("block", "text"), ("block", "richText"), ("block", "comboBox"),
            ("block", "date"), ("block", "picture"), ("block", "opaque"),
            ("block", "richText"),
            *[("block", "richText")] * 3,
        ]
        assert rows["combo"]["selected"] == "combo-code"
        assert rows["combo"]["choices"] == [dict(display_text="Combo label", value="combo-code")]

        def edit(target, action, **fields):
            return change(action, anchor=rows[target]["anchor"], **fields)

        dropdown = dict(kind="drop-down", items=items, selected="one")
        locked = await preview(client, document, source, "locks", "locks", [
            edit(f"lock-{i}", "define", tag=f"lock-{i}", alias=f"Alias {i} & Café", lock=lock,
                 text=dict(kind="plain", **multi_line))
            for i, (lock, multi_line) in enumerate(
                [("control", dict(multi_line=True)), ("content", {}), ("both", dict(multi_line=False))]
            )
        ])
        locked_doc = await opened(client, locked)
        locked_rows = await findings(client, locked_doc, locked)
        assert [(locked_rows[f"lock-{i}"]["lock"], locked_rows[f"lock-{i}"]["multi_line"],
                 locked_rows[f"lock-{i}"]["alias"]) for i in range(3)] == [
            ("sdtLocked", "1", "Alias 0 & Café"), ("contentLocked", None, "Alias 1 & Café"),
            ("sdtContentLocked", "0", "Alias 2 & Café"),
        ]
        result = await client.call_tool("preview_edits", dict(document=locked_doc, edits=[
            change("define", anchor=locked_rows["lock-0"]["anchor"], text=dict(kind="rich")),
        ]))
        assert_tool_failure(result, "rejected_batch")
        native_error = native(locked, "locked-define", tmp_path / "native-locked-error.txt").decode()
        assert result.structuredContent["message"] == f"preview: {native_error}; revise the edits and retry"
        for invalid, diagnostic in [
            (edit("row", "define", text=dict(kind="plain", multi_line=True)),
             "row and cell controls require rich text"),
            (edit("cell", "define", text=dropdown), "row and cell controls require rich text"),
            (edit("define-choice", "define", text={**dropdown, "selected": "two"}),
             "selected choice must match cached text"),
            (edit("define-choice", "define", id=42, text=dict(kind="rich")),
             "duplicate content control ID"),
            (edit("combo", "choice", selected="combo-code"), "expected block or inline drop-down control"),
            (edit("opaque", "unwrap"), "opaque or ambiguous control properties"),
        ]:
            result = await client.call_tool("preview_edits", dict(
                document=document, edits=[edit("inline", "unwrap"), invalid],
            ))
            assert_tool_failure(result, "rejected_batch")
            assert diagnostic in result.structuredContent["message"]
        assert await findings(client, document, source) == rows

        defined = await preview(client, document, source, "defined", "promote-and-define", [
            *(edit(tag, "unwrap") for tag in ("inline", "row", "cell")),
            edit("plain", "define", tag="rich", lock="unlocked", text=dict(kind="rich")),
            edit("define-choice", "define", tag="dropdown", id=43, text=dropdown),
        ])
        document = await opened(client, defined)
        rows = await findings(client, document, defined)
        assert list(rows) == [
            "rich", "dropdown", "combo", "date", "picture", "opaque", "implicit",
            "lock-0", "lock-1", "lock-2",
        ]
        assert {key: rows["rich"][key] for key in ("kind", "lock", "alias", "id", "multi_line")} == {
            "kind": "richText", "lock": "unlocked", "alias": None, "id": None, "multi_line": None,
        }
        assert (rows["dropdown"]["kind"], rows["dropdown"]["id"], rows["dropdown"]["selected"],
                rows["dropdown"]["choices"]) == ("dropDownList", "43", "one", items)
        selected = await preview(client, document, defined, "selected", "select", [
            edit("dropdown", "choice", selected="two"),
        ])
        document = await opened(client, selected)
        rows = await findings(client, document, selected)
        assert (rows["dropdown"]["selected"], rows["dropdown"]["choices"]) == ("two", items)
        noop = await preview(client, document, selected, "noop", "select", [
            edit("dropdown", "choice", selected="two"),
        ])

    readback = docx.Document(defined)
    paragraph = readback.paragraphs[0]
    assert [run.text for run in paragraph.runs] == ["before ", "promoted inline", " after"]
    assert paragraph.runs[1].italic
    assert [[cell.text for cell in row.cells] for row in readback.tables[0].rows] == [
        ["promoted row"], ["next row"],
    ]
    assert [cell.text for cell in readback.tables[1].rows[0].cells] == ["promoted cell", "next cell"]

    ns = {"w": qn("w:p").split("}")[0][1:]}
    for path, cache, value in [(defined, "First label", "one"), (selected, "Second label", "two")]:
        root = etree.fromstring(parts(path)["word/document.xml"])
        dropdown, = root.xpath(".//w:sdt[w:sdtPr/w:tag/@w:val='dropdown']", namespaces=ns)
        assert dropdown.xpath("w:sdtContent//w:t/text()", namespaces=ns) == [cache]
        choices, = dropdown.xpath("w:sdtPr/w:dropDownList", namespaces=ns)
        assert choices.get(qn("w:lastValue")) == value
        assert [dict(display_text=i.get(qn("w:displayText")), value=i.get(qn("w:value")))
                for i in choices] == items
        assert root.xpath(".//w:sdt[w:sdtPr/w:tag/@w:val='rich']/w:sdtContent//w:t/text()",
                          namespaces=ns) == ["keep this cache"]
    for before, after in [(source, defined), (source, locked), (defined, selected)]:
        old, new = parts(before), parts(after)
        assert old.keys() == new.keys()
        assert {name for name in old if old[name] != new[name]} == {"word/document.xml"}
    assert noop.read_bytes() == selected.read_bytes()
    assert source.read_bytes() == original
    validate(source, defined, locked, selected, noop)


async def test_story_control_typed_findings(launch, tmp_path):
    source = tmp_path / "relocated.docx"
    make_entry_source(source)
    doc = docx.Document(source)
    header = doc.sections[0].header.paragraphs[0]
    wrap(header.add_run("header payload")._r, "header", word("richText"))
    doc.add_section()  # Shares the same effective header.
    for kind, reltype in [("footnote", RELATIONSHIP_TYPE.FOOTNOTES),
                          ("endnote", RELATIONSHIP_TYPE.ENDNOTES)]:
        part = doc.part.part_related_by(reltype)
        root = etree.fromstring(part.blob)
        entry, = [node for node in root if node.get(qn("w:id")) == "007"]
        run = entry.find(qn("w:p")).findall(qn("w:r"))[1]
        wrap(run, kind, word("text", multiLine="true"))
        part._blob = etree.tostring(root, encoding="UTF-8", xml_declaration=True)
    doc.save(source)
    expected = json.loads(native(source, "inspect", tmp_path / "story-findings.json"))

    async with launch() as client:
        document = await opened(client, source)
        first = await inspect(client, document, "story-controls", limit=1)
        tail = await inspect(client, document, "story-controls", cursor=first["next_cursor"])
        assert tail["next_cursor"] is None
        rows = first["story_controls"] + tail["story_controls"]
        assert rows == expected["story_controls"]
        assert {(r["tag"], r["owner_part"], r["entry_id"], r["entry_index"], r["multi_line"])
                for r in rows} == {
            ("header", "word/header1.xml", None, None, None),
            ("footnote", "word/notes/footnotes.xml", "007", 1, "true"),
            ("endnote", "word/notes/endnotes.xml", "007", 1, "true"),
        }


async def test_control_native_guards(launch, tmp_path):
    source = tmp_path / "guarded.docx"
    doc = docx.Document()
    bound = doc.add_paragraph("bound")
    wrap(bound._p, "bound", word("dataBinding", xpath="/root/value",
         storeItemID="{6792EBAD-76E0-473F-B2B9-D8A2C392D58F}"), word("text"))
    nested = doc.add_paragraph()
    wrap(nested.add_run("nested")._r, "inner", word("richText"))
    wrap(nested._p, "outer", word("richText"))
    paragraph = doc.add_paragraph()
    wrap(paragraph.add_run("inline")._r, "with-field", word("richText"))
    paragraph._p.append(simple(" PAGE ", "1"))
    doc.save(source)
    async with launch() as client:
        document = await opened(client, source)
        rows = (await inspect(client, document, "controls"))["controls"]
        expected = json.loads(native(source, "inspect", tmp_path / "native-guarded.json"))
        assert [{k: v for k, v in row.items() if k != "anchor"} for row in rows] == expected["controls"]
        assert next(row for row in rows if row["tag"] == "bound")["bound"] is True
        anchors = {row["tag"]: row["anchor"] for row in rows}
        targets = (await inspect(client, document, "fields"))["fields"]
        for edits in [
            [change("patch", anchor=anchors["bound"], text="no")],
            [change("unwrap", anchor=anchors["inner"])],
            [change("unwrap", anchor=anchors["with-field"]),
             dict(kind="field", anchor=targets[0]["anchor"], edit=dict(kind="remove"))],
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=edits)
        await call(client, "preview_edits", document=document,
                   edits=[change("unwrap", anchor=anchors["with-field"])])
