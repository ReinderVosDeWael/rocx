"""Numbering suffix requests stay in numbering XML, not paragraph text."""
import io
import xml.etree.ElementTree as ET
import zipfile

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_suffix_tokens_and_omission_survive_abstract_and_replacement_levels() -> None:
    tokens = [None, "tab", "space", "nothing"]
    snapshot = await d.Document([d.Section([d.Paragraph(text="unchanged", numbering=d.NumberingReference(80))])],
        numbering=[d.NumberingDefinition(80,
            levels=[d.NumberingLevel(level=i, suffix=token) for i, token in enumerate(tokens)],
            level_overrides=[d.NumberingLevel(level=i, suffix=token) for i, token in enumerate(reversed(tokens))])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    root = output.part.numbering_part.element
    abstract = root.findall(f"{W}abstractNum/{W}lvl")[-4:]
    replacements = root.findall(f"{W}num[@{W}numId='80']/{W}lvlOverride/{W}lvl")
    for nodes, expected in [(abstract, tokens), (replacements, list(reversed(tokens)))]:
        assert [None if (n := level.find(f"{W}suff")) is None else n.get(f"{W}val") for level in nodes] == expected
    assert output.paragraphs[0].text == "unchanged"


async def test_suffix_uses_generic_nested_deferred_values() -> None:
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(suffix=lambda: "space")],
        level_overrides=[d.NumberingLevel(suffix=lambda: "nothing")])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = ET.fromstring(package.read("word/numbering.xml"))
    assert [n.get(f"{W}val") for n in root.iter(f"{W}suff")] == ["space", "nothing"]


@pytest.mark.parametrize("token", ["", "none", "SPACE", "invalid"])
@pytest.mark.parametrize("replacement", [False, True])
async def test_invalid_suffix_tokens_are_rejected(token: str, replacement: bool) -> None:
    definition = d.NumberingDefinition(80, levels=[d.NumberingLevel()])
    if replacement:
        definition.level_overrides = [d.NumberingLevel(suffix=token)]
    else:
        definition.levels = [d.NumberingLevel(suffix=token)]
    with pytest.raises(ValueError, match="numbering suffix"):
        await d.Document([], numbering=[definition]).to_bytes()
