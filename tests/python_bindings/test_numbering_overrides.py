"""Concrete numbering starts are serialized requests, not evaluated labels."""
import io
from collections.abc import Callable

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_concrete_numbering_start_overrides_keep_the_abstract_start() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(text="restart", numbering=d.NumberingReference(42))])],
        numbering=[d.NumberingDefinition(42, [d.NumberingLevel(start=1)], start_overrides=[d.NumberingStartOverride(0, 7)])]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    numbering = output.part.numbering_part.element
    concrete = next(n for n in numbering if n.get(f"{W}numId") == "42")
    assert concrete.find(f"{W}lvlOverride").get(f"{W}ilvl") == "0"
    assert concrete.find(f"{W}lvlOverride/{W}startOverride").get(f"{W}val") == "7"
    abstract_id = concrete.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in numbering if n.get(f"{W}abstractNumId") == abstract_id)
    assert abstract.find(f"{W}lvl/{W}start").get(f"{W}val") == "1"
    assert output.paragraphs[0].text == "restart"


@pytest.mark.parametrize("definition", [
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], start_overrides=[d.NumberingStartOverride(1, 7)]),
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], start_overrides=[d.NumberingStartOverride(0, 7), d.NumberingStartOverride(0, 9)]),
    lambda: d.NumberingDefinition(42, [d.NumberingLevel(), d.NumberingLevel()]),
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], start_overrides=[d.NumberingStartOverride(9, 1)]),
])
async def test_ambiguous_or_undefined_override_levels_fail(definition: Callable[[], d.NumberingDefinition]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([], numbering=[definition()]).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_start_override_fields_use_generic_deferred_resolution() -> None:
    definition = d.NumberingDefinition(42, levels=lambda: [d.NumberingLevel()],
        start_overrides=lambda: [d.NumberingStartOverride(level=lambda: 0, start=lambda: 7)])
    snapshot = await d.Document([], numbering=[definition]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert docx.Document(io.BytesIO(data)).part.numbering_part.element.find(f".//{W}startOverride").get(f"{W}val") == "7"


async def test_overrides_sort_by_level_and_support_zero_start() -> None:
    data = await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel(level=0), d.NumberingLevel(level=1)],
        start_overrides=[d.NumberingStartOverride(1, 2), d.NumberingStartOverride(0, 0)])]).to_bytes()
    numbering = docx.Document(io.BytesIO(data)).part.numbering_part.element
    num = next(n for n in numbering if n.get(f"{W}numId") == "42")
    assert [(node.get(f"{W}ilvl"), node.find(f"{W}startOverride").get(f"{W}val")) for node in num.findall(f"{W}lvlOverride")] == [("0", "0"), ("1", "2")]
