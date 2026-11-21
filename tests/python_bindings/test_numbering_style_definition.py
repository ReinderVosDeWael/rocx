"""Numbering style associations link an authored concrete and abstract list."""
import io

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_numbering_style_association_uses_concrete_id_and_abstract_link():
    data = await d.Document([], styles=[d.NumberingStyleDefinition("Outline Numbering", lambda: 80)],
        numbering=[d.NumberingDefinition(80, [d.NumberingLevel()], abstract_id=100)],
    ).to_bytes()
    output = docx.Document(io.BytesIO(data))
    style = next(node for node in output.styles.element
                 if node.get(f"{W}styleId") == "OutlineNumbering")
    assert style.get(f"{W}type") == "numbering"
    assert style.find(f"{W}pPr/{W}numPr/{W}numId").get(f"{W}val") == "80"
    root = output.part.numbering_part.element
    abstract = next(node for node in root if node.get(f"{W}abstractNumId") == "100")
    assert abstract.find(f"{W}styleLink").get(f"{W}val") == "OutlineNumbering"


@pytest.mark.parametrize("target", [0, 81])
async def test_numbering_style_requires_an_authored_nonzero_concrete_id(target):
    with pytest.raises(ValueError, match="unique nonzero authored concrete"):
        await d.Document([], styles=[d.NumberingStyleDefinition("Outline Numbering", target)],
            numbering=[d.NumberingDefinition(80, [d.NumberingLevel()])],
        ).to_bytes()


async def test_shared_abstract_numbering_rejects_two_style_owners():
    levels = [d.NumberingLevel()]
    with pytest.raises(ValueError, match="only one numbering style"):
        await d.Document([], styles=[
            d.NumberingStyleDefinition("First Outline", 80),
            d.NumberingStyleDefinition("Second Outline", 81),
        ], numbering=[
            d.NumberingDefinition(80, levels, abstract_id=100),
            d.NumberingDefinition(81, levels, abstract_id=100),
        ]).to_bytes()


async def test_numbering_style_inheritance_resolves_forward_to_local_levels():
    data = await d.Document([], styles=[d.NumberingStyleDefinition("Outline Numbering", 80)],
        numbering=[
            d.NumberingDefinition(81, inherit_style=lambda: "outline numbering", abstract_id=101),
            d.NumberingDefinition(80, [d.NumberingLevel()], abstract_id=100),
        ],
    ).to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    alias = next(node for node in root if node.get(f"{W}abstractNumId") == "101")
    assert alias.find(f"{W}numStyleLink").get(f"{W}val") == "OutlineNumbering"
    assert alias.find(f"{W}lvl") is None


async def test_numbering_style_cannot_own_its_inheriting_definition():
    with pytest.raises(ValueError, match="with local levels"):
        await d.Document([], styles=[d.NumberingStyleDefinition("Loop", 80)],
            numbering=[d.NumberingDefinition(80, inherit_style="Loop")],
        ).to_bytes()


async def test_inherited_numbering_rejects_local_levels():
    with pytest.raises(ValueError, match="cannot contain levels or overrides"):
        await d.Document([], numbering=[
            d.NumberingDefinition(80, [d.NumberingLevel()], inherit_style="Outline"),
        ]).to_bytes()
