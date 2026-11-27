"""Installed stdio parity for additive inline declaration fields."""

from copy import deepcopy
import subprocess

import docx
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_stdio import PNG, anyio_backend, call, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio
URL = "https://example.org/payload"
SOURCE_NAME = "logo & seal.jpg"  # Label only: the supplied bytes are PNG.
TOOLTIP = "Open <example>"


def declarations(rich):
    tab = {"kind": "tab"}
    br = {"kind": "break"}
    image = {"kind": "image", "bytes": list(PNG.read_bytes())}
    link = {
        "kind": "hyperlink",
        "target": {"kind": "url", "url": URL},
        "children": [tab, br],
    }
    if rich:
        tab.update(format={"bold": False})
        br.update(type="column", format={"italic": True})
        image.update(
            source_name=SOURCE_NAME, format={"strike": True},
            transformation={"width": 24}, alt_text={"description": "A logo"},
        )
        link.update(tooltip=TOOLTIP, history=False)
    return link, image


def insertion(document, targets):
    return dict(document=document, edits=[
        dict(kind="inline", anchor=anchor, edit=dict(kind="insert-after", inline=inline))
        for anchor, inline in targets
    ])


@pytest.mark.parametrize("mode", ["rich", "legacy"])
async def test_inline_payload_native_bytes(launch, tmp_path, mode):
    rich = mode == "rich"
    source, native, output = [tmp_path / name for name in ("source.docx", "native.docx", "out.docx")]
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("Original").add_run("Tail")
    fixture.sections[0].header.paragraphs[0].text = "Untouched"
    fixture.save(source)
    original = source.read_bytes()
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_inline_payload_oracle")
    subprocess.run([oracle, source, native, mode], check=True, cwd=tmp_path, timeout=20)

    async with launch() as client:
        (tool,) = [item for item in (await client.list_tools()).tools if item.name == "preview_edits"]
        schema = tool.inputSchema
        validator = validator_for(schema)(schema)
        document = (await call(client, "open_document", path=source.name))["document"]
        lead, tail = (await call(client, "inspect_document", document=document,
                                view={"kind": "body-inlines"}))["body_inlines"]
        link, image = declarations(rich)
        targets = [(lead["anchor"], link), (tail["anchor"], image)]
        request = insertion(document, targets)
        validator.validate(request)
        if rich:
            for key in ("path", "url"):
                invalid = {**image, key: "sample.png"}
                rejected = insertion(document, [(tail["anchor"], invalid)])
                assert not validator.is_valid(rejected)
                await failure(client, "preview_edits", "invalid_input", **rejected)
            for index in range(2):
                invalid = deepcopy(link)
                invalid["children"][index]["format"] = {"size": -1}
                await failure(client, "preview_edits", "rejected_batch",
                              **insertion(document, [(lead["anchor"], invalid)]))
            invalid = {**image, "format": {"size": -1}}
            await failure(client, "preview_edits", "rejected_batch",
                          **insertion(document, [(tail["anchor"], invalid)]))
            invalid = {key: value for key, value in image.items() if key != "bytes"}
            assert not validator.is_valid(insertion(document, [(tail["anchor"], invalid)]))

        preview = await call(client, "preview_edits", **request)
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)
        saved = (await call(client, "open_document", path=output.name))["document"]
        _, inspected, _, _ = (await call(client, "inspect_document", document=saved,
                                         view={"kind": "body-inlines"}))["body_inlines"]
        assert (inspected["kind"], inspected["tooltip"], inspected["history_xml"]) == (
            "hyperlink", TOOLTIP if rich else None, "0" if rich else None,
        )

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() <= after.keys()
    assert {name for name in before if before[name] != after[name]} == {
        "[Content_Types].xml", "word/document.xml", "word/_rels/document.xml.rels",
    }
    readback = docx.Document(output)
    paragraph = readback.paragraphs[0]
    assert paragraph.runs[0].text == "Original"
    (element,) = paragraph._p.xpath("./w:hyperlink")
    assert readback.part.rels[element.get(qn("r:id"))].target_ref == URL
    assert element.xpath("./w:r/w:tab")
    assert element.xpath("./w:r/w:br/@w:type") == ["column" if rich else "textWrapping"]
    name = SOURCE_NAME if rich else "image.png"
    assert paragraph._p.xpath(".//pic:cNvPr/@name") == [name]
    assert paragraph._p.xpath(".//wp:docPr/@name") == [name]
    (embed,) = paragraph._p.xpath(".//a:blip/@r:embed")
    image = readback.part.related_parts[embed]
    assert image.content_type == "image/png" and image.blob == PNG.read_bytes()
    if rich:
        assert element.xpath("./w:r[w:tab]/w:rPr/w:b/@w:val") == ["0"]
        assert element.xpath("./w:r[w:br]/w:rPr/w:i/@w:val") == ["1"]
        assert paragraph._p.xpath("./w:r[w:drawing]/w:rPr/w:strike/@w:val") == ["1"]
        assert paragraph._p.xpath(".//wp:extent/@cx") == [str(24 * 12700)]
        assert paragraph._p.xpath(".//wp:docPr/@descr") == ["A logo"]
    else:
        assert not paragraph._p.xpath(".//w:rPr")
    validate(source, output)
