"""Author metadata, deleted-text encoding and imported change preservation."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def document_xml(data):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/document.xml"))


async def test_render_revisions_and_preserve_imported_revision_id(tmp_path):
    old = d.TextRun("original change", revision=d.TextRevision("insertion", "Original editor"))
    seed = await d.Document([d.Section([d.Paragraph(children=[old])])]).to_bytes()
    template = tmp_path / "revised.docx"
    template.write_bytes(seed)
    added = d.TextRun(" inserted ", revision=lambda: d.TextRevision(
        "insertion", "A & B", "2024-02-29T12:34:56Z",
    ))
    removed = d.TextRun(" removed ", bold=True, revision=d.TextRevision("deletion", "A & B"))
    hidden = d.TextRun("hidden", condition=False, revision=lambda: pytest.fail("hidden revision producer"))
    snapshot = await d.Document([d.Section([d.Paragraph(children=[added, removed, hidden])])]).resolve()
    data = await snapshot.to_bytes(d.DocumentTemplate(template))
    assert data == await snapshot.to_bytes(d.DocumentTemplate(template))
    root = document_xml(data)
    old_change = document_xml(seed).find(f".//{W}ins")
    retained, inserted = root.findall(f".//{W}ins")
    deleted = root.find(f".//{W}del")
    assert [node.get(f"{W}id") for node in (retained, inserted, deleted)] == ["1", "2", "3"]
    assert retained.get(f"{W}id") == old_change.get(f"{W}id")
    assert retained.find(f"{W}r/{W}t").text == "original change"
    assert inserted.get(f"{W}author") == "A & B"
    assert inserted.get(f"{W}date") == "2024-02-29T12:34:56Z"
    assert inserted.find(f"{W}r/{W}t").text == " inserted "
    deleted_text = deleted.find(f"{W}r/{W}delText")
    assert deleted_text.text == " removed "
    assert deleted_text.get("{http://www.w3.org/XML/1998/namespace}space") == "preserve"
    assert deleted.find(f"{W}r/{W}t") is None
    assert deleted.find(f"{W}r/{W}rPr/{W}b").get(f"{W}val") == "1"


@pytest.mark.parametrize("revision, message", [
    (d.TextRevision("insertion", " "), "revision.author"),
    (d.TextRevision("deletion", "Editor", "2025-02-29T12:34:56Z"), "revision.date"),
    (d.TextRevision("move", "Editor"), "unknown text revision kind"),
])
async def test_invalid_revision_metadata_is_rejected(revision, message):
    run = d.TextRun("text", revision=revision)
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=[run])])]).to_bytes()


async def test_tracking_setting_can_be_disabled_without_removing_existing_revisions(tmp_path):
    original = d.Document([d.Section([d.Paragraph(children=[
        d.TextRun("inserted", revision=d.TextRevision("insertion", "Editor")),
    ])])], defaults=d.DocumentDefaults(track_revisions=True))
    template = tmp_path / "tracking.docx"
    await original.save(template)
    updated = await d.Document([], defaults=d.DocumentDefaults(track_revisions=lambda: False)).to_bytes(
        d.DocumentTemplate(template),
    )
    with zipfile.ZipFile(io.BytesIO(updated)) as package:
        settings = etree.fromstring(package.read("word/settings.xml"))
    assert settings.find(f"{W}trackRevisions").get(f"{W}val") == "0"
    assert document_xml(updated).find(f".//{W}ins/{W}r/{W}t").text == "inserted"


@pytest.mark.parametrize("action, expected", [("accept", "added"), ("reject", "removed")])
async def test_template_text_revision_action_keeps_the_selected_formatted_run(tmp_path, action, expected):
    seed = tmp_path / "review.docx"
    template_doc = d.Document([d.Section([d.Paragraph(children=[
        d.TextRun("before "),
        d.TextRun("added", bold=True, revision=d.TextRevision("insertion", "Editor")),
        d.TextRun("removed", bold=True, revision=d.TextRevision("deletion", "Editor")),
        d.TextRun(" after"),
    ])])])
    await template_doc.save(seed)
    authored = d.TextRun("new review", revision=d.TextRevision("insertion", "Next editor"))
    result = await d.Document([d.Section([d.Paragraph(children=[authored])])]).to_bytes(
        d.DocumentTemplate(seed, text_revision_action=action),
    )
    before, new = document_xml(result).findall(f"{W}body/{W}p")
    assert [node.text for node in before.findall(f"{W}r/{W}t")] == ["before ", expected, " after"]
    assert before.findall(f"{W}ins") == before.findall(f"{W}del") == []
    assert before.findall(f"{W}r")[1].find(f"{W}rPr/{W}b").get(f"{W}val") == "1"
    assert new.find(f"{W}ins/{W}r/{W}t").text == "new review"


async def test_template_text_revision_action_rejects_protected_controls(tmp_path):
    source = tmp_path / "protected-review.docx"
    control = d.InlineContentControl([
        d.TextRun("added", revision=d.TextRevision("insertion", "Editor")),
    ])
    await d.Document([d.Section([d.Paragraph(children=[control])])]).save(source)
    with pytest.raises(ValueError, match="outside protected structures"):
        await d.Document([]).to_bytes(d.DocumentTemplate(source, text_revision_action="accept"))


def text_move(kind, text="moved text", name="ReviewMove", date="2024-02-29T12:34:56Z"):
    return d.TextRun(text, revision=d.TextRevision(kind, "Editor", date, name))


async def test_text_move_pairs_named_ranges_and_preserves_imported_moves(tmp_path):
    runs = [text_move("move_destination"), text_move("move_source")]
    original = d.Document([d.Section([d.Paragraph(children=runs)])])
    source = tmp_path / "move.docx"
    await original.save(source)
    root = document_xml(source.read_bytes())
    range_ids = []
    for role in ("moveFrom", "moveTo"):
        start = root.find(f".//{W}{role}RangeStart")
        wrapper, end = start.getnext(), start.getnext().getnext()
        assert start.get(f"{W}name") == "ReviewMove"
        assert start.get(f"{W}date") == "2024-02-29T12:34:56Z"
        assert wrapper.tag == f"{W}{role}"
        assert end.tag == f"{W}{role}RangeEnd"
        assert end.get(f"{W}id") == start.get(f"{W}id")
        assert wrapper.find(f"{W}r/{W}t").text == "moved text"
        range_ids.extend([start.get(f"{W}id"), wrapper.get(f"{W}id")])
    assert len(set(range_ids)) == 4
    with pytest.raises(ValueError, match="name already exists in template"):
        await original.to_bytes(d.DocumentTemplate(source))


@pytest.mark.parametrize("runs, message", [
    ([text_move("move_source")], "one source and one destination"),
    ([text_move("move_source"), text_move("move_destination", "other text")], "identical text"),
    ([text_move("move_source", date=None), text_move("move_destination")], "requires a date"),
])
async def test_invalid_text_move_pairs_are_rejected(runs, message):
    with pytest.raises(ValueError, match=message):
        await d.Document([d.Section([d.Paragraph(children=runs)])]).to_bytes()


@pytest.mark.parametrize("action, first, second", [
    ("accept", ["source"], ["destination", "moved text"]),
    ("reject", ["source", "moved text"], ["destination"]),
])
async def test_template_move_action_keeps_the_selected_placement(tmp_path, action, first, second):
    source = tmp_path / "review-move.docx"
    document = d.Document([d.Section([
        d.Paragraph(children=[d.TextRun("source"), text_move("move_source")]),
        d.Paragraph(children=[d.TextRun("destination"), text_move("move_destination")]),
    ])])
    await document.save(source)
    data = await d.Document([]).to_bytes(d.DocumentTemplate(source, text_revision_action=action))
    root = document_xml(data)
    paragraphs = root.findall(f"{W}body/{W}p")
    assert [[node.text for node in p.findall(f"{W}r/{W}t")] for p in paragraphs] == [first, second]
    assert root.find(f".//{W}moveFromRangeStart") is None
    assert root.find(f".//{W}moveToRangeStart") is None


async def test_template_move_action_rejects_missing_pair(tmp_path):
    from test_notes import rewrite

    document = d.Document([d.Section([d.Paragraph(children=[
        text_move("move_source"), text_move("move_destination"),
    ])])])
    original = await document.to_bytes()

    def remove_destination(root):
        paragraph = root.find(f".//{W}p")
        for name in ("moveToRangeStart", "moveTo", "moveToRangeEnd"):
            paragraph.remove(paragraph.find(f"{W}{name}"))

    source = tmp_path / "unpaired-move.docx"
    source.write_bytes(rewrite(original, "word/document.xml", remove_destination))
    with pytest.raises(ValueError, match="requires matching source and destination"):
        await d.Document([]).to_bytes(d.DocumentTemplate(source, text_revision_action="accept"))
