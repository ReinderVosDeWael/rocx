"""Physical merge readback and non-destructive direct layout edits."""

import io
import zipfile

import docx
import pytest
from docx.oxml.ns import qn

from rocx.editor import ImportedDocument, TableStructureEdit


def test_layout_and_physical_merges():
    source = docx.Document()
    table = source.add_table(rows=2, cols=3)
    table.cell(0, 0).merge(table.cell(1, 1)).text = "merged"
    table.autofit = False
    source_bytes = io.BytesIO()
    source.save(source_bytes)

    doc = ImportedDocument.open(source_bytes.getvalue())
    target, = doc.body
    assert target.layout == "fixed"
    assert target.grid_widths == [2880] * 3
    for row, marker in zip(target.children, ["restart", "continue"], strict=True):
        wide, plain = row.children
        assert (wide.grid_span, wide.vertical_merge) == (2, marker)
        assert (plain.grid_span, plain.vertical_merge) == (None, None)
    edit = TableStructureEdit.layout(target.anchor, None)
    with pytest.raises(ValueError, match="exclusive"):
        doc.preview([], table_structure=[edit, edit])
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(source_bytes.getvalue()).preview([], table_structure=[edit])
    data = doc.preview([], table_structure=[edit]).to_docx()
    assert ImportedDocument.open(data).body[0].layout is None
    adjusted = docx.Document(io.BytesIO(data)).tables[0]
    assert adjusted._tbl.tblPr.find(qn("w:tblLayout")) is None
    assert adjusted.cell(1, 1).text == "merged"
    with zipfile.ZipFile(source_bytes) as before, zipfile.ZipFile(io.BytesIO(data)) as after:
        assert before.namelist() and set(before.namelist()) == set(after.namelist())
        assert [n for n in before.namelist() if before.read(n) != after.read(n)] == ["word/document.xml"]


def test_layout_clear_rejects_opaque_property_but_set_preserves_it():
    source = docx.Document()
    table = source.add_table(rows=1, cols=1)
    table.autofit = False
    key = "{urn:template-extension}mark"
    table._tbl.tblPr.find(qn("w:tblLayout")).set(key, "preserve")
    data = io.BytesIO()
    source.save(data)
    doc = ImportedDocument.open(data.getvalue())
    anchor = doc.body[0].anchor
    with pytest.raises(ValueError, match="discard unsupported markup"):
        doc.preview([], table_structure=[TableStructureEdit.layout(anchor, None)])
    patched = doc.preview([], table_structure=[TableStructureEdit.layout(anchor, "autofit")])
    readback = docx.Document(io.BytesIO(patched.to_docx())).tables[0]
    assert readback.autofit
    assert readback._tbl.tblPr.find(qn("w:tblLayout")).get(key) == "preserve"
