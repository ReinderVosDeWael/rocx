"""Opaque run children must not be transparent to ordinary replacement."""

import io
from zipfile import ZipFile

import pytest
from docx import Document
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from lxml import etree

from rocx import declarative as d
from rocx.editor import ImportedDocument


@pytest.fixture
def source(tmp_path):
    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    run = doc.add_paragraph().add_run()
    run.bold = True
    run.add_text("AB")
    run._r.append(OxmlElement("w:noBreakHyphen"))
    run.add_text("CD")
    path = tmp_path / "source.docx"
    doc.save(path)
    assert Document(path).paragraphs[0].text == "AB-CD"
    return path


async def test_nonbreaking_hyphen_blocks_crossing_replacement(source):
    with pytest.raises(ValueError, match="crosses a structured boundary"):
        await d.Document([]).to_bytes(d.DocumentTemplate(source, replacements={"ABCD": "X"}))


async def test_nonbreaking_hyphen_survives_adjacent_replacements(source):
    result = await d.Document([]).to_bytes(
        d.DocumentTemplate(source, replacements={"AB": "Y", "CD": "Z"})
    )
    paragraph = Document(io.BytesIO(result)).paragraphs[0]
    assert paragraph.text == "Y-Z"
    assert paragraph.runs[0].bold is True
    with ZipFile(source) as before, ZipFile(io.BytesIO(result)) as after:
        original = etree.fromstring(before.read("word/document.xml"))
        edited = etree.fromstring(after.read("word/document.xml"))
        original_hyphen, = original.findall(f".//{qn('w:r')}/{qn('w:noBreakHyphen')}")
        edited_hyphen, = edited.findall(f".//{qn('w:r')}/{qn('w:noBreakHyphen')}")
        assert etree.tostring(original_hyphen, method="c14n", exclusive=True) == (
            etree.tostring(edited_hyphen, method="c14n", exclusive=True)
        )

    imported = ImportedDocument.open(result)
    run, = imported.body[0].inline_structure
    assert [(item.kind, item.value) for item in run.items] == [
        ("text", "Y"),
        ("opaque", "w:noBreakHyphen"),
        ("text", "Z"),
    ]
