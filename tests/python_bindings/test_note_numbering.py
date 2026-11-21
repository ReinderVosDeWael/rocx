"""Note numbering defaults merge independently of special note entries."""
import io
import zipfile

import pytest
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def settings(data):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/settings.xml"))


async def test_note_numbering_settings_preserve_other_defaults_and_special_entries(tmp_path):
    original = d.Document([d.Section([d.Paragraph(children=[d.NoteReference.footnote(1)])])],
        notes=[d.Note.footnote(1, [d.Paragraph(text="note")])],
        defaults=d.DocumentDefaults(
            footnote_numbering=d.NoteNumbering(3, "eachPage"),
            endnote_numbering=d.NoteNumbering(7, "eachSect"),
        ),
    )
    data = await original.to_bytes()
    template = tmp_path / "note-numbering.docx"
    template.write_bytes(data)
    update = d.Document([], defaults=d.DocumentDefaults(
        footnote_numbering=d.NoteNumbering(restart=lambda: "continuous"),
    ))
    changed = await update.to_bytes(d.DocumentTemplate(template))
    before, after = settings(data), settings(changed)
    old, new = (root.find(f"{W}footnotePr") for root in (before, after))
    assert new.find(f"{W}numStart").get(f"{W}val") == "3"
    assert new.find(f"{W}numRestart").get(f"{W}val") == "continuous"
    assert [node.get(f"{W}id") for node in new.findall(f"{W}footnote")] == [
        node.get(f"{W}id") for node in old.findall(f"{W}footnote")
    ]
    assert etree.tostring(before.find(f"{W}endnotePr")) == etree.tostring(after.find(f"{W}endnotePr"))


async def test_unknown_note_restart_mode_is_rejected():
    defaults = d.DocumentDefaults(footnote_numbering=d.NoteNumbering(restart="invalid"))
    with pytest.raises(ValueError, match="unknown note numbering restart"):
        await d.Document([], defaults=defaults).to_bytes()


@pytest.mark.parametrize("token", ["decimal", "lowerRoman", "upperRoman", "lowerLetter", "upperLetter"])
async def test_note_number_format_is_encoded_before_start(token):
    defaults = d.DocumentDefaults(footnote_numbering=d.NoteNumbering(3, "eachPage", lambda: token))
    root = settings(await d.Document([], defaults=defaults).to_bytes()).find(f"{W}footnotePr")
    assert [node.tag for node in root][:3] == [f"{W}numFmt", f"{W}numStart", f"{W}numRestart"]
    assert root.find(f"{W}numFmt").get(f"{W}val") == token


async def test_unsupported_note_number_format_is_rejected():
    defaults = d.DocumentDefaults(endnote_numbering=d.NoteNumbering(format="unknown"))
    with pytest.raises(ValueError, match="unknown note numbering format"):
        await d.Document([], defaults=defaults).to_bytes()


@pytest.mark.parametrize("footnote, endnote", [
    ("pageBottom", "docEnd"),
    ("beneathText", "sectEnd"),
])
async def test_document_note_positions_serialize_before_numbering(footnote, endnote):
    defaults = d.DocumentDefaults(
        footnote_position=lambda: footnote,
        endnote_position=endnote,
        footnote_numbering=d.NoteNumbering(1, "eachPage", "decimal"),
    )
    root = settings(await d.Document([], defaults=defaults).to_bytes())
    for kind, position in [("footnote", footnote), ("endnote", endnote)]:
        owner = root.find(f"{W}{kind}Pr")
        assert owner[0].tag == f"{W}pos"
        assert owner[0].get(f"{W}val") == position


@pytest.mark.parametrize("kind, value", [("footnote", "docEnd"), ("endnote", "pageBottom")])
async def test_note_positions_reject_unsupported_contexts(kind, value):
    defaults = d.DocumentDefaults(**{f"{kind}_position": value})
    with pytest.raises(ValueError, match=f"unknown {kind} position"):
        await d.Document([], defaults=defaults).to_bytes()
