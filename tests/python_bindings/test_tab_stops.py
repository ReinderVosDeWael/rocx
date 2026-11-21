"""Custom tab-stop authoring, checked positions and consumer-side inheritance."""
import io
import pathlib
import zipfile
from collections.abc import Callable

import docx
from docx.enum import text as docx_text
import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_sorted_signed_tab_stops_and_leaders_have_independent_readback() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("label"), d.Tab(), d.TextRun("value")], tab_stops=[
        d.TabStop(108, "right", "dot"), d.TabStop(-12, "left"), d.TabStop(72, "center", "hyphen"),
        d.TabStop(144, "decimal", "underscore"), d.TabStop(180, "bar"), d.TabStop(216, "clear"),
    ])])]).to_bytes()
    paragraph = docx.Document(io.BytesIO(data)).paragraphs[0]
    assert paragraph.text == "label\tvalue"
    stops = list(paragraph.paragraph_format.tab_stops)
    assert [stop.position.pt for stop in stops] == [-12, 72, 108, 144, 180, 216]
    assert [stop.alignment for stop in stops] == [docx_text.WD_TAB_ALIGNMENT.LEFT, docx_text.WD_TAB_ALIGNMENT.CENTER,
        docx_text.WD_TAB_ALIGNMENT.RIGHT, docx_text.WD_TAB_ALIGNMENT.DECIMAL, docx_text.WD_TAB_ALIGNMENT.BAR, docx_text.WD_TAB_ALIGNMENT.CLEAR]
    assert stops[2].leader == docx_text.WD_TAB_LEADER.DOTS


async def test_style_tabs_and_deferred_clear_overrides_do_not_compute_effective_stops() -> None:
    source = d.Paragraph(text="source", tab_stops=lambda: [d.TabStop(72, "clear"), d.TabStop(108, "right")])
    document = d.Document([d.Section(children=[d.Paragraph(text="inherited", style="Tabbed"),
        d.Paragraph(text="direct", style="Tabbed", tab_stops=source.deferred("tab_stops"))])],
        styles=[d.ParagraphStyleDefinition(name="Tabbed", tab_stops=[d.TabStop(72)])])
    snapshot = await document.resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert len(output.styles["Tabbed"].paragraph_format.tab_stops) == 1
    assert len(output.paragraphs[0].paragraph_format.tab_stops) == 0
    assert output.paragraphs[1].paragraph_format.tab_stops[0].alignment == docx_text.WD_TAB_ALIGNMENT.CLEAR


@pytest.mark.parametrize("value", [float("nan"), float("inf"), 1e300])
@pytest.mark.parametrize("style", [False, True])
async def test_unrepresentable_tab_positions_fail(value: float, style: bool) -> None:
    properties = {"tab_stops": [d.TabStop(value)]}
    document = d.Document([d.Section(children=[d.Paragraph(text="bad", **({} if style else properties))])],
        styles=[d.ParagraphStyleDefinition(name="Bad", **properties)] if style else [])
    with pytest.raises(ValueError) as error:
        await document.to_bytes()
    assert error.value.rule == "invalid_measurement"
    assert "tab_stops[0].position" in error.value.location


@pytest.mark.parametrize("stops", [lambda: [d.TabStop(72), d.TabStop(72.01)],
    lambda: [d.TabStop(0, "unknown")], lambda: [d.TabStop(0, leader="unknown")]])
@pytest.mark.parametrize("style", [False, True])
async def test_duplicate_wire_positions_and_unknown_enum_values_fail(stops: Callable[[], list[d.TabStop]], style: bool) -> None:
    properties = {"tab_stops": stops()}
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(text="bad", **({} if style else properties))])],
            styles=[d.ParagraphStyleDefinition(name="Bad", **properties)] if style else []).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_all_alignment_and_leader_values_round_positions_without_inserting_text() -> None:
    alignments = ["left", "center", "right", "decimal", "bar", "clear", "start", "end"]
    leaders = [None, "none", "dot", "hyphen", "underscore", "heavy", "middle_dot", None]
    data = await d.Document([d.Section(children=[d.Paragraph(text="unchanged", tab_stops=[
        d.TabStop(index * 36 + 0.026, alignment, leader) for index, (alignment, leader) in enumerate(zip(alignments, leaders, strict=True))
    ])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    stops = root.findall(f".//{W}tabs/{W}tab")
    assert [stop.get(f"{W}val") for stop in stops] == alignments
    assert [stop.get(f"{W}leader") for stop in stops] == [None, "none", "dot", "hyphen", "underscore", "heavy", "middleDot", None]
    assert [stop.get(f"{W}pos") for stop in stops] == [str(index * 720 + 1) for index in range(8)]
    assert docx.Document(io.BytesIO(data)).paragraphs[0].text == "unchanged"


async def test_template_style_tab_updates_replace_local_collection_but_keep_other_properties(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "tabs.docx"
    template.write_bytes(await d.Document([], styles=[d.ParagraphStyleDefinition(name="Tabbed",
        tab_stops=[d.TabStop(36), d.TabStop(72)], spacing_after=5, line_height=d.LineHeight.exact(14))]).to_bytes())
    for stops, positions in [([], [36, 72]), ([d.TabStop(108, "right")], [108])]:
        data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Tabbed", tab_stops=stops)]).to_bytes(d.DocumentTemplate(template))
        format = docx.Document(io.BytesIO(data)).styles["Tabbed"].paragraph_format
        assert [stop.position.pt for stop in format.tab_stops] == positions
        assert format.space_after.pt == 5
        assert format.line_spacing.pt == 14


async def test_hidden_tab_stop_producer_is_not_invoked() -> None:
    hidden = d.Paragraph(text="hidden", tab_stops=lambda: (_ for _ in ()).throw(AssertionError("hidden producer ran")))
    hidden.condition = False
    data = await d.Document([d.Section(children=[hidden, d.Paragraph(text="visible")])]).to_bytes()
    assert [paragraph.text for paragraph in docx.Document(io.BytesIO(data)).paragraphs] == ["visible"]
