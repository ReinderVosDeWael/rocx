"""Explicit linked styles remain independent paragraph and character resources."""
import io
import pathlib
from collections.abc import Callable

import docx
import pytest

from rocx import declarative as d
import test_notes

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_forward_reciprocal_links_keep_direct_references_and_properties_separate() -> None:
    snapshot = await d.Document([d.Section([d.Paragraph(style="Pair Paragraph", children=[
        d.TextRun("linked", style="Pair Character", italic=False)])])], styles=lambda: [
        d.ParagraphStyleDefinition(name="Pair Paragraph", linked_style=lambda: "Pair Character", bold=True),
        d.CharacterStyleDefinition(name="Pair Character", linked_style=lambda: "Pair Paragraph", italic=True),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.styles["Pair Paragraph"]._element.find(f"{W}link").get(f"{W}val") == "PairCharacter"
    assert output.styles["Pair Character"]._element.find(f"{W}link").get(f"{W}val") == "PairParagraph"
    assert output.styles["Pair Paragraph"].font.bold is True
    assert output.styles["Pair Character"].font.bold is None
    assert output.paragraphs[0].style.name == "Pair Paragraph"
    assert output.paragraphs[0].runs[0].style.name == "Pair Character"
    assert output.paragraphs[0].runs[0].italic is False


async def test_one_sided_link_does_not_mutate_target_or_its_formatting() -> None:
    data = await d.Document([], styles=[
        d.ParagraphStyleDefinition(name="Source", linked_style="Target", font="Literal"),
        d.CharacterStyleDefinition(name="Target"),
    ]).to_bytes()
    styles = docx.Document(io.BytesIO(data)).styles
    assert styles["Source"]._element.find(f"{W}link").get(f"{W}val") == "Target"
    assert styles["Target"]._element.find(f"{W}link") is None
    assert styles["Target"].font.name is None


@pytest.mark.parametrize("definitions", [
    lambda: [d.ParagraphStyleDefinition(name="Source", linked_style="Missing")],
    lambda: [d.CharacterStyleDefinition(name="Source", linked_style="Missing")],
    lambda: [d.ParagraphStyleDefinition(name="Source", linked_style="Normal")],
    lambda: [d.CharacterStyleDefinition(name="Source", linked_style="Source")],
    lambda: [d.ParagraphStyleDefinition(name="Source", linked_style="Table"), d.TableStyleDefinition(name="Table")],
])
async def test_invalid_authored_link_targets_fail(definitions: Callable[[], list]) -> None:
    with pytest.raises((ValueError, KeyError), match="style"):
        await d.Document([], styles=definitions()).to_bytes()


async def test_links_update_existing_styles_and_omission_preserves_template_links(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "linked.docx"
    template.write_bytes(await d.Document([], styles=[
        d.ParagraphStyleDefinition(name="Source", linked_style="Old"),
        d.CharacterStyleDefinition(name="Old", linked_style="Source"),
        d.CharacterStyleDefinition(name="New"),
    ]).to_bytes())
    for target, expected in [(None, "Old"), ("New", "New")]:
        data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Source", linked_style=target, italic=True)]).to_bytes(d.DocumentTemplate(template))
        styles = docx.Document(io.BytesIO(data)).styles
        assert styles["Source"]._element.find(f"{W}link").get(f"{W}val") == expected
        assert styles["Old"]._element.find(f"{W}link").get(f"{W}val") == "Source"
        assert styles["New"]._element.find(f"{W}link") is None
    data = await d.Document([], styles=[d.CharacterStyleDefinition(name="Old", linked_style="Normal")]).to_bytes(d.DocumentTemplate(template))
    styles = docx.Document(io.BytesIO(data)).styles
    assert styles["Old"]._element.find(f"{W}link").get(f"{W}val") == "Normal"
    assert styles["Source"]._element.find(f"{W}link").get(f"{W}val") == "Old"


async def test_imported_link_defects_are_preserved_unless_explicitly_overridden(tmp_path: pathlib.Path) -> None:
    original = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Source", linked_style="Target"),
        d.CharacterStyleDefinition(name="Target", linked_style="Source")]).to_bytes()
    def mutate(root) -> None:
        style = next(n for n in root if n.get(f"{W}styleId") == "Source")
        del style.attrib[f"{W}type"]
        style.find(f"{W}link").set(f"{W}val", "Missing")
    template = tmp_path / "imported.docx"
    template.write_bytes(test_notes.rewrite(original, "word/styles.xml", mutate))
    data = await d.Document([], styles=[d.CharacterStyleDefinition(name="Target", linked_style="Source", bold=True)]).to_bytes(d.DocumentTemplate(template))
    styles = docx.Document(io.BytesIO(data)).styles.element
    source = next(n for n in styles if n.get(f"{W}styleId") == "Source")
    target = next(n for n in styles if n.get(f"{W}styleId") == "Target")
    assert source.get(f"{W}type") is None
    assert source.find(f"{W}link").get(f"{W}val") == "Missing"
    assert target.find(f"{W}link").get(f"{W}val") == "Source"


async def test_linked_styles_do_not_relax_direct_reference_kind_checks() -> None:
    definitions = [d.ParagraphStyleDefinition(name="Pair Paragraph", linked_style="Pair Character"),
        d.CharacterStyleDefinition(name="Pair Character", linked_style="Pair Paragraph")]
    with pytest.raises(ValueError):
        await d.Document([d.Section([d.Paragraph(text="wrong", style="Pair Character")])], styles=definitions).to_bytes()
    with pytest.raises(ValueError):
        await d.Document([d.Section([d.Paragraph(children=[d.TextRun("wrong", style="Pair Paragraph")])])], styles=definitions).to_bytes()
