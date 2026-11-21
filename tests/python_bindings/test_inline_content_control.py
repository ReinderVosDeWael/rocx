"""Inline content controls preserve their context, metadata and template boundaries."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def xml(data):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/document.xml"))


@pytest.mark.parametrize("lock", [None, "unlocked", "sdtLocked", "contentLocked", "sdtContentLocked"])
async def test_inline_content_control_projects_rich_text_and_typed_lock(lock):
    control = d.InlineContentControl(
        [d.TextRun("controlled ", bold=True), d.Tab(), d.TextRun("value")],
        alias=lambda: "Reader & editor",
        tag="review-caption",
        lock=lock,
    )
    hidden = d.InlineContentControl(condition=False, alias=lambda: pytest.fail("hidden producer"))
    snapshot = await d.Document([d.Section([d.Paragraph(children=[control, hidden])])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    rendered, = xml(data).findall(f".//{W}p/{W}sdt")
    properties = rendered.find(f"{W}sdtPr")
    assert properties.find(f"{W}alias").get(f"{W}val") == "Reader & editor"
    assert properties.find(f"{W}tag").get(f"{W}val") == "review-caption"
    lock_element = properties.find(f"{W}lock")
    assert (lock_element.get(f"{W}val") if lock_element is not None else None) == lock
    assert properties.find(f"{W}richText") is not None
    assert [node.text for node in rendered.findall(f"{W}sdtContent/{W}r/{W}t")] == ["controlled ", "value"]
    assert rendered.find(f"{W}sdtContent/{W}r/{W}tab") is not None


async def test_inline_content_control_rejects_nested_control():
    outer = d.InlineContentControl([d.InlineContentControl([d.TextRun("nested")])])
    with pytest.raises(ValueError, match="children must be text, tabs, breaks or images"):
        await d.Document([d.Section([d.Paragraph(children=[outer])])]).to_bytes()


async def test_imported_control_stays_protected_from_text_replacement(tmp_path):
    control = d.InlineContentControl([d.TextRun("protected text")], tag="protected")
    source = await d.Document([d.Section([d.Paragraph(children=[control])])]).to_bytes()
    template = tmp_path / "control.docx"
    template.write_bytes(source)
    output = await d.Document([]).to_bytes(d.DocumentTemplate(template))
    assert etree.tostring(xml(output).find(f".//{W}sdt")) == etree.tostring(xml(source).find(f".//{W}sdt"))
    with pytest.raises(ValueError, match="structured boundary"):
        await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"protected text": "modified"}))


@pytest.mark.parametrize("multi_line", [None, False, True])
async def test_plain_text_control_encodes_line_break_policy(multi_line):
    content = "first\nsecond" if multi_line else "single"
    control = d.InlineContentControl([d.TextRun(content)],
                                     text_type="plain", multi_line=multi_line)
    data = await d.Document([d.Section([d.Paragraph(children=[control])])]).to_bytes()
    rendered = xml(data).find(f".//{W}sdt")
    settings = rendered.find(f"{W}sdtPr/{W}text")
    assert settings.get(f"{W}multiLine") == (None if multi_line is None else str(int(multi_line)))
    assert len(rendered.findall(f"{W}sdtContent/{W}r")) == 1
    assert len(rendered.findall(f"{W}sdtContent/{W}r/{W}br")) == int(bool(multi_line))


@pytest.mark.parametrize("children, message", [
    ([d.TextRun("one"), d.TextRun("two")], "exactly one text run"),
    ([d.TextRun("one\ntwo")], "line breaks require multi_line"),
    ([d.Tab()], "exactly one text run"),
])
async def test_plain_text_control_rejects_nonconforming_content(children, message):
    control = d.InlineContentControl(children, text_type="plain")
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[control])])]).to_bytes()


async def test_dropdown_control_encodes_selected_display_and_ordered_options():
    control = d.InlineContentControl(
        [d.TextRun("Accepted")], text_type="dropdown", selected=lambda: "accepted",
        items=[d.ContentControlListItem("Pending", "pending"),
               d.ContentControlListItem("Accepted", "accepted")],
    )
    data = await d.Document([d.Section([d.Paragraph(children=[control])])]).to_bytes()
    rendered = xml(data).find(f".//{W}sdt")
    dropdown = rendered.find(f"{W}sdtPr/{W}dropDownList")
    assert dropdown.get(f"{W}lastValue") == "accepted"
    assert [(node.get(f"{W}displayText"), node.get(f"{W}value")) for node in dropdown] == [
        ("Pending", "pending"), ("Accepted", "accepted"),
    ]


@pytest.mark.parametrize("selected, display, items, message", [
    ("missing", "Accepted", [d.ContentControlListItem("Accepted", "accepted")], "selected value must identify"),
    ("accepted", "Pending", [d.ContentControlListItem("Accepted", "accepted")], "cached text must match"),
    ("accepted", "Accepted", [d.ContentControlListItem("Accepted", "accepted")] * 2, "require unique"),
])
async def test_dropdown_rejects_invalid_items_and_selection(selected, display, items, message):
    control = d.InlineContentControl([d.TextRun(display)], text_type="dropdown", selected=selected, items=items)
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[control])])]).to_bytes()
