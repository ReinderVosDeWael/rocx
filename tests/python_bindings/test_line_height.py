"""Typed line heights use points or multiples without conflating their units."""
import io
import pathlib
import zipfile

import docx
from docx.enum import text as docx_text
from lxml import etree
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_line_height_encodings_and_independent_readback() -> None:
    heights = [d.LineHeight.multiple(1.5), d.LineHeight.exact(14.25), d.LineHeight.at_least(18)]
    document = d.Document([d.Section(children=[
        d.Paragraph(text="absent"), d.Paragraph(text="legacy", line_spacing=1.5),
        *[d.Paragraph(text="typed", line_height=height, spacing_before=3, spacing_after=4) for height in heights],
    ])])
    data = await document.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    paragraphs = root.findall(f".//{W}body/{W}p")
    assert paragraphs[0].find(f"{W}pPr/{W}spacing") is None
    for paragraph, (line, rule) in zip(paragraphs[1:], [("360", "auto"), ("360", "auto"), ("285", "exact"), ("360", "atLeast")], strict=True):
        spacing = paragraph.find(f"{W}pPr/{W}spacing")
        assert spacing.get(f"{W}line") == line
        assert spacing.get(f"{W}lineRule") == rule
    spacing = paragraphs[3].find(f"{W}pPr/{W}spacing")
    assert spacing.get(f"{W}before") == "60"
    assert spacing.get(f"{W}after") == "80"
    output = docx.Document(io.BytesIO(data))
    assert output.paragraphs[2].paragraph_format.line_spacing == 1.5
    assert output.paragraphs[3].paragraph_format.line_spacing.pt == 14.25
    assert output.paragraphs[3].paragraph_format.line_spacing_rule == docx_text.WD_LINE_SPACING.EXACTLY
    assert output.paragraphs[4].paragraph_format.line_spacing.pt == 18
    assert output.paragraphs[4].paragraph_format.line_spacing_rule == docx_text.WD_LINE_SPACING.AT_LEAST


async def test_line_heights_in_styles_and_deferred_paragraph_properties() -> None:
    source = d.Paragraph(text="source", line_height=lambda: d.LineHeight.exact(12))
    declaration = d.Document([d.Section(children=[
        d.Paragraph(text="inherited", style="Measured"),
        d.Paragraph(text="override", style="Measured", line_height=source.deferred("line_height")),
    ])], styles=[d.ParagraphStyleDefinition(name="Measured", line_height=d.LineHeight.at_least(20), spacing_after=5)])
    snapshot = await declaration.resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.styles["Measured"].paragraph_format.line_spacing.pt == 20
    assert output.styles["Measured"].paragraph_format.space_after.pt == 5
    assert output.paragraphs[0].paragraph_format.line_spacing is None
    assert output.paragraphs[1].paragraph_format.line_spacing.pt == 12


@pytest.mark.parametrize("kind", ["multiple", "exact", "at_least"])
@pytest.mark.parametrize("value", [-1, float("nan"), float("inf"), 1e300])
@pytest.mark.parametrize("style", [False, True])
async def test_invalid_line_heights_fail(kind: str, value: float, style: bool) -> None:
    height = getattr(d.LineHeight, kind)(value)
    declaration = d.Document([d.Section(children=[d.Paragraph(text="invalid", **({} if style else {"line_height": height}))])],
        styles=[d.ParagraphStyleDefinition(name="Invalid", line_height=height)] if style else [])
    with pytest.raises(ValueError) as error:
        await declaration.to_bytes()
    assert error.value.rule == "invalid_measurement"
    assert "line_height" in error.value.location


@pytest.mark.parametrize("style", [False, True])
async def test_legacy_and_typed_line_height_conflict_is_explicit(style: bool) -> None:
    properties = {"line_spacing": 1.5, "line_height": d.LineHeight.exact(18)}
    declaration = d.Document([d.Section(children=[d.Paragraph(text="conflict", **({} if style else properties))])],
        styles=[d.ParagraphStyleDefinition(name="Conflict", **properties)] if style else [])
    with pytest.raises(ValueError, match="line_spacing") as error:
        await declaration.to_bytes()
    assert error.value.rule == "invalid_content"


async def test_unknown_line_height_rule_is_rejected() -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(text="bad", line_height=d.LineHeight("unknown", 12))])]).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_line_height_rounding_zero_and_hidden_deferred_values() -> None:
    hidden = d.Paragraph(text="hidden", line_height=lambda: (_ for _ in ()).throw(AssertionError("hidden producer ran")))
    hidden.condition = False
    document = d.Document([d.Section(children=[hidden,
        d.Paragraph(text="zero", line_height=d.LineHeight.exact(0)),
        d.Paragraph(text="rounded", line_height=d.LineHeight.at_least(12.126)),
        d.Paragraph(text="fraction", line_height=d.LineHeight.multiple(1.153)),
    ])])
    data = await document.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert [n.get(f"{W}line") for n in root.iter(f"{W}spacing")] == ["0", "243", "277"]


@pytest.mark.parametrize("legacy", [False, True])
async def test_style_line_height_updates_preserve_other_template_spacing(tmp_path: pathlib.Path, legacy: bool) -> None:
    template = tmp_path / "spacing.docx"
    template.write_bytes(await d.Document([], styles=[d.ParagraphStyleDefinition(
        name="Measured", spacing_before=3, spacing_after=4, line_height=d.LineHeight.at_least(20),
    )]).to_bytes())
    properties = {"line_spacing": 1.5} if legacy else {"line_height": d.LineHeight.exact(12)}
    document = d.Document([], styles=[d.ParagraphStyleDefinition(name="Measured", **properties)])
    output = docx.Document(io.BytesIO(await document.to_bytes(d.DocumentTemplate(template))))
    format = output.styles["Measured"].paragraph_format
    assert format.space_before.pt == 3
    assert format.space_after.pt == 4
    if legacy:
        assert format.line_spacing == 1.5
        assert format.line_spacing_rule == docx_text.WD_LINE_SPACING.ONE_POINT_FIVE
    else:
        assert format.line_spacing.pt == 12
        assert format.line_spacing_rule == docx_text.WD_LINE_SPACING.EXACTLY
