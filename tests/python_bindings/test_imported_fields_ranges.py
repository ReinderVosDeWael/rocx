"""Destructive field/range guards exercised against independent Word packages."""
import io

import pytest
from docx import Document
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

from rocx import declarative as d
from rocx.editor import ImportedDocument, InlineEdit


def imported(word):
    f = io.BytesIO()
    word.save(f)
    return ImportedDocument.open(f.getvalue())


def node(tag, **attrs):
    element = OxmlElement(f"w:{tag}")
    for key, value in attrs.items():
        element.set(qn(f"w:{key}"), value)
    return element


@pytest.mark.parametrize("missing_end", [False, True])
def test_complex_field_cannot_discard_opaque_cache_or_missing_end(missing_end):
    word = Document()
    p = word.add_paragraph()
    p.add_run()._r.append(node("fldChar", fldCharType="begin"))
    p.add_run()._r.append(node("instrText"))
    p.add_run()._r.append(node("fldChar", fldCharType="separate"))
    if missing_end:
        p.add_run("unterminated")
    else:
        p._p.append(node("bookmarkStart", id="0", name="Hidden"))
        p.add_run()._r.append(node("fldChar", fldCharType="end"))
    source = imported(word)
    with pytest.raises(ValueError):
        source.replace_field(source.body[0].anchor, 0)


def test_duplicate_comment_end_hidden_in_header_prevents_removal():
    word = Document()
    word.add_comment(word.add_paragraph().add_run("text"), text="keep", author="A")
    word.sections[0].header.paragraphs[0]._p.append(node("commentRangeEnd", id="+000"))
    with pytest.raises(ValueError, match="range markers"):
        imported(word).set_paired_range(0)


def test_new_bookmark_does_not_make_a_move_pair_ambiguous():
    word = Document()
    for text in ("source", "destination", "untouched"):
        word.add_paragraph(text)
    source = imported(word)
    first, second = [p.inline_structure[0].anchor for p in source.body[:2]]
    changed = ImportedDocument.open(source.preview_tracked_move(
        first, second, "MoveName", "A", "2025-01-01T00:00:00Z").to_docx())
    other = changed.body[2].inline_structure[0].anchor
    legs = [r for p in changed.body for r in p.inline_structure if getattr(r, "kind", "") in ("moveSource","moveDestination")]
    for identifier, name in [(int(legs[0].id), "Other"), (93, "movename")]:
        with pytest.raises(ValueError, match="collides"):
            changed.set_paired_range(identifier, name, (other, other))
    reviewed = ImportedDocument.open(changed.preview_revision(legs[0].anchor,"reject",paired=legs[1].anchor).to_docx())
    assert [p.text for p in reviewed.body] == ["source","destination","untouched"]


@pytest.mark.parametrize("cache", ["control", "revision"])
def test_field_cache_cannot_discard_a_control_or_revision(cache):
    word = Document()
    p = word.add_paragraph()
    field = node("fldSimple", instr=" PAGE ")
    p._p.append(field)
    if cache == "control":
        wrapper = node("sdt")
        content = node("sdtContent")
        wrapper.append(content)
    else:
        wrapper = content = node("ins", id="4", author="A")
    field.append(wrapper)
    content.append(p.add_run("protected")._r)
    source = imported(word)
    with pytest.raises(ValueError):
        source.replace_field(source.body[0].anchor, 0)


@pytest.mark.asyncio
async def test_bookmark_removal_rejects_link_but_retries_after_removing_it():
    word = Document()
    word.add_paragraph("Inside")
    word.add_paragraph("After")
    source = imported(word)
    inside = source.body[0].inline_structure[0].anchor
    marked = ImportedDocument.open(source.set_paired_range(4, "Target", (inside, inside)))
    after = marked.body[1].inline_structure[0].anchor
    missing = await d.Hyperlink(d.BookmarkTarget("Unknown"), [d.TextRun("broken")]).resolve()
    with pytest.raises(ValueError, match="bookmark target"):
        marked.preview([], inline_edits=[InlineEdit.insert_after(after, missing)])
    payload = await d.Hyperlink(d.BookmarkTarget("Target"), [d.TextRun("jump")]).resolve()
    linked = ImportedDocument.open(marked.preview([], inline_edits=[InlineEdit.insert_after(after, payload)]).to_docx())
    with pytest.raises(ValueError, match="referenced"):
        linked.set_paired_range(4, "Target")
    link = linked.body[1].inline_structure[1]
    assert link.bookmark == "Target" and link.relationship_id is None
    change = linked.preview([], inline_edits=[InlineEdit.hyperlink_history(link.anchor, False)])
    changed = ImportedDocument.open(change.to_docx())
    link = changed.body[1].inline_structure[1]
    assert link.history_xml == "0"
    unlinked = ImportedDocument.open(changed.preview([], inline_edits=[InlineEdit.remove_hyperlink(link.anchor)]).to_docx())
    plain = Document(io.BytesIO(unlinked.set_paired_range(4, "Target")))
    assert not plain.element.xpath("//w:bookmarkStart | //w:bookmarkEnd")
    assert [p.text for p in plain.paragraphs] == ["Inside", "After"]
