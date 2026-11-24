"""Physical table paths stay within their story, entry, and safe block boundaries."""

import copy
import io
from zipfile import ZipFile

import docx
import pytest
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.opc.packuri import PackURI
from docx.opc.part import Part
from docx.opc.constants import RELATIONSHIP_TYPE as RT
from lxml import etree

from rocx.editor import ImportedDocument, TableStructureEdit as Edit


def snapshot(doc):
    buf = io.BytesIO()
    doc.save(buf)
    return buf.getvalue(), ImportedDocument.open(buf.getvalue())


@pytest.mark.parametrize("kind", ["footnote", "endnote"])
def test_nested_table_in_one_relocated_ordinary_entry(kind):
    original = docx.Document()
    table = original.add_table(rows=1, cols=1)
    table.cell(0,0).add_table(1,1).cell(0,0).text = "nested note"
    root = OxmlElement(f"w:{kind}s")
    for id_, type_ in [("-1","separator"), ("0","continuationSeparator"), ("1",None), ("2",None), ("02",None)]:
        entry = OxmlElement(f"w:{kind}")
        entry.set(qn("w:id"),id_)
        if type_:
            entry.set(qn("w:type"),type_)
        entry.append(copy.deepcopy(table._tbl))
        root.append(entry)
    owner = f"custom/{kind}s.xml"
    part = Part(PackURI("/"+owner),
                f"application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}s+xml",
                etree.tostring(root),original.part.package)
    original.part.relate_to(part, getattr(RT,kind.upper()+"S"))
    before, doc = snapshot(original)
    parent, nested = doc.part_tables(owner)
    assert [t.entry_id for t in (parent,nested)] == ["1","1"]
    assert nested.depth == 1 and nested.path[:len(parent.path)] == parent.path
    edit = Edit.insert_row(nested.anchor,1)
    with pytest.raises(ValueError, match="overlapping"):
        doc.edit_part_tables(owner,[Edit.layout(parent.anchor,None),edit])
    with pytest.raises(ValueError, match="owner mismatch"):
        doc.edit_part_tables(doc.part_tables()[0].part_name,[edit])
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(before).edit_part_tables(owner,[edit])
    with pytest.raises(ValueError, match="physical table anchor"):
        doc.preview([],table_structure=[edit])
    out = doc.edit_part_tables(owner,[edit])
    with ZipFile(io.BytesIO(before)) as old, ZipFile(io.BytesIO(out)) as new:
        assert set(old.namelist()) == set(new.namelist())
        assert {n for n in old.namelist() if old.read(n) != new.read(n)} == {owner}
        root = etree.fromstring(new.read(owner))
        assert [len(e.findall(".//"+qn("w:tr"))) for e in root] == [2,2,3,2,2]
    assert docx.Document(io.BytesIO(out)).tables[0].cell(0,0).tables[0].cell(0,0).text == "nested note"


def test_range_crossing_outer_table_protects_nested_target():
    source = docx.Document()
    p = source.add_paragraph()._p
    start = OxmlElement("w:bookmarkStart")
    start.set(qn("w:id"),"17")
    start.set(qn("w:name"),"protected")
    p.append(start)
    source.add_table(1,1).cell(0,0).add_table(1,1)
    end = OxmlElement("w:bookmarkEnd")
    end.set(qn("w:id"),"17")
    source.add_paragraph()._p.append(end)
    source.add_table(1,1)
    _,doc = snapshot(source)
    parent,nested,after = doc.part_tables()
    with pytest.raises(ValueError,match="protected"):
        doc.edit_part_tables(parent.part_name,[Edit.insert_row(after.anchor,1),Edit.insert_row(nested.anchor,1)])
    out = doc.edit_part_tables(after.part_name,[Edit.insert_row(after.anchor,1)])
    assert [len(t.rows) for t in docx.Document(io.BytesIO(out)).tables] == [1,2]
