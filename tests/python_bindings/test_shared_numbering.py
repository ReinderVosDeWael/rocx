"""Abstract numbering IDs can be shared by independently configured instances."""
import io
import zipfile

import docx
import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def numbering(data):
    return docx.Document(io.BytesIO(data)).part.numbering_part.element


async def test_share_forward_reserved_identity_and_keep_concrete_overrides():
    levels = [d.NumberingLevel()]
    snapshot = await d.Document([], numbering=[
        d.NumberingDefinition(80, levels),
        d.NumberingDefinition(81, levels, abstract_id=lambda: 9),
        d.NumberingDefinition(82, levels, abstract_id=9,
                              start_overrides=[d.NumberingStartOverride(0, 7)]),
    ]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    root = numbering(data)
    ids = {node.get(f"{W}numId"): node for node in root.findall(f"{W}num")}
    assert [ids[str(i)].find(f"{W}abstractNumId").get(f"{W}val") for i in (80, 81, 82)] == ["10", "9", "9"]
    assert len([node for node in root if node.get(f"{W}abstractNumId") == "9"]) == 1
    assert ids["81"].find(f"{W}lvlOverride") is None
    assert ids["82"].find(f"{W}lvlOverride/{W}startOverride").get(f"{W}val") == "7"


async def test_reject_conflicting_levels():
    with pytest.raises(ValueError, match="require identical levels"):
        await d.Document([], numbering=[
            d.NumberingDefinition(80, [d.NumberingLevel()], abstract_id=100),
            d.NumberingDefinition(81, [d.NumberingLevel(start=3)], abstract_id=100),
        ]).to_bytes()


async def test_preserve_imported_abstract_and_reject_collisions(tmp_path):
    levels = [d.NumberingLevel()]
    original = await d.Document([], numbering=[
        d.NumberingDefinition(80, levels, abstract_id=100),
    ]).to_bytes()
    path = tmp_path / "template.docx"
    renamed = "word/resources/list-definitions.xml"
    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(path, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name in {"[Content_Types].xml", "word/_rels/document.xml.rels"}:
                data = data.replace(b"word/numbering.xml", renamed.encode())
                data = data.replace(b'Target="numbering.xml"', b'Target="resources/list-definitions.xml"')
            target.writestr(renamed if name == "word/numbering.xml" else name, data)
    template = d.DocumentTemplate(path)
    with pytest.raises(ValueError, match="abstract numbering ID already exists"):
        await d.Document([], numbering=[
            d.NumberingDefinition(81, levels, abstract_id=100),
        ]).to_bytes(template)

    extended = await d.Document([], numbering=[
        d.NumberingDefinition(81, levels, abstract_id=101),
    ]).to_bytes(template)
    with zipfile.ZipFile(io.BytesIO(extended)) as package:
        assert renamed in package.namelist()
        assert "word/numbering.xml" not in package.namelist()
    before, after = numbering(original), numbering(extended)
    for attr, identity in (("abstractNumId", "100"), ("numId", "80")):
        key = f"{W}{attr}"
        old = next(node for node in before if node.get(key) == identity)
        retained = next(node for node in after if node.get(key) == identity)
        assert etree.tostring(retained) == etree.tostring(old)


async def test_automatic_allocation_reports_exhaustion(tmp_path):
    original = await d.Document([], numbering=[
        d.NumberingDefinition(80, abstract_id=2**32 - 1),
    ]).to_bytes()
    path = tmp_path / "last-abstract-id.docx"
    path.write_bytes(original)
    with pytest.raises(ValueError, match="automatic abstract numbering IDs exhausted"):
        await d.Document([], numbering=[d.NumberingDefinition(81)]).to_bytes(
            d.DocumentTemplate(path)
        )
