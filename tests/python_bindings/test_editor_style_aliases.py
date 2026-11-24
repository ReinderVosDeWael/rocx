import io
from zipfile import ZipFile

import pytest
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

from rocx import declarative as d
from rocx.editor import ImportedDocument, StyleEdit


def alias_fixture(second_alias=None):
    doc = Document()
    for name, value in [("Old Style", "OtherName, Memo Style"), ("Second", second_alias)]:
        if value is not None:
            style = doc.styles.add_style(name, WD_STYLE_TYPE.PARAGRAPH)
            aliases = OxmlElement("w:aliases")
            aliases.set(qn("w:val"), value)
            style.element.append(aliases)
    doc.add_paragraph("body", style="Old Style")
    field = OxmlElement("w:fldSimple")
    field.set(qn("w:instr"), ' STYLEREF "Memo Style" ')
    doc.add_paragraph()._p.append(field)
    data = io.BytesIO()
    doc.save(data)
    return data.getvalue()


def add(name):
    return StyleEdit.add(d.ParagraphStyleDefinition(name=name))


def test_styleref_alias_is_reserved_for_its_owner():
    source = alias_fixture()
    imported = ImportedDocument.open(source)
    for collision in ("mEMo Style", "Other Name"):
        with pytest.raises(ValueError, match="duplicate"):
            imported.preview([], style_edits=[add(collision)])
    output = imported.preview([], style_edits=[add("Fresh Style")]).to_docx()
    styles = Document(io.BytesIO(output)).styles
    assert styles["Fresh Style"].style_id == "FreshStyle"
    assert styles["Old Style"].element.find(qn("w:aliases")).get(qn("w:val")) == "OtherName, Memo Style"
    with ZipFile(io.BytesIO(source)) as before, ZipFile(io.BytesIO(output)) as after:
        assert before.read("word/document.xml") == after.read("word/document.xml")


@pytest.mark.parametrize("alias", ["memo style", "OLD STYLE", "oldstyle"])
def test_ambiguous_imported_alias_rejects_style_edit(alias):
    imported = ImportedDocument.open(alias_fixture(alias))
    with pytest.raises(ValueError, match="ambiguous"):
        imported.preview([], style_edits=[add("Fresh Style")])
