"""Table row heights and pagination flags are requests, not computed layout."""
import io
import zipfile

import docx
from docx.enum import table as docx_table
from lxml import etree
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def row(**properties: object) -> d.TableRow:
    """Create a one-cell row for pagination property checks."""
    return d.TableRow([d.TableCell([d.Paragraph(text="cell")])], **properties)


async def test_row_height_units_flags_and_independent_readback() -> None:
    data = await d.Document([d.Section(children=[d.Table(rows=[
        row(height=d.RowHeight.exact(14.25), repeat_header=True, cant_split=True),
        row(height=d.RowHeight.at_least(18), repeat_header=False, cant_split=False),
        row(),
    ])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    rows = root.findall(f".//{W}tr")
    assert rows[0].find(f"{W}trPr/{W}trHeight").attrib == {f"{W}val": "285", f"{W}hRule": "exact"}
    assert rows[1].find(f"{W}trPr/{W}trHeight").attrib == {f"{W}val": "360", f"{W}hRule": "atLeast"}
    for index, flag in enumerate(["1", "0"]):
        assert rows[index].find(f"{W}trPr/{W}tblHeader").get(f"{W}val") == flag
        assert rows[index].find(f"{W}trPr/{W}cantSplit").get(f"{W}val") == flag
    assert rows[2].find(f"{W}trPr") is None
    output = docx.Document(io.BytesIO(data)).tables[0].rows
    assert output[0].height.pt == 14.25
    assert output[0].height_rule == docx_table.WD_ROW_HEIGHT_RULE.EXACTLY
    assert output[1].height.pt == 18
    assert output[1].height_rule == docx_table.WD_ROW_HEIGHT_RULE.AT_LEAST
    assert output[2].height is None


@pytest.mark.parametrize("kind", ["exact", "at_least"])
@pytest.mark.parametrize("value", [-1, float("inf"), float("nan"), 1e300])
async def test_invalid_row_heights_fail(kind: str, value: float) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Table(rows=[row(height=getattr(d.RowHeight, kind)(value))])])]).to_bytes()
    assert error.value.rule == "invalid_measurement"
    assert "height" in error.value.location


async def test_deferred_row_properties_and_hidden_rows_use_generic_resolution() -> None:
    source = row(height=lambda: d.RowHeight.at_least(12.126), repeat_header=lambda: True, cant_split=lambda: False)
    shared = row(height=source.deferred("height"), repeat_header=source.deferred("repeat_header"), cant_split=source.deferred("cant_split"))
    hidden = row(height=lambda: (_ for _ in ()).throw(AssertionError("hidden producer ran")))
    hidden.condition = False
    snapshot = await d.Document([d.Section(children=[d.Table(rows=[shared, hidden])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data)).tables[0].rows
    assert len(output) == 1
    assert output[0].height.pt == 12.15


async def test_unknown_row_height_rule_fails() -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Table(rows=[row(height=d.RowHeight("unknown", 12))])])]).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_zero_height_and_nonleading_header_requests_are_retained_without_layout_claims() -> None:
    data = await d.Document([d.Section(children=[d.Table(rows=[row(),
        row(height=d.RowHeight.exact(0), repeat_header=True),
    ])])]).to_bytes()
    output = docx.Document(io.BytesIO(data)).tables[0].rows
    assert output[1].height.pt == 0
    # The consumer ignores this nonleading header request; the writer does not
    # reorder rows, synthesize missing leading flags or calculate page breaks.
    assert output[0]._tr.find(f"{W}trPr") is None
    assert output[1]._tr.find(f"{W}trPr/{W}tblHeader").get(f"{W}val") == "1"
