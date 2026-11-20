"""Character styles share run formatting without computing effective properties."""
import io
import pathlib
from collections.abc import Callable

import docx
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_character_styles_support_forward_bases_and_direct_overrides() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.TextRun("styled", style="Child Character", bold=False),
    ])])], styles=[
        d.CharacterStyleDefinition(name="Child Character", base_style="Later Character", italic=True,
            language=d.RunLanguage(bidi="ar-SA"), bold_complex_script=True, size_complex_script=14.5),
        d.CharacterStyleDefinition(name="Later Character", bold=True, font="Arial"),
    ]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    style = output.styles["Child Character"]
    assert style.base_style.name == "Later Character"
    assert style.font.italic is True
    assert style.font.cs_bold is True
    assert style._element.find(f"{W}rPr/{W}szCs").get(f"{W}val") == "29"
    run = output.paragraphs[0].runs[0]
    assert run.style.name == "Child Character"
    assert run.bold is False
    assert run.italic is None  # no computed style formatting is materialized
    assert run.text == "styled"


@pytest.mark.parametrize("style", ["Normal", "Normal Table", "Missing Character"])
async def test_run_style_must_target_an_existing_character_style(style: str) -> None:
    with pytest.raises((ValueError, KeyError)):
        await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("bad", style=style)])])]).to_bytes()


@pytest.mark.parametrize("styles", [
    lambda: [d.CharacterStyleDefinition(name="A", base_style="B"), d.CharacterStyleDefinition(name="B", base_style="A")],
    lambda: [d.CharacterStyleDefinition(name="A", base_style="Normal")],
])
async def test_bad_character_inheritance_fails(styles: Callable[[], list[object]]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([], styles=styles()).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_run_style_can_use_a_generic_deferred_reference() -> None:
    source = d.TextRun("source", style=lambda: "Emphasis")
    target = d.TextRun("target", style=source.deferred("style"))
    snapshot = await d.Document([d.Section(children=[d.Paragraph(children=[target])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert docx.Document(io.BytesIO(data)).paragraphs[0].runs[0].style.name == "Emphasis"


async def test_existing_character_styles_update_without_changing_their_base(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "characters.docx"
    template.write_bytes(await d.Document([], styles=[d.CharacterStyleDefinition(name="Existing", base_style="Emphasis", bold=True)]).to_bytes())
    data = await d.Document([], styles=[d.CharacterStyleDefinition(name="Existing", base_style="Not consulted", italic=True)]).to_bytes(d.DocumentTemplate(template))
    style = docx.Document(io.BytesIO(data)).styles["Existing"]
    assert style.base_style.name == "Emphasis"
    assert style.font.bold is True
    assert style.font.italic is True


@pytest.mark.parametrize("value", [-1, float("nan"), float("inf"), 1e300])
async def test_invalid_character_style_font_size_fails(value: float) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([], styles=[d.CharacterStyleDefinition(name="Bad", size=value)]).to_bytes()
    assert error.value.rule == "invalid_measurement"
