"""Regression for tab-stop definitions being mistaken for inline tabs."""

import io
from zipfile import ZipFile

from docx import Document
from docx.shared import Inches
from lxml import etree
from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_tab_replacement_preserves_paragraph_tab_stops(tmp_path):
    source = tmp_path / "source.docx"
    fixture = Document()
    paragraph = fixture.add_paragraph("Hello")
    paragraph.paragraph_format.tab_stops.add_tab_stop(Inches(1))
    fixture.add_paragraph("Before\tAfter")
    # python-docx's default template omits this required Transitional attribute.
    fixture.settings.element.find(f"{W}zoom").set(f"{W}percent", "100")
    fixture.save(source)

    result = await d.Document([]).to_bytes(
        d.DocumentTemplate(source, replacements={"\t": "X"})
    )
    with ZipFile(source) as before, ZipFile(io.BytesIO(result)) as after:
        assert set(before.namelist()) == set(after.namelist())
        for name in before.namelist():
            if name.endswith(".rels") or name.startswith("customXml/"):
                assert before.read(name) == after.read(name), name
        original = etree.fromstring(before.read("word/document.xml"))
        edited = etree.fromstring(after.read("word/document.xml"))

    original_tabs = original.find(f".//{W}pPr/{W}tabs")
    edited_tabs = edited.find(f".//{W}pPr/{W}tabs")
    assert [(child.tag, dict(child.attrib)) for child in original_tabs] == [
        (child.tag, dict(child.attrib)) for child in edited_tabs
    ]
    assert edited_tabs.find(f"{W}tab").get(f"{W}pos") == "1440"
    assert not edited.findall(f".//{W}r/{W}tab")
    reopened = Document(io.BytesIO(result))
    assert [p.text for p in reopened.paragraphs] == ["Hello", "BeforeXAfter"]
    assert reopened.paragraphs[0].paragraph_format.tab_stops[0].position == Inches(1)
