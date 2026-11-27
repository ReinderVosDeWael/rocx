"""Snapshot-bound content controls over installed MCP stdio."""

import subprocess

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_fields import opened, simple, word
from test_ranges import inspect, saved
from test_stdio import anyio_backend, call, executable, failure, launch, parts, validate

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
    wrap(doc.add_paragraph("locked payload")._p, "locked", word("lock", val="sdtContentLocked"),
         word("richText"))
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
        sites = {(row["context"], row["text"]): row["anchor"] for row in sites}
        first, last = (sites["block", text] for text in ("block first", "block last"))
        body = (await call(client, "inspect_document", document=document))["body"]
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

    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_control_oracle")
    for action, path in outputs.items():
        expected = tmp_path / f"native-{action}.docx"
        subprocess.run([oracle, source, action, expected], check=True, timeout=20, cwd=tmp_path)
        assert path.read_bytes() == expected.read_bytes()
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
