"""Block content controls keep story blocks and table cell boundaries."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_block_control_preserves_block_types_and_cell_trailing_paragraph():
    def table(child):
        return d.Table([d.TableRow([d.TableCell([child])])])

    document = d.Document([d.Section([
        d.BlockContentControl(
            lambda: [d.Paragraph(text="outer paragraph"), table(d.Paragraph(text="outer table"))],
            alias="Block",
            tag="outer",
            lock=lambda: "sdtLocked",
        ),
        table(d.BlockContentControl([d.Paragraph(text="cell control")], tag="cell")),
        d.BlockContentControl(condition=False, alias=lambda: pytest.fail("hidden alias")),
    ])])
    snapshot = await document.resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    outer = root.find(f"{W}body/{W}sdt")
    assert outer.find(f"{W}sdtPr/{W}lock").get(f"{W}val") == "sdtLocked"
    assert [node.tag for node in outer.find(f"{W}sdtContent")] == [f"{W}p", f"{W}tbl"]
    cells = root.findall(f"{W}body/{W}tbl/{W}tr/{W}tc")
    assert [node.tag for node in cells[0]][-2:] == [f"{W}sdt", f"{W}p"]


async def test_block_control_rejects_a_directly_nested_block_control():
    control = d.BlockContentControl([d.BlockContentControl([d.Paragraph(text="nested")])])
    with pytest.raises(ValueError, match="nested block content controls"):
        await d.Document([d.Section([control])]).to_bytes()


async def test_control_ids_are_checked_across_inline_and_block_stories(tmp_path):
    initial = d.InlineContentControl([d.TextRun("old")], id=lambda: 23)
    template = tmp_path / "control-id.docx"
    await d.Document([d.Section([d.Paragraph(children=[initial])])]).save(template)
    duplicate = d.BlockContentControl([d.Paragraph(text="duplicate")], id=23)
    with pytest.raises(ValueError, match="duplicate content control ID"):
        await d.Document([d.Section([duplicate])]).to_bytes(d.DocumentTemplate(template))
    with pytest.raises(ValueError, match="duplicate content control ID"):
        await d.Document([d.Section([d.Paragraph(children=[initial]), duplicate])]).to_bytes()

    added = d.BlockContentControl([d.Paragraph(text="added")], id=24)
    data = await d.Document([d.Section([added])]).to_bytes(d.DocumentTemplate(template))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert [node.get(f"{W}val") for node in root.findall(f".//{W}sdtPr/{W}id")] == ["23", "24"]


@pytest.mark.parametrize("id", [0, 2**31])
async def test_content_control_ids_require_supported_positive_range(id):
    control = d.BlockContentControl([d.Paragraph(text="content")], id=id)
    with pytest.raises(ValueError, match="content control ID must be"):
        await d.Document([d.Section([control])]).to_bytes()


async def test_plain_text_block_control_accepts_one_paragraph():
    control = d.BlockContentControl([d.Paragraph(text="plain")], text_type="plain")
    data = await d.Document([d.Section([control])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    wrapper = root.find(f"{W}body/{W}sdt")
    assert wrapper.find(f"{W}sdtPr/{W}text") is not None
    assert wrapper.find(f"{W}sdtContent/{W}p/{W}r/{W}t").text == "plain"
    invalid = d.BlockContentControl([d.Paragraph(text="a"), d.Paragraph(text="b")], text_type="plain")
    with pytest.raises(ValueError, match="exactly one paragraph"):
        await d.Document([d.Section([invalid])]).to_bytes()
