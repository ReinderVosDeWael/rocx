"""Row and cell wrappers retain table geometry and contextual content."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_row_and_cell_controls_preserve_grid_spans_row_order_and_ids():
    control = d.TableRow([
        d.TableCell([d.Paragraph(text="spanning cell")], grid_span=2,
                    content_control=d.ContentControlMetadata(tag="cell", id=102)),
    ], repeat_header=True, content_control=lambda: d.ContentControlMetadata(tag="row", id=101))
    other = d.TableRow([d.TableCell([d.Paragraph(text="left")]), d.TableCell([d.Paragraph(text="right")])])
    data = await d.Document([d.Section([d.Table([control, other], column_widths=[1440, 2880])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    table = root.find(f".//{W}tbl")
    assert [node.tag for node in list(table)[-2:]] == [f"{W}sdt", f"{W}tr"]
    assert [node.get(f"{W}w") for node in table.findall(f"{W}tblGrid/{W}gridCol")] == ["1440", "2880"]
    row_control = table.find(f"{W}sdt")
    assert row_control.find(f"{W}sdtPr/{W}id").get(f"{W}val") == "101"
    row = row_control.find(f"{W}sdtContent/{W}tr")
    assert row.find(f"{W}trPr/{W}tblHeader").get(f"{W}val") == "1"
    cell_control = row.find(f"{W}sdt")
    assert cell_control.find(f"{W}sdtPr/{W}id").get(f"{W}val") == "102"
    cell = cell_control.find(f"{W}sdtContent/{W}tc")
    assert cell.find(f"{W}tcPr/{W}gridSpan").get(f"{W}val") == "2"
    assert cell.find(f"{W}p/{W}r/{W}t").text == "spanning cell"


async def test_table_control_metadata_cannot_be_a_block_control():
    row = d.TableRow([d.TableCell([d.Paragraph(text="text")])],
                     content_control=d.BlockContentControl([d.Paragraph(text="wrong context")]))
    with pytest.raises(TypeError, match="row and cell controls require ContentControlMetadata"):
        await d.Document([d.Section([d.Table([row])])]).to_bytes()
