"""Level restart requests do not compute or rewrite list counters."""
import io

import docx
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_restart_preserves_omitted_zero_and_one_based_requests() -> None:
    values = [None, 0, 1, 2, 4294967295]
    snapshot = await d.Document([d.Section([d.Paragraph(text="not a calculated counter", numbering=d.NumberingReference(80, 2))])],
        numbering=[d.NumberingDefinition(80,
            levels=[d.NumberingLevel(level=i, restart=value) for i, value in enumerate(values)],
            level_overrides=[d.NumberingLevel(level=i, restart=value) for i, value in enumerate(reversed(values))],
            start_overrides=[d.NumberingStartOverride(2, 7)])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    root = output.part.numbering_part.element
    num = next(n for n in root if n.get(f"{W}numId") == "80")
    abstract_id = num.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in root if n.get(f"{W}abstractNumId") == abstract_id)
    for nodes, expected in [(abstract.findall(f"{W}lvl"), values),
        (num.findall(f"{W}lvlOverride/{W}lvl"), list(reversed(values)))]:
        assert [None if (n := level.find(f"{W}lvlRestart")) is None else int(n.get(f"{W}val")) for level in nodes] == expected
        assert [level.find(f"{W}start").get(f"{W}val") for level in nodes] == ["1"] * 5
    assert num.find(f"{W}lvlOverride[@{W}ilvl='2']/{W}startOverride").get(f"{W}val") == "7"
    assert output.paragraphs[0].text == "not a calculated counter"


async def test_restart_fields_use_generic_nested_deferred_inputs() -> None:
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(restart=lambda: 0)],
        level_overrides=[d.NumberingLevel(restart=lambda: 1)])]).to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    assert [n.get(f"{W}val") for n in root.iter(f"{W}lvlRestart")] == ["0", "1"]


@pytest.mark.parametrize("value", [-1, 4294967296])
@pytest.mark.parametrize("replacement", [False, True])
async def test_restart_rejects_values_outside_the_authoring_range(value: int, replacement: bool) -> None:
    definition = d.NumberingDefinition(80, [d.NumberingLevel()])
    if replacement:
        definition.level_overrides = [d.NumberingLevel(restart=value)]
    else:
        definition.levels = [d.NumberingLevel(restart=value)]
    with pytest.raises((ValueError, OverflowError)):
        await d.Document([], numbering=[definition]).to_bytes()
