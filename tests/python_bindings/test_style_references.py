"""Style reference allocation is independent of declaration order."""
import io
import pathlib
from collections.abc import Callable

import docx
import pytest

from rocx import declarative as d


async def test_forward_base_and_next_style_references_resolve() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(text="child", style="Child")])], styles=[
        d.ParagraphStyleDefinition(name="Child", base_style="Later Base", next_paragraph_style="Later Next"),
        d.ParagraphStyleDefinition(name="Later Next", next_paragraph_style="Later Next"),
        d.ParagraphStyleDefinition(name="Later Base", base_style="Normal", bold=True),
    ]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.styles["Child"].base_style.name == "Later Base"
    assert output.styles["Child"].next_paragraph_style.name == "Later Next"
    assert output.styles["Later Next"].next_paragraph_style.name == "Later Next"
    assert output.paragraphs[0].style.name == "Child"


@pytest.mark.parametrize("styles", [
    lambda: [d.ParagraphStyleDefinition(name="Self", base_style="Self")],
    lambda: [d.ParagraphStyleDefinition(name="A", base_style="B"), d.ParagraphStyleDefinition(name="B", base_style="A")],
    lambda: [d.TableStyleDefinition(name="A", base_style="B"), d.TableStyleDefinition(name="B", base_style="A")],
])
async def test_based_on_cycles_fail_explicitly(styles: Callable[[], list[object]]) -> None:
    with pytest.raises(ValueError, match="cycle") as error:
        await d.Document([], styles=styles()).to_bytes()
    assert error.value.rule == "invalid_content"


@pytest.mark.parametrize("styles", [
    lambda: [d.ParagraphStyleDefinition(name="Child", base_style="Normal Table")],
    lambda: [d.TableStyleDefinition(name="Table", base_style="Normal")],
    lambda: [d.ParagraphStyleDefinition(name="Child", next_paragraph_style="Normal Table")],
    lambda: [d.ParagraphStyleDefinition(name="Normal Table")],
])
async def test_wrong_style_reference_kinds_fail(styles: Callable[[], list[object]]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([], styles=styles()).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_existing_paragraph_base_is_unchanged_but_next_is_updated(tmp_path: pathlib.Path) -> None:
    source = d.Document([], styles=[d.ParagraphStyleDefinition(name="Existing", base_style="Normal")])
    template = tmp_path / "styles.docx"
    template.write_bytes(await source.to_bytes())
    data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Existing", base_style="Not consulted",
        next_paragraph_style="Later"), d.ParagraphStyleDefinition(name="Later")]).to_bytes(d.DocumentTemplate(template))
    style = docx.Document(io.BytesIO(data)).styles["Existing"]
    assert style.base_style.name == "Normal"
    assert style.next_paragraph_style.name == "Later"


@pytest.mark.parametrize("reachable", [False, True])
async def test_imported_cycles_are_checked_only_when_reachable_from_authored_styles(tmp_path: pathlib.Path, reachable: bool) -> None:
    import test_notes
    w = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
    # The new writer correctly rejects authoring cycles; build this imported
    # defect by mutating an otherwise valid package instead.
    source = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Imported")]).to_bytes()
    def cycle(root):
        from lxml import etree
        style = next(n for n in root if n.get(f"{w}styleId") == "Imported")
        etree.SubElement(style, f"{w}basedOn", {f"{w}val": "Imported"})
    template = tmp_path / "cycle.docx"
    template.write_bytes(test_notes.rewrite(source, "word/styles.xml", cycle))
    document = d.Document([], styles=[d.ParagraphStyleDefinition(name="Child", base_style="Imported")] if reachable else [])
    if reachable:
        with pytest.raises(ValueError, match="cycle"):
            await document.to_bytes(d.DocumentTemplate(template))
    else:
        assert await document.to_bytes(d.DocumentTemplate(template))


async def test_imported_style_with_omitted_type_defaults_to_paragraph(tmp_path: pathlib.Path) -> None:
    import test_notes
    w = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
    source = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Implicit")]).to_bytes()
    def omit_type(root):
        style = next(n for n in root if n.get(f"{w}styleId") == "Implicit")
        del style.attrib[f"{w}type"]
    template = tmp_path / "implicit.docx"
    template.write_bytes(test_notes.rewrite(source, "word/styles.xml", omit_type))
    data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Implicit", bold=True),
        d.ParagraphStyleDefinition(name="Child", base_style="Implicit")]).to_bytes(d.DocumentTemplate(template))
    output = docx.Document(io.BytesIO(data))
    # python-docx's StyleFactory does not implement the normative omitted-type
    # paragraph default. Inspect its loaded XML rather than altering the import.
    child = next(n for n in output.styles.element if n.get(f"{w}styleId") == "Child")
    assert child.find(f"{w}basedOn").get(f"{w}val") == "Implicit"
