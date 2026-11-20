"""Numbering-label alignment is independent of paragraph alignment."""
import io
import pathlib
import xml.etree.ElementTree as ET

import docx
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_label_alignments_and_omission_survive_both_level_locations() -> None:
    values = [None, "left", "center", "right", "start", "end"]
    snapshot = await d.Document([d.Section([d.Paragraph(text="unchanged", alignment="center", numbering=d.NumberingReference(80))])],
        numbering=[d.NumberingDefinition(80,
            levels=[d.NumberingLevel(level=i, alignment=value) for i, value in enumerate(values)],
            level_overrides=[d.NumberingLevel(level=i, alignment=value) for i, value in enumerate(reversed(values))])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    root = output.part.numbering_part.element
    num = next(n for n in root if n.get(f"{W}numId") == "80")
    identity = num.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in root if n.get(f"{W}abstractNumId") == identity)
    for nodes, expected in [(abstract.findall(f"{W}lvl"), values), (num.findall(f"{W}lvlOverride/{W}lvl"), list(reversed(values)))]:
        assert [None if (n := level.find(f"{W}lvlJc")) is None else n.get(f"{W}val") for level in nodes] == expected
    assert output.paragraphs[0].text == "unchanged"
    assert output.paragraphs[0]._p.find(f"{W}pPr/{W}jc").get(f"{W}val") == "center"


async def test_label_alignment_uses_generic_nested_deferred_fields() -> None:
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(alignment=lambda: "start")],
        level_overrides=[d.NumberingLevel(alignment=lambda: "end")])]).to_bytes()
    root = docx.Document(io.BytesIO(data)).part.numbering_part.element
    num = next(n for n in root if n.get(f"{W}numId") == "80")
    identity = num.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in root if n.get(f"{W}abstractNumId") == identity)
    assert abstract.find(f"{W}lvl/{W}lvlJc").get(f"{W}val") == "start"
    assert num.find(f"{W}lvlOverride/{W}lvl/{W}lvlJc").get(f"{W}val") == "end"


async def test_new_label_alignment_does_not_rewrite_imported_numbering(tmp_path: pathlib.Path) -> None:
    original = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(alignment="right")])]).to_bytes()
    template = tmp_path / "numbering-template.docx"
    template.write_bytes(original)
    data = await d.Document([], numbering=[d.NumberingDefinition(81,
        levels=[d.NumberingLevel(alignment="start")])]).to_bytes(d.DocumentTemplate(template))
    before = docx.Document(io.BytesIO(original)).part.numbering_part.element
    after = docx.Document(io.BytesIO(data)).part.numbering_part.element
    for old in before:
        identity = old.get(f"{W}abstractNumId")
        if old.tag == f"{W}abstractNum":
            retained = next(n for n in after if n.tag == old.tag and n.get(f"{W}abstractNumId") == identity)
            assert ET.tostring(retained) == ET.tostring(old)
    added = next(n for n in after if n.get(f"{W}numId") == "81")
    identity = added.find(f"{W}abstractNumId").get(f"{W}val")
    abstract = next(n for n in after if n.get(f"{W}abstractNumId") == identity)
    assert abstract.find(f"{W}lvl/{W}lvlJc").get(f"{W}val") == "start"


@pytest.mark.parametrize("token", ["", "CENTER", "justify", "both", "invalid"])
@pytest.mark.parametrize("replacement", [False, True])
async def test_unsupported_label_alignment_tokens_are_rejected(token: str, replacement: bool) -> None:
    definition = d.NumberingDefinition(80, [d.NumberingLevel()])
    if replacement:
        definition.level_overrides = [d.NumberingLevel(alignment=token)]
    else:
        definition.levels = [d.NumberingLevel(alignment=token)]
    with pytest.raises(ValueError, match="numbering alignment"):
        await d.Document([], numbering=[definition]).to_bytes()
