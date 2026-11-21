"""Scoped structural template replacements for editable plain-text controls."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def paragraph(tag, text="original", **properties):
    return d.Paragraph(children=[
        d.InlineContentControl([d.TextRun(text, bold=True)], tag=tag,
                               text_type="plain", **properties),
    ])


def xml(data, name="word/document.xml"):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read(name))


async def test_exact_tag_edit_preserves_format_and_other_controls(tmp_path):
    seed = tmp_path / "tagged.docx"
    original = d.Document([d.Section([
        paragraph("target", multi_line=True, lock="sdtLocked"),
        paragraph("other", text="other text"),
    ])])
    await original.save(seed)
    options = d.DocumentTemplate(seed, content_control_replacements={"target": "new\ntext"})
    data = await d.Document([]).to_bytes(options)
    changed, kept = xml(data).findall(f".//{W}sdt")
    run = changed.find(f"{W}sdtContent/{W}r")
    assert [node.text for node in run.findall(f"{W}t")] == ["new", "text"]
    assert run.find(f"{W}br") is not None
    assert run.find(f"{W}rPr/{W}b").get(f"{W}val") == "1"
    assert changed.find(f"{W}sdtPr/{W}lock").get(f"{W}val") == "sdtLocked"
    assert kept.find(f"{W}sdtContent/{W}r/{W}t").text == "other text"


async def test_tag_edit_respects_story_scope_and_detects_ambiguity(tmp_path):
    source = tmp_path / "two-stories.docx"
    original = d.Document([d.Section([paragraph("shared")],
        headers={"default": d.Header([paragraph("shared", text="header")])},
    )])
    await original.save(source)
    with pytest.raises(ValueError, match="matches more than one"):
        await d.Document([]).to_bytes(d.DocumentTemplate(
            source, content_control_replacements={"shared": "new"},
        ))
    data = await d.Document([]).to_bytes(d.DocumentTemplate(
        source, scope="body", content_control_replacements={"shared": "new"},
    ))
    assert xml(data).find(f".//{W}sdtContent/{W}r/{W}t").text == "new"
    assert xml(data, "word/header1.xml").find(f".//{W}sdtContent/{W}r/{W}t").text == "header"


@pytest.mark.parametrize("tag, replacement, message", [
    ("absent", "new", "was not found"),
    ("locked", "new", "locked, bound and placeholder"),
    ("single-line", "new\nline", "line breaks require multiLine"),
])
async def test_tag_edit_rejects_unsupported_requests(tmp_path, tag, replacement, message):
    source = tmp_path / "unsupported-edit.docx"
    original = d.Document([d.Section([
        paragraph("locked", lock="contentLocked"),
        paragraph("single-line"),
    ])])
    await original.save(source)
    with pytest.raises(ValueError, match=message):
        await d.Document([]).to_bytes(d.DocumentTemplate(
            source, content_control_replacements={tag: replacement},
        ))
