"""Table structure edits must preserve proof ranges spanning physical cells."""

import io

import docx
import pytest
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

from rocx.editor import ImportedDocument, TableStructureEdit as Edit


@pytest.mark.parametrize("kind", ["spell", "gram"])
def test_cross_cell_proof_range(kind):
    source = docx.Document()
    source.add_table(rows=1, cols=1)
    protected = source.add_table(rows=2, cols=3)
    for col, text in enumerate(["a", "proofed", "z"]):
        protected.cell(0, col).text = text
    for col, suffix in [(0, "Start"), (2, "End")]:
        marker = OxmlElement("w:proofErr")
        marker.set(qn("w:type"), kind + suffix)
        protected.cell(0, col).paragraphs[0]._p.append(marker)
    source.add_table(rows=1, cols=1)
    buf = io.BytesIO()
    source.save(buf)
    imported = ImportedDocument.open(buf.getvalue())
    before, target, after = [table.anchor for table in imported.body]
    for op in [
        Edit.replace_cell(target, 0, 1, []),
        Edit.insert_cell(target, 0, 1),
        Edit.remove_cell(target, 0, 1),
        Edit.insert_column(target, 1),
        Edit.remove_column(target, 1),
        Edit.insert_row(target, 0),
        Edit.remove_row(target, 0),
    ]:
        with pytest.raises(ValueError, match="table intersects protected range"):
            imported.preview([], table_structure=[op])

    output = imported.preview(
        [], table_structure=[Edit.insert_row(before, 1), Edit.insert_row(after, 1)]
    ).to_docx()
    first, middle, last = docx.Document(io.BytesIO(output)).tables
    assert len(first.rows) == len(last.rows) == 2
    assert [c.text for c in middle.rows[0].cells] == ["a", "proofed", "z"]
    for col, suffix in [(0, "Start"), (2, "End")]:
        marker, = middle.cell(0, col)._tc.iter(qn("w:proofErr"))
        assert marker.get(qn("w:type")) == kind + suffix
