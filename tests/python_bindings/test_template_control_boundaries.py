"""Ordinary replacements must not modify text enclosed by content controls."""

import io
import zipfile

import pytest
from docx import Document
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from lxml import etree

from rocx import declarative as d


W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def control(element):
    parent = element.getparent()
    index = parent.index(element)
    parent.remove(element)
    sdt = OxmlElement("w:sdt")
    properties = OxmlElement("w:sdtPr")
    tag = OxmlElement("w:tag")
    tag.set(qn("w:val"), "target")
    properties.append(tag)
    properties.append(OxmlElement("w:text"))
    content = OxmlElement("w:sdtContent")
    content.append(element)
    sdt.extend([properties, content])
    parent.insert(index, sdt)


@pytest.fixture
def source(tmp_path, request):
    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.add_paragraph("before outside")
    if request.param == "block":
        control(doc.add_paragraph("protected")._p)
    else:
        table = doc.add_table(rows=1, cols=1)
        table.cell(0, 0).text = "protected"
        control(table.rows[0]._tr if request.param == "row" else table.cell(0, 0)._tc)
    doc.add_paragraph("after outside")
    path = tmp_path / f"{request.param}.docx"
    doc.save(path)
    # Independent reader confirms the structure remains readable.
    assert [p.text for p in Document(path).paragraphs] == ["before outside", "after outside"]
    return path


@pytest.mark.parametrize("source", ["block", "row", "cell"], indirect=True)
async def test_rejects_ordinary_replacement_inside_control(source):
    with pytest.raises(ValueError, match="content control"):
        await d.Document([]).to_bytes(
            d.DocumentTemplate(source, replacements={"protected": "changed"})
        )


@pytest.mark.parametrize("source", ["block", "row", "cell"], indirect=True)
async def test_unrelated_text_remains_editable_and_control_untouched(source):
    result = await d.Document([]).to_bytes(
        d.DocumentTemplate(source, replacements={"outside": "edited"})
    )
    assert [p.text for p in Document(io.BytesIO(result)).paragraphs] == [
        "before edited", "after edited"
    ]
    with zipfile.ZipFile(source) as before, zipfile.ZipFile(io.BytesIO(result)) as after:
        old = etree.fromstring(before.read("word/document.xml"))
        new = etree.fromstring(after.read("word/document.xml"))
        original_control = etree.tostring(
            old.find(f".//{W}sdt"), method="c14n", exclusive=True
        )
        edited_control = etree.tostring(
            new.find(f".//{W}sdt"), method="c14n", exclusive=True
        )
        assert original_control == edited_control
        # The template renderer regenerates these package metadata parts.
        regenerated = {
            "[Content_Types].xml", "docProps/core.xml",
            "word/document.xml", "word/styles.xml",
        }
        for name in before.namelist():
            if name not in regenerated:
                assert before.read(name) == after.read(name)


@pytest.mark.parametrize("source", ["block"], indirect=True)
async def test_block_control_remains_editable_by_tag(source):
    result = await d.Document([]).to_bytes(
        d.DocumentTemplate(source, content_control_replacements={"target": "changed"})
    )
    with zipfile.ZipFile(io.BytesIO(result)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{W}sdtContent/{W}p/{W}r/{W}t").text == "changed"
