"""Document grid settings are section-local requests with attribute merging."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_document_grid_merges_deferred_character_delta(tmp_path):
    template = tmp_path / "section-grid.docx"
    original = d.SectionProperties(document_grid=d.DocumentGrid("linesAndChars", 360, 4096))
    await d.Document([d.Section([], properties=original)]).save(template)
    update = d.SectionProperties(document_grid=d.DocumentGrid(character_pitch_delta=lambda: -4096))
    data = await d.Document([d.Section([], properties=update)]).to_bytes(d.DocumentTemplate(template))
    grid = docx.Document(io.BytesIO(data)).sections[-1]._sectPr.find(f"{W}docGrid")
    assert grid.get(f"{W}type") == "linesAndChars"
    assert grid.get(f"{W}linePitch") == "360"
    assert grid.get(f"{W}charSpace") == "-4096"


@pytest.mark.parametrize("grid, message", [
    (d.DocumentGrid(mode="invalid"), "unknown document grid mode"),
    (d.DocumentGrid(line_pitch=0), r"document_grid\.line_pitch"),
])
async def test_invalid_document_grid_arguments_are_rejected(grid, message):
    props = d.SectionProperties(document_grid=grid)
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([], properties=props)]).to_bytes()


async def test_grid_snapping_preserves_direct_styles_and_defaults():
    data = await d.Document([
        d.Section([
            d.Paragraph(text="omitted"),
            d.Paragraph(style="Grid Paragraph", snap_to_grid=lambda: False,
                children=[d.TextRun("explicit", style="Grid Character", snap_to_grid=True)]),
        ], properties=d.SectionProperties(document_grid=d.DocumentGrid("snapToChars"))),
    ], styles=[
        d.ParagraphStyleDefinition("Grid Paragraph", snap_to_grid=True),
        d.CharacterStyleDefinition("Grid Character", snap_to_grid=False),
    ], defaults=d.DocumentDefaults(
        paragraph=d.ParagraphDefaults(snap_to_grid=False),
        run=d.RunDefaults(snap_to_grid=True),
    )).to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.paragraphs[0]._p.find(f"{W}pPr/{W}snapToGrid") is None
    direct = output.paragraphs[1]
    assert direct._p.find(f"{W}pPr/{W}snapToGrid").get(f"{W}val") == "0"
    assert direct.runs[0]._r.find(f"{W}rPr/{W}snapToGrid").get(f"{W}val") == "1"
    styles = output.styles.element
    paragraph_style = next(node for node in styles if node.get(f"{W}styleId") == "GridParagraph")
    character_style = next(node for node in styles if node.get(f"{W}styleId") == "GridCharacter")
    assert paragraph_style.find(f"{W}pPr/{W}snapToGrid").get(f"{W}val") == "1"
    assert character_style.find(f"{W}rPr/{W}snapToGrid").get(f"{W}val") == "0"
    defaults = styles.find(f"{W}docDefaults")
    assert defaults.find(f"{W}pPrDefault/{W}pPr/{W}snapToGrid").get(f"{W}val") == "0"
    assert defaults.find(f"{W}rPrDefault/{W}rPr/{W}snapToGrid").get(f"{W}val") == "1"
