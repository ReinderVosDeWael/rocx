"""Body inline candidates, original-coordinate edits, and native byte parity."""
import json
from pathlib import Path
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
IMAGES = Path(__file__).resolve().parents[1] / "fixtures" / "images"
TOGGLES = ("bold", "italic", "strike", "hidden")


def add_link(paragraph, label):
    link = OxmlElement("w:hyperlink")
    link.set(qn("r:id"), paragraph.part.relate_to(
        "https://example.org/inline", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    link.set(qn("w:tooltip"), "Original tip")
    link.set(qn("w:history"), "1")
    run, text = OxmlElement("w:r"), OxmlElement("w:t")
    text.text = label
    run.append(text)
    link.append(run)
    paragraph._p.append(link)


def make_source(path, *, properties=False):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    paragraph = source.add_paragraph("Lead")
    add_link(paragraph, "Linked")
    paragraph.add_run("Tail")
    source.add_paragraph("Sentinel")
    if properties:
        paragraph = source.add_paragraph()
        paragraph.add_run("Enable")
        for label in ("Disable", "Inherit"):
            run = paragraph.add_run(label)
            for toggle in TOGGLES:
                setattr(run.font, toggle, True)
        run = paragraph.add_run("Patch")
        run.font.size = docx.shared.Pt(15)
        run.underline = True
        add_link(paragraph, "Clear link")
        for image in ("sample.png", "sample.jpg"):
            picture = paragraph.add_run().add_picture(str(IMAGES / image))
            picture._inline.docPr.set("descr", image)
    source.save(path)


def edit(row, kind, **payload):
    return {"kind": "inline", "anchor": row["anchor"], "edit": {"kind": kind, **payload}}


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


async def test_ordinary_inline_properties_match_native(launch, tmp_path):
    source = tmp_path / "source.docx"
    make_source(source, properties=True)
    original = source.read_bytes()
    before = parts(source)
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_body_inline_oracle")

    def format_edit(row, kind, **fields):
        return edit(row, "format", action=dict(kind=kind, **fields))

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        view = {"kind": "body-inlines", "limit": 20}
        inspection = await call(client, "inspect_document", document=document, view=view)
        rows = inspection["body_inlines"]
        link, = [row for row in rows if (row["body_index"], row["inline_index"]) == (0, 1)]
        enable, disable, inherit, patch, clear_link, remove, describe = [
            row for row in rows if row["body_index"] == 2
        ]
        edits = [
            format_edit(row, "set-toggle", toggle=toggle, value=value)
            for row, value in [(enable, True), (disable, False), (inherit, None)]
            for toggle in TOGGLES
        ] + [
            format_edit(patch, "patch-format", set={"size": 13.5}, clear=["underline"]),
            edit(link, "hyperlink-tooltip", value="Updated & tip"),
            edit(link, "hyperlink-history", value=False),
            edit(clear_link, "hyperlink-tooltip", value=None),
            edit(clear_link, "hyperlink-history", value=None),
            edit(remove, "remove-object"),
            edit(describe, "drawing-description", value="Alt <updated> & kept"),
        ]
        await failure(client, "preview_edits", "rejected_batch", document=document, edits=[
            *edits, format_edit(enable, "patch-format", clear=["bold"]),
        ])

        async def compare(mode, batch):
            output = tmp_path / f"{mode}.docx"
            native = tmp_path / f"native-{mode}.docx"
            result = subprocess.run([oracle, mode, native, source], check=True,
                                    capture_output=True, timeout=20)
            preview = await call(client, "preview_edits", document=document, edits=batch)
            assert preview["changes"]["inline_changes"] == json.loads(result.stdout)
            await call(client, "save_copy", document=document, preview=preview["preview"],
                       output=output.name)
            assert output.read_bytes() == native.read_bytes()
            return output, preview["changes"]["inline_changes"]

        output, _ = await compare("mixed", edits)
        noop, changes = await compare("noop", [
            *[format_edit(enable, "set-toggle", toggle=toggle, value=None) for toggle in TOGGLES],
            format_edit(patch, "patch-format", set={"size": 15}, clear=["color"]),
            edit(link, "hyperlink-tooltip", value="Original tip"),
            edit(link, "hyperlink-history", value=True),
            edit(describe, "drawing-description", value="sample.jpg"),
        ])
        assert changes == [] and parts(noop) == before
        assert await call(client, "inspect_document", document=document, view=view) == inspection
        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=edits)

    assert source.read_bytes() == original
    after = parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {MAIN}
    lead, sentinel, edited = docx.Document(output).paragraphs
    assert lead.text == "LeadLinkedTail" and sentinel.text == "Sentinel"
    assert edited.text == "EnableDisableInheritPatchClear link"
    assert [link.url for paragraph in (lead, edited) for link in paragraph.hyperlinks] == [
        "https://example.org/inline", "https://example.org/inline",
    ]
    enable_run, disable_run, inherit_run, patch_run, _ = edited.runs
    for run, value in [(enable_run, True), (disable_run, False), (inherit_run, None)]:
        assert [getattr(run.font, toggle) for toggle in TOGGLES] == [value] * len(TOGGLES)
    assert patch_run.font.size.pt == 13.5 and patch_run.underline is None
    root = ElementTree.fromstring(after[MAIN])
    set_link, unset_link = root.iter(qn("w:hyperlink"))
    assert (set_link.get(qn("w:tooltip")), set_link.get(qn("w:history"))) == ("Updated & tip", "0")
    assert unset_link.get(qn("w:tooltip")) is unset_link.get(qn("w:history")) is None
    old_pictures = list(ElementTree.fromstring(before[MAIN]).iter(qn("wp:docPr")))
    picture, = root.iter(qn("wp:docPr"))
    assert picture.attrib == dict(old_pictures[1].attrib, descr="Alt <updated> & kept")
    validate(output, noop)
