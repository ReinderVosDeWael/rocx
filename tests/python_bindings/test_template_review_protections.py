"""Template review must honor enclosing ranges and bound cached content."""

from io import BytesIO

import pytest
from docx import Document
from docx.oxml import parse_xml
from docx.oxml.ns import nsdecls, qn

from rocx import declarative as d


def append(paragraph, markup):
    paragraph._p.extend(parse_xml(f'<root {nsdecls("w")}>{markup}</root>'))


def leg(role, rid):
    attrs = 'w:author="A" w:date="2025-01-02T03:04:05Z"'
    return (
        f'<w:{role}RangeStart w:id="{rid}" w:name="Move1" {attrs}/>'
        f'<w:{role} w:id="{rid + 1}" {attrs}><w:r><w:t>moved</w:t></w:r></w:{role}>'
        f'<w:{role}RangeEnd w:id="{rid}"/>'
    )


@pytest.mark.parametrize("action", ["accept", "reject"])
@pytest.mark.parametrize("protected_leg", ["from", "to", None])
async def test_bookmark_across_paragraphs(tmp_path, action, protected_leg):
    doc = Document()
    start = '<w:bookmarkStart w:id="90" w:name="Protected"/>'
    end = '<w:bookmarkEnd w:id="90"/>'
    append(doc.add_paragraph(), {"from": start, "to": "", None: start + end}[protected_leg])
    append(doc.add_paragraph(), leg("moveFrom", 11))
    append(doc.add_paragraph(), {"from": end, "to": start, None: ""}[protected_leg])
    append(doc.add_paragraph(), leg("moveTo", 13))
    if protected_leg == "to":
        append(doc.add_paragraph(), end)
    path = tmp_path / "move.docx"
    doc.save(path)
    review = d.DocumentTemplate(path, text_revision_action=action)
    if protected_leg:
        with pytest.raises(ValueError, match="protected range"):
            await d.Document([]).to_bytes(review)
    else:
        reviewed = Document(BytesIO(await d.Document([]).to_bytes(review)))
        assert [p.text for p in reviewed.paragraphs] == [
            "", "moved" if action == "reject" else "", "", "moved" if action == "accept" else ""
        ]


@pytest.mark.parametrize("binding", ["direct", "alternate", None])
async def test_extension_binding_in_control_properties(tmp_path, binding):
    doc = Document()
    prop = (
        '<ext:dataBinding xmlns:ext="http://schemas.microsoft.com/office/word/2012/wordml" '
        'w:xpath="/bib:Sources" w:storeItemID="{EF278816-EC6F-A645-907D-7F25AECB1D4A}"/>'
    )
    if binding == "alternate":
        prop = (
            '<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
            'xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml">'
            f'<mc:Choice Requires="w15">{prop}</mc:Choice><mc:Fallback/></mc:AlternateContent>'
        )
    append(doc.add_paragraph(), (
        f'<w:sdt><w:sdtPr><w:tag w:val="sample"/>{prop if binding else ""}<w:text/></w:sdtPr>'
        '<w:sdtContent><w:r><w:t>keep</w:t></w:r></w:sdtContent></w:sdt>'
    ))
    path = tmp_path / "control.docx"
    doc.save(path)
    edit = d.DocumentTemplate(path, content_control_replacements={"sample": "changed"})
    if binding:
        with pytest.raises(ValueError, match="bound"):
            await d.Document([]).to_bytes(edit)
    else:
        edited = Document(BytesIO(await d.Document([]).to_bytes(edit)))
        assert edited.paragraphs[0]._p.find(".//" + qn("w:t")).text == "changed"
