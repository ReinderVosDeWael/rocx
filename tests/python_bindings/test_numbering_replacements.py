"""Concrete level replacements never mutate abstract numbering resources."""
import io
import pathlib
from collections.abc import Callable

import docx
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_replacement_and_start_override_share_one_sorted_concrete_level() -> None:
    snapshot = await d.Document([d.Section([d.Paragraph(text="not a calculated label", numbering=d.NumberingReference(42))])],
        numbering=[d.NumberingDefinition(42, [d.NumberingLevel(0, 1, "decimal", "%1."), d.NumberingLevel(1, 2, "decimal", "%2.")],
            start_overrides=[d.NumberingStartOverride(0, 7)],
            level_overrides=[d.NumberingLevel(1, 0, "bullet", "•"), d.NumberingLevel(0, 4, "upperRoman", "(%1)")])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    root = output.part.numbering_part.element
    num = next(n for n in root if n.get(f"{W}numId") == "42")
    overrides = num.findall(f"{W}lvlOverride")
    assert [n.get(f"{W}ilvl") for n in overrides] == ["0", "1"]
    assert [n.tag for n in overrides[0]] == [f"{W}startOverride", f"{W}lvl"]
    assert overrides[0].find(f"{W}startOverride").get(f"{W}val") == "7"
    assert overrides[0].find(f"{W}lvl").get(f"{W}ilvl") == "0"
    assert overrides[0].find(f"{W}lvl/{W}start").get(f"{W}val") == "4"
    assert overrides[0].find(f"{W}lvl/{W}numFmt").get(f"{W}val") == "upperRoman"
    assert overrides[0].find(f"{W}lvl/{W}lvlText").get(f"{W}val") == "(%1)"
    assert overrides[1].find(f"{W}startOverride") is None
    assert overrides[1].find(f"{W}lvl/{W}start").get(f"{W}val") == "0"
    abstract_id = num.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in root if n.get(f"{W}abstractNumId") == abstract_id)
    assert [(n.get(f"{W}ilvl"), n.find(f"{W}start").get(f"{W}val"), n.find(f"{W}numFmt").get(f"{W}val")) for n in abstract.findall(f"{W}lvl")] == [("0", "1", "decimal"), ("1", "2", "decimal")]
    assert output.paragraphs[0].text == "not a calculated label"


@pytest.mark.parametrize("definition", [
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], level_overrides=[d.NumberingLevel(1)]),
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], level_overrides=[d.NumberingLevel(), d.NumberingLevel()]),
    lambda: d.NumberingDefinition(42, [d.NumberingLevel()], level_overrides=[d.NumberingLevel(9)]),
    lambda: d.NumberingDefinition(42, [], level_overrides=[d.NumberingLevel()]),
])
async def test_undefined_duplicate_or_out_of_range_replacement_targets_fail(definition: Callable[[], d.NumberingDefinition]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([], numbering=[definition()]).to_bytes()
    assert error.value.rule == "invalid_content"
    assert "level_overrides" in str(error.value)


async def test_replacement_fields_support_generic_nested_deferred_values() -> None:
    snapshot = await d.Document([], numbering=lambda: [d.NumberingDefinition(42, [d.NumberingLevel()],
        level_overrides=lambda: [d.NumberingLevel(level=lambda: 0, start=lambda: 0, format=lambda: "bullet", text=lambda: "★")])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    num = next(n for n in root if n.get(f"{W}numId") == "42")
    assert num.find(f"{W}lvlOverride/{W}lvl/{W}lvlText").get(f"{W}val") == "★"


async def test_mixed_start_only_replacement_only_and_combined_overrides_form_a_union() -> None:
    data = await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel(i) for i in range(3)],
        start_overrides=[d.NumberingStartOverride(2, 0), d.NumberingStartOverride(0, 7)],
        level_overrides=[d.NumberingLevel(1, 3, "lowerLetter", "%2)"), d.NumberingLevel(2, 4, "decimal", "%3)")])]).to_bytes()
    num = next(n for n in docx.Document(io.BytesIO(data)).part.numbering_part.element if n.get(f"{W}numId") == "42")
    assert [[node.tag for node in override] for override in num.findall(f"{W}lvlOverride")] == [
        [f"{W}startOverride"], [f"{W}lvl"], [f"{W}startOverride", f"{W}lvl"]]


async def test_replacements_do_not_bypass_existing_template_numbering_id_collisions(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "numbering.docx"
    template.write_bytes(await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel()])]).to_bytes())
    with pytest.raises(ValueError, match="numbering ID already exists"):
        await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel()],
            level_overrides=[d.NumberingLevel(0, 4, "upperRoman", "(%1)")])]).to_bytes(d.DocumentTemplate(template))
