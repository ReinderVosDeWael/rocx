"""Numbering level references resolve registered paragraph styles by name or ID."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_level_style_links_on_abstract_and_replacement_levels():
    data = await d.Document([], styles=[d.ParagraphStyleDefinition("List Caption")],
        numbering=[d.NumberingDefinition(80, [
            d.NumberingLevel(paragraph_style=lambda: "list caption"),
        ], level_overrides=[
            d.NumberingLevel(paragraph_style="Heading2"),
        ], abstract_id=100)],
    ).to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    abstract = next(node for node in root if node.get(f"{W}abstractNumId") == "100")
    instance = next(node for node in root if node.get(f"{W}numId") == "80")
    assert abstract.find(f"{W}lvl/{W}pStyle").get(f"{W}val") == "ListCaption"
    assert instance.find(f"{W}lvlOverride/{W}lvl/{W}pStyle").get(f"{W}val") == "Heading2"


@pytest.mark.parametrize("name, error, message", [
    ("Missing style", KeyError, "unknown style"),
    ("Strong", ValueError, "must be a paragraph style"),
])
async def test_level_style_links_require_registered_paragraph_styles(name, error, message):
    with pytest.raises(error, match=message):
        await d.Document([], numbering=[
            d.NumberingDefinition(80, [d.NumberingLevel(paragraph_style=name)]),
        ]).to_bytes()
