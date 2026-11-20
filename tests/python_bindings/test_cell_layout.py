"""Cell-local point margins and text flow do not calculate effective layout."""
import io
import zipfile

import docx
from lxml import etree
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def document(cell: d.TableCell) -> d.Document:
    """Wrap a configured cell in a minimal table."""
    return d.Document([d.Section(children=[d.Table([d.TableRow([cell])])])])


async def test_cell_margins_round_points_preserve_zero_and_keep_omitted_sides_absent() -> None:
    data = await document(d.TableCell([d.Paragraph(text="cell")], margins=d.CellMargins(top=3.126, start=0, bottom=6, end=9), text_direction="tbRl")).to_bytes()
    output = docx.Document(io.BytesIO(data))
    cell = output.tables[0].cell(0, 0)
    properties = cell._tc.find(f"{W}tcPr")
    margins = properties.find(f"{W}tcMar")
    assert [(etree.QName(n).localname, n.get(f"{W}w"), n.get(f"{W}type")) for n in margins] == [
        ("top", "63", "dxa"), ("start", "0", "dxa"), ("bottom", "120", "dxa"), ("end", "180", "dxa")]
    assert properties.find(f"{W}textDirection").get(f"{W}val") == "tbRl"
    assert cell.text == "cell"


@pytest.mark.parametrize("margins", [None, lambda: d.CellMargins(), lambda: d.CellMargins(start=0)])
async def test_omitted_margins_do_not_materialize_defaults(margins: object) -> None:
    value = margins() if callable(margins) else margins
    data = await document(d.TableCell([d.Paragraph(text="cell")], margins=value)).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    node = root.find(f".//{W}tcMar")
    if value is not None and value.start == 0:
        assert len(node) == 1
        assert node[0].get(f"{W}w") == "0"
    else:
        assert node is None
    assert root.find(f".//{W}textDirection") is None


@pytest.mark.parametrize("side", ["top", "start", "bottom", "end"])
@pytest.mark.parametrize("value", [-1, float("nan"), float("inf"), 1e300])
async def test_invalid_cell_margin_measurements_fail_with_the_side(side: str, value: float) -> None:
    with pytest.raises(ValueError) as error:
        await document(d.TableCell([d.Paragraph(text="bad")], margins=d.CellMargins(**{side: value}))).to_bytes()
    assert error.value.rule == "invalid_measurement"
    assert f"margins.{side}" in error.value.location


@pytest.mark.parametrize("direction", ["lrTb", "tbRl", "btLr", "lrTbV", "tbRlV", "tbLrV"])
async def test_each_supported_cell_text_direction_is_retained(direction: str) -> None:
    data = await document(d.TableCell([d.Paragraph(text="unchanged")], text_direction=direction)).to_bytes()
    output = docx.Document(io.BytesIO(data)).tables[0].cell(0, 0)
    assert output._tc.find(f"{W}tcPr/{W}textDirection").get(f"{W}val") == direction
    assert output.text == "unchanged"


async def test_unknown_cell_direction_fails() -> None:
    with pytest.raises(ValueError) as error:
        await document(d.TableCell([d.Paragraph(text="bad")], text_direction="sideways")).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_cell_layout_uses_generic_deferred_fields_and_hidden_guards() -> None:
    source = d.TableCell([], margins=lambda: d.CellMargins(start=6), text_direction=lambda: "btLr")
    target = d.TableCell([d.Paragraph(text="target")], margins=source.deferred("margins"), text_direction=source.deferred("text_direction"))
    hidden = d.TableCell([], margins=lambda: (_ for _ in ()).throw(AssertionError("hidden margins ran")))
    hidden.condition = False
    snapshot = await d.Document([d.Section(children=[d.Table([d.TableRow([target, hidden])])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    cell = docx.Document(io.BytesIO(data)).tables[0].cell(0, 0)
    assert cell._tc.find(f"{W}tcPr/{W}tcMar/{W}start").get(f"{W}w") == "120"
    assert cell._tc.find(f"{W}tcPr/{W}textDirection").get(f"{W}val") == "btLr"
