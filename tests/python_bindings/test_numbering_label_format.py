"""Label properties use the same run-property encoding as other authoring contexts."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_deferred_label_format_and_replacement_remain_separate():
    snapshot = await d.Document([], numbering=[
        d.NumberingDefinition(80, [d.NumberingLevel(
            label_format=lambda: d.RunDefaults(font="Calibri", size=11, bold=True),
        )], level_overrides=[d.NumberingLevel(label_format=d.RunDefaults(
            bold=False, italic=True,
        ))], abstract_id=100),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    abstract = next(node for node in root if node.get(f"{W}abstractNumId") == "100")
    concrete = next(node for node in root if node.get(f"{W}numId") == "80")
    base = abstract.find(f"{W}lvl/{W}rPr")
    replacement = concrete.find(f"{W}lvlOverride/{W}lvl/{W}rPr")
    assert base.find(f"{W}b").get(f"{W}val") == "1"
    assert base.find(f"{W}sz").get(f"{W}val") == "22"
    assert base.find(f"{W}rFonts").get(f"{W}ascii") == "Calibri"
    assert replacement.find(f"{W}b").get(f"{W}val") == "0"
    assert replacement.find(f"{W}i").get(f"{W}val") == "1"


@pytest.mark.parametrize("replacement", [False, True])
async def test_invalid_label_size_fails_rendering(replacement):
    invalid = d.NumberingLevel(label_format=d.RunDefaults(size=-1))
    definition = d.NumberingDefinition(
        80, [d.NumberingLevel() if replacement else invalid],
        level_overrides=[invalid] if replacement else [],
    )
    with pytest.raises(ValueError, match=r"label_format\.size"):
        await d.Document([], numbering=[definition]).to_bytes()


async def test_level_paragraph_properties_precede_label_format():
    data = await d.Document([], numbering=[
        d.NumberingDefinition(80, [d.NumberingLevel(
            alignment="left",
            paragraph_format=lambda: d.ParagraphDefaults(
                left_indent=24, first_line_indent=-12,
                tab_stops=[d.TabStop(position=24, alignment="left")],
            ),
            label_format=d.RunDefaults(bold=True),
        )], abstract_id=100, level_overrides=[d.NumberingLevel(
            paragraph_format=d.ParagraphDefaults(left_indent=36),
        )]),
    ]).to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    base = next(node for node in root if node.get(f"{W}abstractNumId") == "100").find(f"{W}lvl")
    override = next(node for node in root if node.get(f"{W}numId") == "80").find(f"{W}lvlOverride/{W}lvl")
    assert [node.tag for node in base][-3:] == [f"{W}lvlJc", f"{W}pPr", f"{W}rPr"]
    assert base.find(f"{W}pPr/{W}tabs/{W}tab").get(f"{W}pos") == "480"
    assert base.find(f"{W}pPr/{W}ind").get(f"{W}hanging") == "240"
    assert override.find(f"{W}pPr/{W}ind").get(f"{W}left") == "720"
    assert override.find(f"{W}rPr") is None


async def test_numbering_level_paragraph_properties_reject_duplicate_tabs():
    invalid = d.NumberingLevel(paragraph_format=d.ParagraphDefaults(tab_stops=[
        d.TabStop(position=24, alignment="left"),
        d.TabStop(position=24, alignment="right"),
    ]))
    with pytest.raises(ValueError, match=r"paragraph_format\.tab_stops"):
        await d.Document([], numbering=[d.NumberingDefinition(80, [invalid])]).to_bytes()
