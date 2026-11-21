"""Section line numbering is an independent, mergeable section request."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_line_numbering_preserves_omission_and_partial_section_updates():
    def section(line):
        return d.Section([d.Paragraph(text="Line numbering")],
                         properties=d.SectionProperties(line_numbering=line))

    snapshot = await d.Document([
        section(None),
        section(d.LineNumbering(count_by=lambda: 5, start=0, distance=240, restart="newPage")),
        section(d.LineNumbering(restart="newSection")),
        section(d.LineNumbering(restart="continuous")),
        section(None),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    sections = docx.Document(io.BytesIO(data)).sections
    assert sections[0]._sectPr.find(f"{W}lnNumType") is None
    for sect, mode in zip(list(sections)[1:], ["newPage", "newSection", "continuous", "continuous"]):
        node = sect._sectPr.find(f"{W}lnNumType")
        assert {key: node.get(f"{W}{key}") for key in ("countBy", "start", "distance", "restart")} == {
            "countBy": "5", "start": "0", "distance": "240", "restart": mode,
        }


@pytest.mark.parametrize("field, value, message", [
    ("count_by", 0, "count_by"),
    ("count_by", 101, "count_by"),
    ("start", 32768, "start"),
    ("restart", "invalid", "line numbering restart"),
])
async def test_invalid_line_numbering_is_rejected(field, value, message):
    props = d.SectionProperties(line_numbering=d.LineNumbering(**{field: value}))
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(text="Line numbering")], properties=props)]).to_bytes()


async def test_imported_line_numbering_is_preserved_when_editing_restart(tmp_path):
    original = d.SectionProperties(line_numbering=d.LineNumbering(5, 0, 240, "newPage"))
    update = d.SectionProperties(line_numbering=d.LineNumbering(restart="continuous"))
    template = tmp_path / "numbered.docx"
    await d.Document([d.Section([], properties=original)]).save(template)
    data = await d.Document([d.Section([], properties=update)]).to_bytes(d.DocumentTemplate(template))
    section = docx.Document(io.BytesIO(data)).sections[-1]
    node = section._sectPr.find(f"{W}lnNumType")
    assert node.get(f"{W}countBy") == "5"
    assert node.get(f"{W}start") == "0"
    assert node.get(f"{W}distance") == "240"
    assert node.get(f"{W}restart") == "continuous"


async def test_paragraph_suppression_preserves_omission_and_explicit_false():
    data = await d.Document([
        d.Section([
            d.Paragraph(text="omitted"),
            d.Paragraph(text="on", suppress_line_numbers=lambda: True),
            d.Paragraph(text="off", suppress_line_numbers=False, style="Suppressed"),
        ], properties=d.SectionProperties(line_numbering=d.LineNumbering(1))),
    ], styles=[d.ParagraphStyleDefinition("Suppressed", suppress_line_numbers=True)],
        defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(suppress_line_numbers=False)),
    ).to_bytes()
    output = docx.Document(io.BytesIO(data))
    for paragraph, value in zip(output.paragraphs, [None, "1", "0"]):
        node = paragraph._p.find(f"{W}pPr/{W}suppressLineNumbers")
        assert (node.get(f"{W}val") if node is not None else None) == value
    styles = output.styles.element
    authored = next(node for node in styles if node.get(f"{W}styleId") == "Suppressed")
    assert authored.find(f"{W}pPr/{W}suppressLineNumbers").get(f"{W}val") == "1"
    defaults = styles.find(f"{W}docDefaults/{W}pPrDefault/{W}pPr/{W}suppressLineNumbers")
    assert defaults.get(f"{W}val") == "0"


async def test_clear_line_numbering_applies_to_later_sections_and_preserves_page_numbering():
    def section(line, page=None):
        return d.Section([d.Paragraph(text="Line numbering")], properties=d.SectionProperties(
            line_numbering=line, page_numbering=page,
        ))

    data = await d.Document([
        section(d.LineNumbering(count_by=5), {"start": 7, "format": "decimal"}),
        section(d.LineNumbering(clear=True)),
        section(None),
    ]).to_bytes()
    before, cleared, later = docx.Document(io.BytesIO(data)).sections
    assert before._sectPr.find(f"{W}lnNumType") is not None
    for target in (cleared, later):
        assert target._sectPr.find(f"{W}lnNumType") is None
        assert target._sectPr.find(f"{W}pgNumType").get(f"{W}start") == "7"


async def test_clear_line_numbering_rejects_conflicting_settings():
    props = d.SectionProperties(line_numbering=d.LineNumbering(count_by=5, clear=True))
    with pytest.raises(ValueError, match="clearing line numbering cannot be combined"):
        await d.Document([d.Section([], properties=props)]).to_bytes()
