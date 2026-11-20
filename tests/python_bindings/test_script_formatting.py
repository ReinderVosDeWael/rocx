"""Language hints and complex-script formatting serialize without shaping text."""
import io
import pathlib
import zipfile

import docx
from lxml import etree
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_language_hints_and_complex_script_flags_keep_independent_values() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.TextRun("العربية 日本語", language=d.RunLanguage(latin="en-GB", east_asia="ja-JP", bidi="ar-SA"),
            bold=False, bold_complex_script=True, italic=True, italic_complex_script=False,
            size=11, size_complex_script=14.5, rtl=True, complex_script=False),
        d.TextRun("off", rtl=False, complex_script=True), d.TextRun("absent"),
    ])])]).to_bytes()
    paragraph = docx.Document(io.BytesIO(data)).paragraphs[0]
    run = paragraph.runs[0]
    assert run.text == "العربية 日本語"
    assert run.bold is False
    assert run.font.cs_bold is True
    assert run.italic is True
    assert run.font.cs_italic is False
    assert run.font.rtl is True
    assert run.font.complex_script is False
    properties = run._r.find(f"{W}rPr")
    assert properties.find(f"{W}sz").get(f"{W}val") == "22"
    assert properties.find(f"{W}szCs").get(f"{W}val") == "29"
    assert properties.find(f"{W}lang").attrib == {f"{W}val": "en-GB", f"{W}eastAsia": "ja-JP", f"{W}bidi": "ar-SA"}
    assert paragraph.runs[1].font.rtl is False
    assert paragraph.runs[1].font.complex_script is True
    assert paragraph.runs[2]._r.find(f"{W}rPr") is None


async def test_script_properties_use_generic_deferred_references_and_styles() -> None:
    source = d.TextRun("source", language=lambda: d.RunLanguage(bidi="he-IL"), rtl=lambda: True,
        complex_script=lambda: False, bold_complex_script=lambda: False, italic_complex_script=lambda: True, size_complex_script=lambda: 13,
        font_east_asia=lambda: "East Asian", font_complex_script=lambda: "Hebrew")
    target = d.TextRun("target", **{name: source.deferred(name) for name in ["language", "rtl", "complex_script", "bold_complex_script", "italic_complex_script", "size_complex_script", "font_east_asia", "font_complex_script"]})
    snapshot = await d.Document([d.Section(children=[d.Paragraph(children=[target], style="Script")])],
        styles=[d.ParagraphStyleDefinition(name="Script", language=d.RunLanguage(latin="en-US", bidi="ar-SA"),
            rtl=False, complex_script=True, bold_complex_script=True, italic_complex_script=False, size_complex_script=14)]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.styles["Script"].font.cs_bold is True
    assert output.styles["Script"].font.rtl is False
    run = output.paragraphs[0].runs[0]
    assert run.font.cs_bold is False
    assert run.font.rtl is True
    assert run._r.find(f"{W}rPr/{W}lang").get(f"{W}bidi") == "he-IL"
    assert run._r.find(f"{W}rPr/{W}rFonts").get(f"{W}cs") == "Hebrew"


@pytest.mark.parametrize("value", [-1, float("nan"), float("inf"), 1e300])
@pytest.mark.parametrize("style", [False, True])
async def test_bad_complex_script_sizes_fail(value: float, style: bool) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("bad", **({} if style else {"size_complex_script": value}))])])],
            styles=[d.ParagraphStyleDefinition(name="Bad", size_complex_script=value)] if style else []).to_bytes()
    assert error.value.rule == "invalid_measurement"


async def test_language_slots_are_unparsed_strings_and_xml_is_escaped() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("text", language=d.RunLanguage(latin="custom<&", bidi=""))])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    language = root.find(f".//{W}lang")
    assert language.get(f"{W}val") == "custom<&"
    assert language.get(f"{W}bidi") == ""
    assert language.get(f"{W}eastAsia") is None


async def test_script_font_slots_do_not_overwrite_each_other() -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("mixed", font="Latin",
        font_east_asia="日本語 < Font", font_complex_script="Arabic")])])]).to_bytes()
    run = docx.Document(io.BytesIO(data)).paragraphs[0].runs[0]
    assert run.font.name == "Latin"
    assert run._r.find(f"{W}rPr/{W}rFonts").attrib == {f"{W}ascii": "Latin", f"{W}hAnsi": "Latin", f"{W}eastAsia": "日本語 < Font", f"{W}cs": "Arabic"}


async def test_script_font_style_updates_remove_only_the_overridden_theme_slot(tmp_path: pathlib.Path) -> None:
    import test_notes
    source = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Fonts", font="Latin", font_east_asia="Old EA", font_complex_script="Old CS")]).to_bytes()
    def themes(root):
        style = next(n for n in root if n.get(f"{W}styleId") == "Fonts")
        fonts = style.find(f"{W}rPr/{W}rFonts")
        for name, value in [("asciiTheme", "minorAscii"), ("hAnsiTheme", "minorHAnsi"), ("eastAsiaTheme", "minorEastAsia"), ("cstheme", "minorBidi")]:
            fonts.set(f"{W}{name}", value)
    template = tmp_path / "fonts.docx"
    template.write_bytes(test_notes.rewrite(source, "word/styles.xml", themes))
    data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Fonts", font_east_asia="New EA")]).to_bytes(d.DocumentTemplate(template))
    style = docx.Document(io.BytesIO(data)).styles["Fonts"]
    fonts = style._element.find(f"{W}rPr/{W}rFonts")
    assert fonts.get(f"{W}eastAsia") == "New EA"
    assert fonts.get(f"{W}eastAsiaTheme") is None
    assert fonts.get(f"{W}cstheme") == "minorBidi"
    assert fonts.get(f"{W}asciiTheme") == "minorAscii"
    assert fonts.get(f"{W}cs") == "Old CS"


async def test_hidden_script_properties_are_not_evaluated() -> None:
    hidden = d.TextRun("hidden", language=lambda: (_ for _ in ()).throw(AssertionError("hidden language ran")))
    hidden.condition = False
    data = await d.Document([d.Section(children=[d.Paragraph(children=[hidden, d.TextRun("visible")])])]).to_bytes()
    assert docx.Document(io.BytesIO(data)).paragraphs[0].text == "visible"


async def test_script_language_style_updates_preserve_omitted_slots(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "languages.docx"
    template.write_bytes(await d.Document([], styles=[d.ParagraphStyleDefinition(name="Languages",
        language=d.RunLanguage(latin="en-US", east_asia="ja-JP", bidi="ar-SA"))]).to_bytes())
    data = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Languages", language=d.RunLanguage(bidi="he-IL"))]).to_bytes(d.DocumentTemplate(template))
    language = docx.Document(io.BytesIO(data)).styles["Languages"]._element.find(f"{W}rPr/{W}lang")
    assert dict(language.attrib) == {f"{W}val": "en-US", f"{W}eastAsia": "ja-JP", f"{W}bidi": "he-IL"}


@pytest.mark.parametrize("bidi", [None, False, True])
async def test_paragraph_bidi_is_distinct_from_run_rtl_and_keeps_text_order(bidi: bool | None) -> None:
    data = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("first العربية last", rtl=False)],
        bidi=bidi, alignment="left", left_indent=12)])]).to_bytes()
    output = docx.Document(io.BytesIO(data)).paragraphs[0]
    assert output.text == "first العربية last"
    assert output.runs[0].font.rtl is False
    assert output.paragraph_format.left_indent.pt == 12
    node = output._p.find(f"{W}pPr/{W}bidi")
    if bidi is None:
        assert node is None
    else:
        assert node.get(f"{W}val") == ("1" if bidi else "0")


async def test_paragraph_bidi_style_and_deferred_override_remain_separate() -> None:
    source = d.Paragraph(text="source", bidi=lambda: False)
    data = await d.Document([d.Section(children=[d.Paragraph(text="direct", style="RTL", bidi=source.deferred("bidi"))])],
        styles=[d.ParagraphStyleDefinition(name="RTL", bidi=True)]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.styles["RTL"]._element.find(f"{W}pPr/{W}bidi").get(f"{W}val") == "1"
    assert output.paragraphs[0]._p.find(f"{W}pPr/{W}bidi").get(f"{W}val") == "0"
