"""Conditional script requests remain in table styles, not direct runs."""
import io

import docx
import pytest

from rocx import declarative as d
from test_conditional_paragraph_formatting import REGIONS

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


@pytest.mark.parametrize("region,wire", REGIONS)
async def test_script_requests_stay_in_the_requested_table_style_region(region: str, wire: str) -> None:
    formatting = d.TableSectionFormat(language=d.RunLanguage("en-US", "ja-JP", "ar-SA"),
        bold_complex_script=False, italic_complex_script=True, size_complex_script=13.5,
        rtl=False, complex_script=True, font_east_asia="East", font_complex_script="Complex")
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Scripts", **{region: formatting})]).to_bytes()
    style = docx.Document(io.BytesIO(data)).styles["Scripts"]._element
    node = style if region == "whole_table" else next(n for n in style.findall(f"{W}tblStylePr") if n.get(f"{W}type") == wire)
    properties = node.find(f"{W}rPr")
    assert dict(properties.find(f"{W}lang").attrib) == {f"{W}val": "en-US", f"{W}eastAsia": "ja-JP", f"{W}bidi": "ar-SA"}
    assert dict(properties.find(f"{W}rFonts").attrib) == {f"{W}eastAsia": "East", f"{W}cs": "Complex"}
    for element, expected in [("bCs", "0"), ("iCs", "1"), ("szCs", "27"), ("rtl", "0"), ("cs", "1")]:
        assert properties.find(f"{W}{element}").get(f"{W}val") == expected


@pytest.mark.parametrize("size", [-1, float("nan"), float("inf"), 1e30])
async def test_invalid_conditional_complex_script_sizes_are_rejected(size: float) -> None:
    with pytest.raises(ValueError):
        await d.Document([], styles=[d.TableStyleDefinition(name="Bad", first_row=d.TableSectionFormat(size_complex_script=size))]).to_bytes()


async def test_nested_deferred_script_fields_do_not_change_direct_run_formatting() -> None:
    snapshot = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([
        d.Paragraph(children=[d.TextRun("first العربية last", rtl=False)])])])], style="Scripts")])],
        styles=lambda: [d.TableStyleDefinition(name="Scripts", first_row=lambda: d.TableSectionFormat(
            language=lambda: d.RunLanguage(bidi=lambda: "ar-SA"), rtl=lambda: True,
            size_complex_script=lambda: 14, font_complex_script=lambda: "Complex"))]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    paragraph = output.tables[0].cell(0, 0).paragraphs[0]
    assert paragraph.text == "first العربية last"
    assert paragraph.runs[0].font.rtl is False
    assert paragraph.runs[0]._r.find(f"{W}rPr/{W}szCs") is None
    properties = output.styles["Scripts"]._element.find(f"{W}tblStylePr/{W}rPr")
    assert properties.find(f"{W}rtl").get(f"{W}val") == "1"
    assert properties.find(f"{W}szCs").get(f"{W}val") == "28"
    assert properties.find(f"{W}lang").get(f"{W}bidi") == "ar-SA"


async def test_omitted_script_fields_do_not_author_script_overrides() -> None:
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Plain", first_row=d.TableSectionFormat(bold=True))]).to_bytes()
    properties = docx.Document(io.BytesIO(data)).styles["Plain"]._element.find(f"{W}tblStylePr/{W}rPr")
    assert [node.tag for node in properties] == [f"{W}b"]


async def test_zero_complex_script_size_is_an_explicit_override() -> None:
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Zero", whole_table=d.TableSectionFormat(size_complex_script=0))]).to_bytes()
    properties = docx.Document(io.BytesIO(data)).styles["Zero"]._element.find(f"{W}rPr")
    assert properties.find(f"{W}szCs").get(f"{W}val") == "0"
