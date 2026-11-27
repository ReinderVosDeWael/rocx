"""Note reference lifecycle over the installed stdio binary."""

import json
import subprocess
from xml.etree import ElementTree

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.opc.part import Part
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import (
    anyio_backend, assert_tool_failure, call, digest, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio

NEW_TEXT = "New note 😀 & tail"
CREATE = dict(kind="create", id=13, text=NEW_TEXT)
CASES = {
    "create": ("endnote", 0, CREATE),
    "first-part": ("footnote", 0, CREATE),
    "empty-owner": ("footnote", 0, CREATE),
    "relocated-empty-footnote": ("footnote", 0, CREATE),
    "relocated-empty-endnote": ("endnote", 0, CREATE),
    "reserved-only": ("endnote", 0, CREATE),
    "attach": ("footnote", 3, dict(kind="attach")),
    "detach": ("footnote", 4, dict(kind="detach")),
    "delete": ("footnote", 4, dict(kind="delete")),
    "retarget": ("footnote", 4, dict(kind="retarget")),
    "renumber": ("endnote", 5, dict(kind="renumber", id=12)),
}


def w(tag, **attributes):
    element = OxmlElement(f"w:{tag}")
    for name, value in attributes.items():
        element.set(qn(f"w:{name}"), value)
    return element


def make_source(path, case="create", invalid_comments=False):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    first = source.add_paragraph()
    first.add_run("Insert here").bold = True
    hyperlink = w("hyperlink")
    hyperlink.set(qn("r:id"), source.part.relate_to(
        "https://example.com/", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    ))
    nested = w("r")
    label = w("t")
    label.text = "nested link"
    nested.append(label)
    hyperlink.append(nested)
    first._p.append(hyperlink)
    first.add_run()._r.append(w("sym", font="Symbol", char="F041"))
    first.add_run().add_tab()

    for kind in ("footnote", "endnote"):
        if case == "first-part" and kind == "footnote":
            continue
        plural = f"{kind}s"
        conventional_empty = case == "empty-owner" and kind == "footnote"
        empty = conventional_empty or case == f"relocated-empty-{kind}"
        reserved_only = case == "reserved-only" and kind == "endnote"
        owner = f"word/{plural}.xml" if conventional_empty else f"word/notes/{plural}.xml"
        root = w(plural)
        if not empty:
            separator = w(kind, id="-1", type="separator")
            p, r = w("p"), w("r")
            r.append(w("separator"))
            p.append(r)
            separator.append(p)
            root.append(separator)
        if not (empty or reserved_only):
            for ident in ("007", "0008"):
                entry = w(kind, id=ident)
                p, mark, run, text = w("p"), w("r"), w("r"), w("t")
                mark.append(w(f"{kind}Ref"))
                text.text = f"{kind} {ident}"
                run.append(text)
                p.extend((mark, run))
                entry.append(p)
                root.append(entry)
            first.add_run()._r.append(w(f"{kind}Reference", id="007"))
        source.part.relate_to(Part(
            PackURI(f"/{owner}"),
            f"application/vnd.openxmlformats-officedocument.wordprocessingml.{plural}+xml",
            etree.tostring(root, encoding="UTF-8", xml_declaration=True, standalone=True),
        ), getattr(RELATIONSHIP_TYPE, plural.upper()))

    if invalid_comments:
        # Well-formed XML attached by OPC; wrong document element for comments.
        source.part.relate_to(Part(
            PackURI("/word/comments.xml"),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
            etree.tostring(w("body")),
        ), RELATIONSHIP_TYPE.COMMENTS)

    source.add_table(1, 1).cell(0, 0).text = "not a direct body run"
    tail = source.add_paragraph("Keep tail")
    tail.add_run().add_break()
    tail.add_run()._r.append(w("cr"))
    source.sections[0].header.paragraphs[0].text = "Keep header"
    source.save(path)
    return path.read_bytes()


def operation(anchor, note_kind, edit):
    return dict(kind="edit-note-reference", anchor=anchor, note_kind=note_kind, edit=edit)


async def page(client, document, **view):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="note-references", **view))


def rows(result):
    return result["note_references"]


def oracle_bytes(case, source, output):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_note_oracle")
    subprocess.run([oracle, case, source, output], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


def assert_native_sites(result, source):
    actual = [{k: v for k, v in row.items() if k != "anchor"} for row in rows(result)]
    assert actual == json.loads(oracle_bytes("inspect", source, source.with_suffix(".sites.json")))
    return actual


async def test_note_reference_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
        inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
        preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])

        def view(**values):
            return dict(document="doc", view=dict(kind="note-references", **values))

        assert inspect.is_valid(view())
        assert inspect.is_valid(view(limit=100, cursor="next"))
        for values in [dict(limit=0), dict(limit=101), dict(cursor=None), dict(extra=1)]:
            assert not inspect.is_valid(view(**values))

        for kind, _, template in CASES.values():
            edit = dict(template)
            if edit["kind"] in ("attach", "retarget"):
                edit["id"] = 8
            scoped = operation("site", kind, edit)
            request = dict(document="doc", operation=scoped)
            assert preview.is_valid(request)
            assert not preview.is_valid(dict(request, edits=[dict(kind="run-remove", anchor="run")]))
            assert not preview.is_valid(dict(request, operation=dict(scoped, part_name="wrong")))
            assert not preview.is_valid(dict(request, operation=dict(scoped, note_kind="comment")))
            assert not preview.is_valid(dict(request, operation=dict(scoped, edit=dict(edit, extra=1))))
            for key in edit:
                missing = {field: value for field, value in edit.items() if field != key}
                assert not preview.is_valid(dict(request, operation=dict(scoped, edit=missing)))
            if "id" in edit:
                for invalid in (-1, 2**32, "007", 1.5, None):
                    bad_edit = dict(edit, id=invalid)
                    assert not preview.is_valid(dict(request, operation=dict(scoped, edit=bad_edit)))


async def test_note_reference_paging_and_indices(launch, tmp_path):
    source = tmp_path / "notes.docx"
    make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        first = await page(client, document, limit=2)
        cursor = first["next_cursor"]
        second = await page(client, document, limit=1, cursor=cursor)
        tail = await page(client, document, limit=100, cursor=second["next_cursor"])
        assert await page(client, document, limit=2) == first
        full = await page(client, document)
        assert tail["next_cursor"] is full["next_cursor"] is None
        assert rows(full) == rows(first) + rows(second) + rows(tail)
        assert len({row["anchor"] for row in rows(full)}) == len(rows(full))
        assert assert_native_sites(full, source) == [
            dict(kind="insertion-candidate", body_index=0, inline_index=0),
            dict(kind="insertion-candidate", body_index=0, inline_index=3),
            dict(kind="reference", body_index=0, inline_index=4, note_kind="footnote", id="007"),
            dict(kind="reference", body_index=0, inline_index=5, note_kind="endnote", id="007"),
            dict(kind="insertion-candidate", body_index=2, inline_index=0),
            dict(kind="insertion-candidate", body_index=2, inline_index=1),
            dict(kind="insertion-candidate", body_index=2, inline_index=2),
        ]
        other = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "wrong_owner", document=other,
                      view=dict(kind="note-references", cursor=cursor))
        entries = await call(client, "inspect_document", document=document,
                             view=dict(kind="entries", limit=1))
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="note-references", cursor=entries["next_cursor"]))
        await call(client, "close_document", document=document)
        await failure(client, "inspect_document", "invalid_document", document=document,
                      view=dict(kind="note-references", cursor=cursor))


def references(package, kind):
    document = ElementTree.fromstring(package["word/document.xml"])
    return [node.get(qn("w:id")) for node in document.iter(qn(f"w:{kind}Reference"))]


def definitions(package, owner):
    if owner not in package:
        return {}
    return {node.get(qn("w:id")): node for node in ElementTree.fromstring(package[owner])}


def assert_package_delta(source, output, case, kind):
    original, edited = parts(source), parts(output)
    plural = f"{kind}s"
    owner = (
        f"word/{plural}.xml" if case in ("first-part", "empty-owner")
        else f"word/notes/{plural}.xml"
    )
    if case == f"relocated-empty-{kind}":
        assert f"word/{plural}.xml" not in original
        assert len(ElementTree.fromstring(original[owner])) == 0
    added = {owner} if case == "first-part" else set()
    assert set(edited) == set(original) | added
    changed = {"word/document.xml"}
    if case == "first-part":
        changed |= {"[Content_Types].xml", "word/_rels/document.xml.rels"}
    elif case not in ("attach", "detach", "retarget"):
        changed.add(owner)
    assert {part for part in original if original[part] != edited[part]} == changed

    assert references(edited, kind) == {
        "create": ["13", "007"], "first-part": ["13"], "empty-owner": ["13"],
        "relocated-empty-footnote": ["13"], "relocated-empty-endnote": ["13"],
        "reserved-only": ["13"], "attach": ["8", "007"], "retarget": ["8"],
        "detach": [], "delete": [], "renumber": ["12"],
    }[case]
    opposite = "endnote" if kind == "footnote" else "footnote"
    assert references(edited, opposite) == references(original, opposite) == ["007"]
    old, new = definitions(original, owner), definitions(edited, owner)
    removed = {"007"} if case in ("delete", "renumber") else set()
    inserted = {"13"} if CASES[case][2]["kind"] == "create" else (
        {"12"} if case == "renumber" else set()
    )
    assert set(new) == (set(old) - removed) | inserted
    for ident in set(old) - removed:
        assert ElementTree.tostring(old[ident]) == ElementTree.tostring(new[ident])
    if inserted:
        ident, = inserted
        assert new[ident].find(f"./{qn('w:p')}/{qn('w:r')}/{qn(f'w:{kind}Ref')}") is not None
        assert [node.text for node in new[ident].iter(qn("w:t"))] == [
            "endnote 007" if case == "renumber" else NEW_TEXT,
        ]

    document = docx.Document(output)
    assert [p.text for p in document.paragraphs] == ["Insert herenested link\t", "Keep tail\n\n"]
    assert document.paragraphs[0].runs[0].bold is True
    assert document.paragraphs[0]._p.xpath("./w:r/w:sym/@w:char") == ["F041"]
    assert document.tables[0].cell(0, 0).text == "not a direct body run"
    assert document.sections[0].header.paragraphs[0].text == "Keep header"
    target = next(rel.target_part.partname for rel in document.part.rels.values()
                  if rel.reltype == getattr(RELATIONSHIP_TYPE, plural.upper()))
    assert target == f"/{owner}"


@pytest.mark.parametrize("case", CASES)
async def test_note_reference_native_bytes_and_preservation(launch, tmp_path, case):
    source, output, native = [tmp_path / name for name in ("source.docx", "copy.docx", "native.docx")]
    original = make_source(source, case)
    kind, inline_index, template = CASES[case]
    edit = dict(template)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        inspection = await page(client, document)  # No compact inspection.
        site = next(row for row in rows(inspection)
                    if row["body_index"] == 0 and row["inline_index"] == inline_index)
        if edit["kind"] in ("attach", "retarget"):
            entries = await call(client, "inspect_document", document=document,
                                 view=dict(kind="entries"))
            entry = next(row for row in entries["entries"]
                         if row["kind"] == kind and row["id"] == "0008")
            edit["id"] = int(entry["id"], 10)
        change = operation(site["anchor"], kind, edit)
        draft = await call(client, "preview_edits", document=document, operation=change)
        assert draft["document"] == document and draft["changes"] is None
        alternative = await call(
            client, "preview_edits", document=document,
            operation=operation(rows(inspection)[0]["anchor"], kind, dict(CREATE, id=14)),
        )
        assert draft["preview"] != alternative["preview"]
        assert draft["output_snapshot"] != alternative["output_snapshot"]
        assert list(tmp_path.iterdir()) == [source]
        saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())
        assert await page(client, document) == inspection
        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=change)
        updated = [r["id"] for r in assert_native_sites(await page(client, reopened), output)
                   if r.get("note_kind") == kind]
        assert updated == references(parts(output), kind)

    assert source.read_bytes() == original
    assert output.read_bytes() == oracle_bytes(case, source, native)
    assert_package_delta(source, output, case, kind)
    validate(source, output)


async def test_note_reference_rejection_and_retry(launch, tmp_path):
    source = tmp_path / "notes.docx"
    original = make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await page(client, document)
        insertion = rows(seen)[0]["anchor"]
        reference = rows(seen)[2]["anchor"]
        entries = await call(client, "inspect_document", document=document,
                             view=dict(kind="entries"))
        definition = next(row["anchor"] for row in entries["entries"] if row["id"] == "007")
        for change, category in [
            (operation(insertion, "footnote", dict(CREATE, id=7)), "rejected_batch"),
            (operation(insertion, "footnote", dict(kind="attach", id=7)), "rejected_batch"),
            (operation(reference, "endnote", dict(kind="detach")), "rejected_batch"),
            (operation(reference, "footnote", dict(kind="renumber", id=2**32)), "invalid_input"),
            (operation(definition, "footnote", dict(kind="delete")), "wrong_anchor_type"),
        ]:
            await failure(client, "preview_edits", category, document=document, operation=change)
        assert list(tmp_path.iterdir()) == [source]
        result = await client.call_tool("preview_edits", dict(document=document, operation=operation(
            reference, "footnote", dict(kind="retarget", id=99),
        )))
        assert_tool_failure(result, "rejected_batch")
        error = oracle_bytes("missing-target", source, tmp_path / "native-error.txt").decode()
        assert result.structuredContent["message"] == f"{error}; revise the operation and retry"
        assert await page(client, document) == seen
        draft = await call(client, "preview_edits", document=document,
                           operation=operation(reference, "footnote", dict(kind="retarget", id=8)))
        await call(client, "save_copy", document=document, preview=draft["preview"], output="retry.docx")
    assert source.read_bytes() == original
    assert_package_delta(source, tmp_path / "retry.docx", "retarget", "footnote")


async def test_note_reference_first_part_ignores_unrelated_entry_catalog_error(launch, tmp_path):
    source, output = [tmp_path / name for name in ("source.docx", "copy.docx")]
    make_source(source, "first-part", invalid_comments=True)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="entries"))
        insertion = rows(await page(client, document))[0]["anchor"]
        draft = await call(client, "preview_edits", document=document,
                           operation=operation(insertion, "footnote", CREATE))
        await call(client, "save_copy", document=document, preview=draft["preview"], output=output.name)
    assert output.read_bytes() == oracle_bytes("first-part", source, tmp_path / "native.docx")
    assert parts(output)["word/comments.xml"] == parts(source)["word/comments.xml"]
