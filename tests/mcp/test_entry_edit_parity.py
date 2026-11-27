"""Standalone EntryEdit parity at the installed MCP boundary."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.opc.part import Part
from docx.oxml.ns import qn
from lxml import etree
import pytest

from test_notes import w
from test_stdio import (
    anyio_backend, assert_tool_failure, call, digest, entry_key, entry_operation,
    entry_page, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio

TEXT = " New 😀 & text "
COMMENT = ("comment", "0")
UNUSED_ENDNOTE = ("endnote", "0008")
ENDNOTES = "word/notes/endnotes.xml"
COMMENTS = "word/comments.xml"
EXTENDED_COMMENTS = "word/commentsExtended.xml"
DELETE = dict(kind="delete-unused-note")
CASES = {
    "insert-comment": (COMMENT, dict(kind="insert-paragraph", index=1, text=TEXT)),
    "replace-comment": (COMMENT, dict(kind="replace-paragraph", index=0, text=TEXT)),
    "remove-comment": (COMMENT, dict(kind="remove-paragraph", index=1)),
    "delete-endnote": (UNUSED_ENDNOTE, DELETE),
}
GUARDS = {
    "last-block": (("comment", "1"), dict(kind="remove-paragraph", index=0),
                   "cannot remove the last entry paragraph"),
    "marked-block": (COMMENT, dict(kind="remove-paragraph", index=0),
                     "protected first note paragraph"),
    "rich-block": (COMMENT, dict(kind="replace-paragraph", index=2, text=TEXT),
                   "cannot discard rich or opaque entry paragraph"),
    "protected-gap": (("comment", "2"), dict(kind="insert-paragraph", index=1, text=TEXT),
                      "entry block intersects protected field or range"),
    "rich-note": (("endnote", "0009"), DELETE, "expected a discardable ordinary note"),
    "live-note": (("endnote", "007"), DELETE, "already referenced"),
}


def add_complex_field(source):
    comment = source.comments.add_comment(text="Field start", author="Field")
    comment._element.set(qn("w:date"), "2025-01-02T03:04:05Z")
    start = comment.paragraphs[0]
    start.add_run()._r.append(w("fldChar", fldCharType="begin"))
    instruction = w("instrText")
    instruction.text = " QUOTE cached "
    instruction.set(qn("xml:space"), "preserve")
    start.add_run()._r.append(instruction)
    start.add_run()._r.append(w("fldChar", fldCharType="separate"))
    end = comment.add_paragraph("cached")
    end.add_run()._r.append(w("fldChar", fldCharType="end"))


def add_extended_comments(source, comment):
    namespace = "http://schemas.microsoft.com/office/word/2012/wordml"
    para_id = "00000042"
    comment.paragraphs[-1]._p.set(qn("w14:paraId"), para_id)
    root = etree.Element(f"{{{namespace}}}commentsEx", nsmap={"w15": namespace})
    etree.SubElement(root, f"{{{namespace}}}commentEx", attrib={
        f"{{{namespace}}}paraId": para_id, f"{{{namespace}}}done": "0",
    })
    source.part.relate_to(Part(
        PackURI(f"/{EXTENDED_COMMENTS}"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml",
        etree.tostring(root, encoding="UTF-8", xml_declaration=True, standalone=True),
    ), "http://schemas.microsoft.com/office/2011/relationships/commentsExtended")


def make_source(path, variant=None):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    body = source.add_paragraph("Body unchanged")
    body.add_run()._r.append(w("endnoteReference", id="007"))

    comment = source.add_comment(body.runs[0], text="First", author="Original", initials="OO")
    # Only the first marked block has the specially retained CommentText style.
    comment.add_paragraph("Removable", style="Normal")
    comment.add_paragraph("Opaque", style="Normal").add_run()._r.append(
        w("sym", font="Symbol", char="F041"),
    )

    single = source.comments.add_comment(text="Single block", author="Single")
    # Remove the generated style and mark so the last-block guard is reached.
    (p,) = single.paragraphs
    for node in p._p.xpath("./w:pPr | ./w:r[w:annotationRef]"):
        p._p.remove(node)

    ranged = source.comments.add_comment(text="Range start", author="Range")
    ranged.paragraphs[0]._p.append(w("bookmarkStart", id="31", name="EntryRange"))
    ranged.add_paragraph("Range end")._p.append(w("bookmarkEnd", id="31"))
    for definition in (comment, single, ranged):
        definition._element.set(qn("w:date"), "2025-01-02T03:04:05Z")
    if variant == "field":
        add_complex_field(source)
    elif variant == "extended":
        add_extended_comments(source, comment)

    notes = w("endnotes")
    definitions = [("-1", "separator"), ("007", "endnoteRef"),
                   ("0008", "endnoteRef"), ("0009", "endnoteRef")]
    if variant == "aliases":
        definitions.append(("8", "endnoteRef"))
    for ident, mark in definitions:
        note, p, marker = w("endnote", id=ident), w("p"), w("r")
        marker.append(w(mark))
        p.append(marker)
        if mark == "separator":
            note.set(qn("w:type"), mark)
        else:
            run, text = w("r"), w("t")
            text.text = f"Endnote {ident}"
            run.append(text)
            if ident == "0009":
                run.append(w("sym", font="Symbol", char="F041"))
            p.append(run)
        note.append(p)
        notes.append(note)
    source.part.relate_to(Part(
        PackURI(f"/{ENDNOTES}"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml",
        etree.tostring(notes, encoding="UTF-8", xml_declaration=True, standalone=True),
    ), RELATIONSHIP_TYPE.ENDNOTES)
    source.save(path)
    return path.read_bytes()


def oracle(case, source, output):
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_entry_edit_parity_oracle")
    return subprocess.run(
        [binary, case, source, output], check=True, cwd=source.parent, timeout=20,
        capture_output=True, text=True,
    ).stdout.strip()


def assert_entry_part_delta(source, output, target, *, deleted=False):
    kind, ident = target
    owner = ENDNOTES if kind == "endnote" else COMMENTS
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {owner}
    old, new = [
        {entry.get(qn("w:id")): entry for entry in ElementTree.fromstring(package[owner])}
        for package in (before, after)
    ]
    removed = {ident} if deleted else set()
    assert list(new) == [key for key in old if key not in removed]
    for untouched in old.keys() - {ident}:
        assert ElementTree.tostring(new[untouched]) == ElementTree.tostring(old[untouched])
    return old[ident], new.get(ident)


def assert_package_delta(source, output, case):
    target, _ = CASES[case]
    old, new = assert_entry_part_delta(source, output, target, deleted=case == "delete-endnote")
    document = docx.Document(output)
    assert [p.text for p in document.paragraphs] == ["Body unchanged"]
    if case == "delete-endnote":
        return
    comment = document.comments.get(0)
    expected = {
        "insert-comment": ["First", TEXT, "Removable", "Opaque"],
        "replace-comment": [TEXT, "Removable", "Opaque"],
        "remove-comment": ["First", "Opaque"],
    }[case]
    assert [p.text for p in comment.paragraphs] == expected
    assert old.attrib == new.attrib
    previous, edited = list(old), list(new)
    if case == "insert-comment":
        assert edited[1].find(f"./{qn('w:r')}/{qn('w:t')}").get(qn("xml:space")) == "preserve"
        del edited[1]
    elif case == "replace-comment":
        # The annotation run, its formatting, and the first paragraph style survive.
        assert edited[0][0].find(qn("w:pStyle")).get(qn("w:val")) == "CommentText"
        assert edited[0][1].find(qn("w:annotationRef")) is not None
        assert [ElementTree.tostring(n) for n in edited[0][:2]] == [
            ElementTree.tostring(n) for n in previous[0][:2]
        ]
        del previous[0], edited[0]
    else:
        del previous[1]
    assert [ElementTree.tostring(n) for n in edited] == [
        ElementTree.tostring(n) for n in previous
    ]


async def save_edit(client, document, operation, output):
    draft = await call(client, "preview_edits", document=document, operation=operation)
    assert draft["document"] == document and draft["changes"] is None
    assert not output.exists()
    saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                       output=output.name)
    assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())


async def assert_native_guard(client, document, anchor, edit, case, reason, source, native):
    operation = entry_operation(anchor, edit)
    result = await client.call_tool("preview_edits", dict(document=document, operation=operation))
    assert_tool_failure(result, "rejected_batch")
    native_error = oracle(case, source, native)
    assert reason in native_error, (case, native_error)
    assert native_error in result.structuredContent["message"], (case, result)


@pytest.mark.parametrize("case", CASES)
async def test_entry_edit_native_bytes_and_preservation(launch, tmp_path, case):
    source, output, native = [tmp_path / name for name in ("source.docx", "copy.docx", "native.docx")]
    original = make_source(source)
    target, edit = CASES[case]
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await entry_page(client, document)
        entry = next(e for e in seen["entries"] if entry_key(e) == target)
        assert entry["part_name"] == (ENDNOTES if target == UNUSED_ENDNOTE else COMMENTS)
        operation = entry_operation(entry["anchor"], edit)
        await save_edit(client, document, operation, output)
        assert await entry_page(client, document) == seen
        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=operation)
        updated = {entry_key(e): e for e in (await entry_page(client, reopened))["entries"]}
        if case == "delete-endnote":
            assert target not in updated
        else:
            assert len(updated[target]["blocks"]) == len(entry["blocks"]) + {
                "insert-comment": 1, "replace-comment": 0, "remove-comment": -1,
            }[case]

    oracle(case, source, native)
    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    assert_package_delta(source, output, case)
    validate(source, output)


async def test_entry_edit_guard_diagnostics_and_retry(launch, tmp_path):
    source, output, native = [tmp_path / name for name in ("source.docx", "retry.docx", "native.docx")]
    original = make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await entry_page(client, document)
        anchors = {entry_key(e): e["anchor"] for e in seen["entries"] if "anchor" in e}
        for case, (target, edit, reason) in GUARDS.items():
            await assert_native_guard(client, document, anchors[target], edit, case, reason,
                                      source, native)
        assert list(tmp_path.iterdir()) == [source]
        assert source.read_bytes() == original
        assert await entry_page(client, document) == seen

        # Retry on the same issued anchor after the rejected destructive edits.
        case = "replace-comment"
        target, edit = CASES[case]
        await save_edit(client, document, entry_operation(anchors[target], edit), output)
    oracle(case, source, native)
    assert output.read_bytes() == native.read_bytes()
    assert source.read_bytes() == original
    assert_package_delta(source, output, case)


@pytest.mark.parametrize("variant", ("field", "aliases", "extended"))
async def test_entry_edit_additional_native_guards_and_retry(launch, tmp_path, variant):
    source, output, native = [tmp_path / name for name in ("source.docx", "retry.docx", "native.docx")]
    original = make_source(source, variant)
    field = ("comment", "3")
    insert = dict(kind="insert-paragraph", index=1, text=TEXT)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await entry_page(client, document)
        entries = {entry_key(e): e for e in seen["entries"]}
        if variant == "field":
            rejected = [("field-gap", field, insert, "entry block intersects protected field or range")]
            retry_case, target, edit = "after-field", field, dict(insert, index=2)
        elif variant == "aliases":
            aliases = [entries["endnote", ident] for ident in ("0008", "8")]
            assert {e["part_name"] for e in aliases} == {ENDNOTES}
            assert len({e["anchor"] for e in aliases}) == 2
            rejected = [
                (f"alias-{e['id']}", entry_key(e), DELETE, "duplicate or reserved entry ID")
                for e in aliases
            ]
            retry_case = "replace-comment"
            target, edit = CASES[retry_case]
        else:
            rejected = [("extended-author", COMMENT, dict(kind="comment-author", value="Updated"),
                         "extended comment metadata or threading is unsupported")]
            retry_case = "delete-endnote"
            target, edit = CASES[retry_case]
        for case, key, bad_edit, reason in rejected:
            await assert_native_guard(client, document, entries[key]["anchor"], bad_edit,
                                      case, reason, source, native)
        assert list(tmp_path.iterdir()) == [source]
        assert source.read_bytes() == original
        assert await entry_page(client, document) == seen
        await save_edit(client, document, entry_operation(entries[target]["anchor"], edit), output)

    oracle(retry_case, source, native)
    assert output.read_bytes() == native.read_bytes()
    assert source.read_bytes() == original
    if variant == "field":
        old, new = assert_entry_part_delta(source, output, field)
        assert old.attrib == new.attrib
        assert [ElementTree.tostring(block) for block in old] == [
            ElementTree.tostring(block) for block in new[:-1]
        ]
        assert docx.Document(output).comments.get(3).paragraphs[-1].text == TEXT
        validate(source, output)
    else:
        assert_package_delta(source, output, retry_case)
