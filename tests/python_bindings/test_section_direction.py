"""Section direction is independent of text-level bidi formatting."""
import io
import pathlib

import docx
import pytest

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


@pytest.mark.parametrize("bidi", [None, False, True])
async def test_section_direction_preserves_explicit_text_direction(bidi: bool | None) -> None:
    data = await d.Document([d.Section(
        children=[d.Paragraph(children=[d.TextRun("first العربية last", rtl=False)], bidi=False)],
        properties=d.SectionProperties(bidi=bidi, columns={"count": 2}),
    )]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    properties = output.sections[0]._sectPr
    node = properties.find(f"{W}bidi")
    if bidi is None:
        assert node is None
    else:
        assert node.get(f"{W}val") == ("1" if bidi else "0")
    assert properties.find(f"{W}cols").get(f"{W}num") == "2"
    paragraph = output.paragraphs[0]
    assert paragraph.text == "first العربية last"
    assert paragraph._p.find(f"{W}pPr/{W}bidi").get(f"{W}val") == "0"
    assert paragraph.runs[0].font.rtl is False


async def test_section_direction_inherits_and_explicit_false_overrides() -> None:
    source = d.Section([], properties=lambda: d.SectionProperties(bidi=lambda: True))
    hidden = d.Section([], properties=lambda: (_ for _ in ()).throw(AssertionError("hidden section ran")))
    hidden.condition = False
    snapshot = await d.Document([
        d.Section([d.Paragraph(text="first")], properties=source.deferred("properties")),
        hidden,
        d.Section([d.Paragraph(text="inherited")]),
        d.Section([d.Paragraph(text="off")], properties=d.SectionProperties(bidi=False)),
        d.Section([d.Paragraph(text="inherited off")], properties=d.SectionProperties()),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert [section._sectPr.find(f"{W}bidi").get(f"{W}val") for section in output.sections] == ["1", "1", "0", "0"]


async def test_template_section_direction_is_preserved_or_explicitly_overridden(tmp_path: pathlib.Path) -> None:
    template = tmp_path / "rtl.docx"
    template.write_bytes(await d.Document([d.Section([], properties=d.SectionProperties(bidi=True))]).to_bytes())
    for value, expected in [(None, "1"), (False, "0")]:
        data = await d.Document([d.Section([], properties=d.SectionProperties(bidi=value))]).to_bytes(d.DocumentTemplate(template))
        output = docx.Document(io.BytesIO(data))
        assert output.sections[-1]._sectPr.find(f"{W}bidi").get(f"{W}val") == expected
