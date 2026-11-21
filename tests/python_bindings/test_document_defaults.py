"""Document defaults merge explicitly without materializing effective styles."""
import io
import pathlib
from collections.abc import Callable

import docx
import pytest

from rocx import declarative as d
import test_notes

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_defaults_are_document_properties_not_direct_formatting() -> None:
    snapshot = await d.Document([d.Section([d.Paragraph(children=[d.TextRun("unchanged", bold=False)])])],
        defaults=lambda: d.DocumentDefaults(
            run=lambda: d.RunDefaults(font="Literal", size=13.5, bold=lambda: True,
                rtl=False, language=d.RunLanguage(bidi=lambda: "ar-SA")),
            paragraph=lambda: d.ParagraphDefaults(spacing_before=0, line_height=d.LineHeight.exact(14),
                left_indent=12, first_line_indent=-6, bidi=True, tab_stops=[d.TabStop(36)]))).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    defaults = output.styles.element.find(f"{W}docDefaults")
    run = defaults.find(f"{W}rPrDefault/{W}rPr")
    assert run.find(f"{W}rFonts").get(f"{W}ascii") == "Literal"
    assert run.find(f"{W}rFonts").get(f"{W}asciiTheme") is None
    assert run.find(f"{W}b").get(f"{W}val") == "1"
    assert run.find(f"{W}sz").get(f"{W}val") == "27"
    assert run.find(f"{W}lang").get(f"{W}bidi") == "ar-SA"
    paragraph = defaults.find(f"{W}pPrDefault/{W}pPr")
    assert paragraph.find(f"{W}spacing").get(f"{W}before") == "0"
    assert paragraph.find(f"{W}spacing").get(f"{W}line") == "280"
    assert paragraph.find(f"{W}ind").get(f"{W}left") == "240"
    assert paragraph.find(f"{W}ind").get(f"{W}hanging") == "120"
    assert paragraph.find(f"{W}tabs/{W}tab").get(f"{W}pos") == "720"
    assert output.paragraphs[0].runs[0].font.bold is False
    assert output.paragraphs[0].paragraph_format.left_indent is None


async def test_defaults_merge_template_attributes_and_keep_unspecified_properties(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "defaults.docx"
    template.write_bytes(await d.Document([], defaults=d.DocumentDefaults(
        run=d.RunDefaults(font_east_asia="East", bold=True, size=12, language=d.RunLanguage("en-US", "ja-JP")),
        paragraph=d.ParagraphDefaults(spacing_before=4, spacing_after=8, first_line_indent=-6,
            tab_stops=[d.TabStop(72)]))).to_bytes())
    for defaults in [None, d.DocumentDefaults(), d.DocumentDefaults(run=d.RunDefaults(), paragraph=d.ParagraphDefaults())]:
        data = await d.Document([], defaults=defaults).to_bytes(d.DocumentTemplate(template))
        node = docx.Document(io.BytesIO(data)).styles.element.find(f"{W}docDefaults")
        assert node.find(f"{W}rPrDefault/{W}rPr/{W}b").get(f"{W}val") == "1"
        assert node.find(f"{W}pPrDefault/{W}pPr/{W}spacing").get(f"{W}before") == "80"
    data = await d.Document([], defaults=d.DocumentDefaults(run=d.RunDefaults(bold=False, size=0,
        language=d.RunLanguage(bidi="ar-SA")), paragraph=d.ParagraphDefaults(spacing_before=0,
        first_line_indent=0))).to_bytes(d.DocumentTemplate(template))
    node = docx.Document(io.BytesIO(data)).styles.element.find(f"{W}docDefaults")
    run = node.find(f"{W}rPrDefault/{W}rPr")
    assert run.find(f"{W}b").get(f"{W}val") == "0"
    assert run.find(f"{W}sz").get(f"{W}val") == "0"
    assert run.find(f"{W}rFonts").get(f"{W}eastAsia") == "East"
    assert dict(run.find(f"{W}lang").attrib) == {f"{W}val": "en-US", f"{W}eastAsia": "ja-JP", f"{W}bidi": "ar-SA"}
    paragraph = node.find(f"{W}pPrDefault/{W}pPr")
    assert paragraph.find(f"{W}spacing").get(f"{W}before") == "0"
    assert paragraph.find(f"{W}spacing").get(f"{W}after") == "160"
    assert paragraph.find(f"{W}ind").get(f"{W}firstLine") == "0"
    assert paragraph.find(f"{W}ind").get(f"{W}hanging") is None
    assert paragraph.find(f"{W}tabs/{W}tab").get(f"{W}pos") == "1440"
    replaced = await d.Document([], defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(
        tab_stops=[d.TabStop(18)]))).to_bytes(d.DocumentTemplate(template))
    tabs = docx.Document(io.BytesIO(replaced)).styles.element.find(f"{W}docDefaults/{W}pPrDefault/{W}pPr/{W}tabs")
    assert [n.get(f"{W}pos") for n in tabs] == ["360"]


@pytest.mark.parametrize("defaults", [
    lambda: d.DocumentDefaults(run=d.RunDefaults(size=-1)),
    lambda: d.DocumentDefaults(paragraph=d.ParagraphDefaults(line_height=d.LineHeight.exact(-1))),
    lambda: d.DocumentDefaults(paragraph=d.ParagraphDefaults(tab_stops=[d.TabStop(1), d.TabStop(1.001)])),
])
async def test_invalid_default_measurements_fail(defaults: Callable[[], d.DocumentDefaults]) -> None:
    with pytest.raises(ValueError):
        await d.Document([], defaults=defaults()).to_bytes()


async def test_template_without_defaults_gets_a_legally_ordered_new_defaults_node(tmp_path: pathlib.Path) -> None:
    original = await d.Document([]).to_bytes()
    def remove_defaults(root) -> None:
        root.remove(root.find(f"{W}docDefaults"))
    template = tmp_path / "without-defaults.docx"
    template.write_bytes(test_notes.rewrite(original, "word/styles.xml", remove_defaults))
    absent = await d.Document([]).to_bytes(d.DocumentTemplate(template))
    assert docx.Document(io.BytesIO(absent)).styles.element.find(f"{W}docDefaults") is None
    data = await d.Document([], defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(bidi=False),
        run=d.RunDefaults(rtl=True))).to_bytes(d.DocumentTemplate(template))
    styles = docx.Document(io.BytesIO(data)).styles.element
    assert styles[0].tag == f"{W}docDefaults"
    assert [n.tag for n in styles[0]] == [f"{W}rPrDefault", f"{W}pPrDefault"]


async def test_explicit_default_indents_remove_conflicting_character_unit_attributes(tmp_path: pathlib.Path) -> None:
    original = await d.Document([], defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(
        left_indent=12, right_indent=24, first_line_indent=6))).to_bytes()
    def character_units(root) -> None:
        ind = root.find(f"{W}docDefaults/{W}pPrDefault/{W}pPr/{W}ind")
        for name in ["leftChars", "rightChars", "firstLineChars", "hangingChars"]:
            ind.set(f"{W}{name}", "100")
    template = tmp_path / "character-indents.docx"
    template.write_bytes(test_notes.rewrite(original, "word/styles.xml", character_units))
    data = await d.Document([], defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(
        left_indent=0, right_indent=0, first_line_indent=-6))).to_bytes(d.DocumentTemplate(template))
    ind = docx.Document(io.BytesIO(data)).styles.element.find(f"{W}docDefaults/{W}pPrDefault/{W}pPr/{W}ind")
    assert dict(ind.attrib) == {f"{W}left": "0", f"{W}right": "0", f"{W}hanging": "120"}


async def test_default_references_are_resolved_once_and_snapshots_keep_detached_values() -> None:
    calls = []
    defaults = d.DocumentDefaults(run=d.RunDefaults(size=12))
    def produce() -> d.DocumentDefaults:
        calls.append(True)
        return defaults
    source = d.Document([], defaults=produce)
    snapshot = await d.Document([], defaults=source.deferred("defaults")).resolve()
    defaults.run.size = 20
    data = await snapshot.to_bytes()
    assert await snapshot.to_bytes() == data
    assert calls == [True]
    run = docx.Document(io.BytesIO(data)).styles.element.find(f"{W}docDefaults/{W}rPrDefault/{W}rPr")
    assert run.find(f"{W}sz").get(f"{W}val") == "24"
