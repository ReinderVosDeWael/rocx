"""Conditional table paragraph properties reuse checked paragraph formatting."""
import io
from collections.abc import Callable

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
REGIONS = [
    ("whole_table", "wholeTable"), ("first_row", "firstRow"), ("last_row", "lastRow"),
    ("first_column", "firstCol"), ("last_column", "lastCol"),
    ("banding_1_row", "band1Horz"), ("banding_2_row", "band2Horz"),
    ("banding_1_column", "band1Vert"), ("banding_2_column", "band2Vert"),
    ("top_left_cell", "nwCell"), ("top_right_cell", "neCell"),
    ("bottom_left_cell", "swCell"), ("bottom_right_cell", "seCell"),
]


@pytest.mark.parametrize("region,wire", REGIONS)
async def test_conditional_paragraph_properties_remain_in_the_requested_region(region: str, wire: str) -> None:
    formatting = d.TableSectionFormat(line_height=d.LineHeight.exact(14.5),
        tab_stops=[d.TabStop(72, "right", "dot"), d.TabStop(-12, "clear")], bidi=False)
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Conditional", **{region: formatting})]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    style = output.styles["Conditional"]._element
    node = style if region == "whole_table" else next(n for n in style.findall(f"{W}tblStylePr") if n.get(f"{W}type") == wire)
    properties = node.find(f"{W}pPr")
    assert properties.find(f"{W}spacing").get(f"{W}line") == "290"
    assert properties.find(f"{W}spacing").get(f"{W}lineRule") == "exact"
    assert properties.find(f"{W}bidi").get(f"{W}val") == "0"
    assert [(n.get(f"{W}pos"), n.get(f"{W}val")) for n in properties.find(f"{W}tabs")] == [("-240", "clear"), ("1440", "right")]


@pytest.mark.parametrize("formatting", [
    lambda: d.TableSectionFormat(line_height=d.LineHeight.exact(-1)),
    lambda: d.TableSectionFormat(line_height=d.LineHeight.multiple(float("inf"))),
    lambda: d.TableSectionFormat(tab_stops=[d.TabStop(1), d.TabStop(1.001)]),
])
async def test_invalid_conditional_paragraph_measurements_fail(formatting: Callable[[], d.TableSectionFormat]) -> None:
    with pytest.raises(ValueError):
        await d.Document([], styles=[d.TableStyleDefinition(name="Bad", first_row=formatting())]).to_bytes()


async def test_nested_conditional_paragraph_fields_support_generic_deferred_values() -> None:
    snapshot = await d.Document([], styles=lambda: [d.TableStyleDefinition(name="Deferred",
        first_row=lambda: d.TableSectionFormat(bidi=lambda: True,
            line_height=lambda: d.LineHeight.at_least(12), tab_stops=lambda: [d.TabStop(36)]))]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    properties = docx.Document(io.BytesIO(data)).styles["Deferred"]._element.find(f"{W}tblStylePr/{W}pPr")
    assert properties.find(f"{W}bidi").get(f"{W}val") == "1"
    assert properties.find(f"{W}spacing").get(f"{W}lineRule") == "atLeast"


async def test_conditional_properties_do_not_materialize_direct_cell_formatting() -> None:
    data = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="unchanged")])])], style="Conditional")])],
        styles=[d.TableStyleDefinition(name="Conditional", first_row=d.TableSectionFormat(
            line_height=d.LineHeight.multiple(1.5), bidi=True))]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    paragraph = output.tables[0].cell(0, 0).paragraphs[0]
    assert paragraph.text == "unchanged"
    assert paragraph._p.find(f"{W}pPr/{W}bidi") is None
    assert paragraph._p.find(f"{W}pPr/{W}spacing") is None
    properties = output.styles["Conditional"]._element.find(f"{W}tblStylePr/{W}pPr")
    assert properties.find(f"{W}spacing").get(f"{W}line") == "360"
