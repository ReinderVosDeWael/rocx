"""Body inline candidates, original-coordinate edits, and native byte parity."""
import json
import subprocess
from xml.etree import ElementTree

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
import pytest

from test_stdio import anyio_backend, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio
MAIN = "word/document.xml"


def make_source(path):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    paragraph = source.add_paragraph("Lead")
    link = OxmlElement("w:hyperlink")
    link.set(qn("r:id"), source.part.relate_to(
        "https://example.org/inline", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    link.set(qn("w:tooltip"), "Original tip")
    link.set(qn("w:history"), "1")
    run, text = OxmlElement("w:r"), OxmlElement("w:t")
    text.text = "Linked"
    run.append(text)
    link.append(run)
    paragraph._p.append(link)
    paragraph.add_run("Tail")
    source.add_paragraph("Sentinel")
    source.save(path)


@pytest.mark.parametrize("mode", ["mixed", "remove"])
async def test_body_inline_edits_match_native(launch, tmp_path, mode):
    source, native, output = (tmp_path / name for name in ["source.docx", "native.docx", "out.docx"])
    make_source(source)
    original = source.read_bytes()
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_inline_oracle")
    result = subprocess.run([binary, mode, native, source], check=True, capture_output=True, timeout=20)
    native_report = json.loads(result.stdout)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        page = await call(client, "inspect_document", document=document,
                          view={"kind": "body-inlines", "limit": 2})
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view={"kind": "fields", "cursor": page["next_cursor"]})
        lead, link = page["body_inlines"]
        next_page = await call(client, "inspect_document", document=document,
                               view={"kind": "body-inlines", "cursor": page["next_cursor"]})
        tail, sentinel = next_page["body_inlines"]
        assert [row["inline_index"] for row in [lead, link, tail]] == [0, 1, 2]
        assert [row["body_index"] for row in [lead, link, tail, sentinel]] == [0, 0, 0, 1]
        assert (link["kind"], link["tooltip"], link["history_xml"]) == ("hyperlink", "Original tip", "1")
        assert link["children"][0]["text"] == "Linked"

        def edit(row, kind, **payload):
            return {"kind": "inline", "anchor": row["anchor"], "edit": {"kind": kind, **payload}}

        if mode == "mixed":
            edits = [
                edit(lead, "insert-after", inline={
                    "kind": "field", "instruction": {"kind": "page"},
                    "children": [{"kind": "text", "text": "1"}],
                }),
                edit(link, "hyperlink-tooltip", value=None),
                edit(link, "hyperlink-history", value=False),
                edit(tail, "replace", inline={"kind": "fraction", "numerator": "1", "denominator": "2"}),
                edit(sentinel, "format", action={"kind": "set-bold", "value": True}),
            ]
            await failure(client, "preview_edits", "invalid_input", document=document,
                          edits=[edit(link, "hyperlink-tooltip")])
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=[*edits, edit(link, "remove-hyperlink")])
        else:
            edits = [edit(link, "remove-hyperlink")]

        other = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=other, edits=edits)
        preview = await call(client, "preview_edits", document=document, edits=edits)
        assert preview["changes"]["inline_changes"] == native_report
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)

    assert source.read_bytes() == original and output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {MAIN}
    readback = docx.Document(output)
    assert readback.paragraphs[1].text == "Sentinel"
    root = ElementTree.fromstring(after[MAIN])
    if mode == "mixed":
        assert readback.paragraphs[0].hyperlinks[0].url == "https://example.org/inline"
        hyperlink, = root.iter(qn("w:hyperlink"))
        assert hyperlink.get(qn("w:history")) == "0" and hyperlink.get(qn("w:tooltip")) is None
        assert next(root.iter(qn("w:fldSimple"))).get(qn("w:instr")) == " PAGE "
        assert len(list(root.iter(qn("m:f")))) == 1
        assert readback.paragraphs[1].runs[0].bold is True
    else:
        assert readback.paragraphs[0].text == "LeadTail"
        assert not readback.paragraphs[0].hyperlinks
    validate(output)
