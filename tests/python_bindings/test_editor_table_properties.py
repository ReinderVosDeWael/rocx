"""Direct field codec, physical coordinates, destructive guards and retry."""

import io

import docx
import pytest
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

from rocx import declarative as d
from rocx.editor import CellAlignmentEdit, ImportedDocument, TableStructureEdit as Edit


def snapshot(document):
    buf = io.BytesIO()
    document.save(buf)
    return ImportedDocument.open(buf.getvalue())


async def test_independent_properties_and_clear_preserve_siblings():
    original = docx.Document()
    table = original.add_table(rows=1, cols=2)
    table.cell(0, 1).text = "unmodified"
    table.cell(0, 1).add_table(rows=1, cols=1).cell(0, 0).text = "nested"
    doc = snapshot(original)
    source = await d.Table([d.TableRow([d.TableCell([d.Paragraph(text="NOT copied")],
        background_color=(1, 2, 3), vertical_alignment="bottom", text_direction="tbRl",
        margins=d.CellMargins(top=4.5, end=8), borders=[d.CellBorder("end", 8, (1, 2, 3), "dashed")])],
        height=d.RowHeight.exact(16), cant_split=True, repeat_header=False)],
        alignment="center", column_widths=[1200], style="Table Grid",
        look=d.TableLook(first_row=False), borders=[d.TableBorder("insideH", 8, (1, 2, 3))]).resolve()
    groups = [
        ({}, {"alignment": {"val": "center"}, "style": {"val":"TableGrid"},
              "look.firstRow":{"firstRow":"0", "maskBit":"0"}, "border.insideH":{"val":"single","sz":"8","color":"010203"}}),
        ({"row":0},{"height":{"val":"320","hRule":"exact"}, "cantSplit":{"val":"1"}, "repeatHeader":{"val":"0"}}),
        ({"row":0,"cell":0},{"verticalAlignment":{"val":"bottom"}, "textDirection":{"val":"tbRl"}, "backgroundColor":{"fill":"010203"},
            "margin.top":{"w":"90","type":"dxa"}, "margin.end":{"w":"160","type":"dxa"}, "border.end":{"val":"dashed","sz":"8","color":"010203"}}),
    ]
    target = doc.body[0].anchor
    patch = [Edit.patch_property(target, k, source, **coord) for coord, fields in groups for k in fields]
    patch.append(Edit.patch_property(target, "width", source, column=1))
    with pytest.raises(ValueError, match="duplicate or conflicting"):
        doc.preview([], table_structure=patch, cell_alignments=[CellAlignmentEdit(target, 0, 0, "top")])
    data = doc.preview([], table_structure=patch).to_docx()
    assert doc.preview([], table_structure=patch[::-1]).to_docx() == data
    view = ImportedDocument.open(data)
    t = view.body[0]
    assert t.grid_widths == [4320, 1200]
    for obj, (_, fields) in zip([t, t.children[0], t.children[0].children[0]], groups, strict=True):
        for k, v in fields.items():
            assert obj.properties[k] == v
    clear = [Edit.patch_property(t.anchor, k, None, **coord) for coord, fields in groups for k in fields if k!="margin.end"]
    data = view.preview([], table_structure=clear).to_docx()
    t = ImportedDocument.open(data).body[0]
    for obj, (_, fields) in zip([t, t.children[0], t.children[0].children[0]], groups, strict=True):
        assert not (set(obj.properties) & (fields.keys() - {"margin.end"}))
    assert t.children[0].children[0].properties["margin.end"]["w"] == "160"
    restored = docx.Document(io.BytesIO(data)).tables[0]
    assert restored.cell(0,0).text == ""
    assert restored.cell(0,1).tables[0].cell(0,0).text == "nested"


async def test_extension_on_border_survives_set_and_guards_clear_and_destruction():
    original = docx.Document()
    table = original.add_table(rows=2, cols=2)
    borders = OxmlElement("w:tcBorders")
    top = OxmlElement("w:top")
    mark = "{urn:template-extension}mark"
    top.set(qn("w:val"), "single")
    top.set(mark, "keep")
    borders.append(top)
    table.cell(0,1)._tc.get_or_add_tcPr().append(borders)
    doc = snapshot(original)
    a = doc.body[0].anchor
    donor = await d.Table([d.TableRow([d.TableCell([], borders=[d.CellBorder("top", 8, (1,2,3))])])]).resolve()
    for edit in [Edit.remove_table(a), Edit.remove_row(a,0), Edit.remove_cell(a,0,1), Edit.remove_column(a,1),
                 Edit.replace_node(a,donor), Edit.replace_node(a,donor,row=0,cell=1),
                 Edit.patch_property(a,"border.top",None,row=0,cell=1)]:
        with pytest.raises(ValueError, match="unsupported"):
            doc.preview([], table_structure=[edit])
    data = doc.preview([], table_structure=[Edit.patch_property(a,"border.top",donor,row=0,cell=1)]).to_docx()
    leaf, = docx.Document(io.BytesIO(data)).tables[0].cell(0,1)._tc.xpath("w:tcPr/w:tcBorders/w:top")
    assert (leaf.get(mark), leaf.get(qn("w:color"))) == ("keep","010203")


async def test_invalid_span_or_vertical_chain_rejects_entire_batch():
    original = docx.Document()
    t = original.add_table(rows=2,cols=2)
    t.cell(1,0).text = "visible"
    doc = snapshot(original)
    a = doc.body[0].anchor
    wide = await d.Table([d.TableRow([d.TableCell([],grid_span=2)])]).resolve()
    merged = await d.Table([d.TableRow([d.TableCell([],vmerge="continue")])]).resolve()
    nested_bad = await d.Table([d.TableRow([d.TableCell([
        d.Table([d.TableRow([d.TableCell([], vmerge="continue")])])])])]).resolve()
    for op in [Edit.patch_property(a,"gridSpan",wide,row=0,cell=0),
               Edit.patch_property(a,"verticalMerge",merged,row=0,cell=0),
               Edit.patch_property(a,"verticalMerge",merged,row=1,cell=0),
               Edit.replace_node(a,wide,row=0,cell=0), Edit.replace_node(a,nested_bad)]:
        with pytest.raises(ValueError, match="grid|merge|hide"):
            doc.preview([], table_structure=[op])
    assert doc.body[0].children[0].children[0].grid_span is None
