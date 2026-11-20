"""Legal numbering preserves requests without calculating displayed labels."""
import io
import zipfile
import xml.etree.ElementTree as ET

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_legal_numbering_distinguishes_omitted_false_and_true() -> None:
    definition = d.NumberingDefinition(80, levels=[
        d.NumberingLevel(level=i, format="upperRoman", text="%1.%2.%3", is_legal=flag)
        for i, flag in enumerate([None, False, True])], level_overrides=[
            d.NumberingLevel(level=2, format="lowerLetter", is_legal=False)])
    snapshot = await d.Document([d.Section([d.Paragraph(text="unchanged")])], numbering=[definition]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = ET.fromstring(package.read("word/numbering.xml"))
    abstract = root.findall(f"{W}abstractNum/{W}lvl")[-3:]
    assert [None if (n := level.find(f"{W}isLgl")) is None else n.get(f"{W}val") for level in abstract] == [None, "0", "1"]
    assert [level.find(f"{W}numFmt").get(f"{W}val") for level in abstract] == ["upperRoman"] * 3
    replacement = root.find(f"{W}num[@{W}numId='80']/{W}lvlOverride/{W}lvl")
    assert replacement.find(f"{W}isLgl").get(f"{W}val") == "0"
    assert replacement.find(f"{W}numFmt").get(f"{W}val") == "lowerLetter"


async def test_legal_numbering_uses_generic_nested_deferred_fields() -> None:
    data = await d.Document([d.Section([])], numbering=[d.NumberingDefinition(81,
        levels=[d.NumberingLevel(is_legal=lambda: True)],
        level_overrides=[d.NumberingLevel(is_legal=lambda: False)])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = ET.fromstring(package.read("word/numbering.xml"))
    assert [node.get(f"{W}val") for node in root.iter(f"{W}isLgl")] == ["1", "0"]
