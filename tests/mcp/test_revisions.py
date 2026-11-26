"""Installed SDK revision workflows with independent Rust and package readback."""

import json
import subprocess

import docx
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import (  # noqa: F401 (import shared pytest fixtures)
    anyio_backend, call, digest, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio

AUTHOR = "Ada & Co."
DATE = "2030-02-03T04:05:06Z"
TEXT = "New 😀 & text"
MAIN = "word/document.xml"
TAIL = ["Target", "Tail"]

PROPOSALS = {
    "insert-text-after": dict(text=TEXT),
    "replace-text": dict(text=TEXT, date=DATE),
    "delete-text": {},
    "move-text": dict(name="Move1", date=DATE),
    "set-bold": dict(bold=False, date=DATE),
    "insert-paragraph-before": dict(text=TEXT, date=DATE),
    "delete-paragraph": {},
}
PARAGRAPH = {"insert-paragraph-before", "delete-paragraph"}
BASE = ["Source", *TAIL]


def element(name, **attributes):
    node = OxmlElement(f"w:{name}")
    for key, value in attributes.items():
        node.set(qn(f"w:{key}"), value)
    return node


def make_revision_source(path, rich=False):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.sections[0].header.paragraphs[0].text = "Protected header payload"
    document.add_table(1, 1).cell(0, 0).text = "Not a direct body paragraph"
    target = document.add_paragraph("Source")
    document.add_paragraph("Target")
    document.add_paragraph("Tail")
    if rich:
        wrapper = element("ins", id="005", author="Café")
        child = element("r")
        wrapper.append(child)
        leaf = element("t")
        leaf.text = "Pending"
        child.append(leaf)
        child.append(element("sym", font="Symbol", char="F041"))
        target._p.append(wrapper)
        link = element("hyperlink", anchor="NotResolved")
        nested = element("del", id="006", author="Someone", date=DATE)
        link.append(nested)
        child = element("r")
        nested.append(child)
        leaf = element("delText")
        leaf.text = "Nested"
        child.append(leaf)
        target._p.append(link)
    document.save(path)
    return path.read_bytes()


def operation(anchor, edit, paired=None):
    value = dict(kind="edit-revision", anchor=anchor, edit=edit)
    if paired is not None:
        value["paired_anchor"] = paired
    return value


async def opened(client, path):
    return (await call(client, "open_document", path=path.name))["document"]


async def revision_page(client, document, **view):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="revisions", **view))


def oracle(output, command, *args):
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_revision_oracle")
    subprocess.run([binary, command, output, *args], check=True, cwd=output.parent, timeout=20)
    return output.read_bytes()


async def compare_preview(client, document, request, output, native_command, source, *args):
    staged = await call(client, "preview_edits", document=document, operation=request)
    assert isinstance(staged["changes"], dict)
    await call(client, "save_copy", document=document, preview=staged["preview"], output=output.name)
    data = output.read_bytes()
    assert staged["output_snapshot"] == digest(data)
    assert data == oracle(output.with_suffix(".native.docx"), native_command, source, *args)
    before, after = parts(source), parts(output)
    assert set(before) == set(after)
    assert all(after[name] == contents for name, contents in before.items() if name != MAIN)
    validate(output)
    return etree.fromstring(after[MAIN])


async def test_revision_discovery(launch):
    async with launch() as client:
        tools = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}

    def schema(tool):
        value = tools[tool]
        return validator_for(value)(value)

    inspect = schema("inspect_document")
    for view in [dict(kind="revisions"), dict(kind="revisions", limit=1, cursor="page")]:
        assert inspect.is_valid(dict(document="doc", view=view))
    for extra in [dict(limit=0), dict(limit=101), dict(cursor=None), dict(extra=True)]:
        assert not inspect.is_valid(dict(document="doc", view=dict(kind="revisions", **extra)))

    preview = schema("preview_edits")
    edits = [dict(kind=kind, author=AUTHOR, **data) for kind, data in PROPOSALS.items()]
    edits += [dict(kind="review", action="accept"), dict(kind="review-paragraph", action="reject")]
    for edit in edits:
        scoped = operation("anchor", edit)
        request = dict(document="doc", operation=scoped)
        assert preview.is_valid(request), edit
        assert not preview.is_valid(dict(request, edits=[dict(kind="run-remove", anchor="run")]))
        assert not preview.is_valid(dict(request, operation=dict(scoped, paired_anchor=None)))
        assert not preview.is_valid(dict(request, operation=operation("a", dict(edit, extra=1))))
        for field in set(edit) - ({"date"} if edit["kind"] != "move-text" else set()):
            incomplete = {key: value for key, value in edit.items() if key != field}
            assert not preview.is_valid(dict(request, operation=operation("anchor", incomplete)))
    assert preview.is_valid(dict(document="doc", operation=operation("a", edits[3], "b")))
    for edit in [dict(kind="review", action="maybe"), dict(kind="delete-text", author="a", date=None)]:
        assert not preview.is_valid(dict(document="doc", operation=operation("a", edit)))


def assert_move_markup(root, rows):
    ids = set()
    for suffix, row in zip(("From", "To"), rows, strict=True):
        start, = root.iter(qn(f"w:move{suffix}RangeStart"))
        end, = root.iter(qn(f"w:move{suffix}RangeEnd"))
        wrapper, = root.iter(qn(f"w:move{suffix}"))
        range_id = start.get(qn("w:id"))
        assert start.get(qn("w:name")) == "Move1"
        assert range_id == end.get(qn("w:id"))
        assert start.getnext() is wrapper and wrapper.getnext() is end
        assert wrapper.get(qn("w:id")) == row["id"]
        ids.update((range_id, row["id"]))
    assert len(ids) == 4


def assert_native_metadata(rows, source):
    native = json.loads(oracle(source.with_suffix(".native.json"), "inspect", source))
    fields = ("body_index", "inline_path", "kind", "id", "author", "date")
    assert [{field: row[field] for field in fields} for row in rows] == native


@pytest.mark.parametrize("kind", list(PROPOSALS))
async def test_revision_native_proposal_and_both_dispositions(launch, tmp_path, kind):
    source = tmp_path / "source.docx"
    original = make_revision_source(source)
    proposal = tmp_path / "proposal.docx"
    async with launch() as client:
        document = await opened(client, source)
        body = (await call(client, "inspect_document", document=document))["body"]
        paragraph = body[1]["anchor"]
        run = body[1]["runs"][0]["anchor"]
        after = body[2]["runs"][0]["anchor"]
        paired = after if kind == "move-text" else None
        target = paragraph if kind in PARAGRAPH else run
        edit = dict(kind=kind, author=AUTHOR, **PROPOSALS[kind])
        root = await compare_preview(
            client, document, operation(target, edit, paired), proposal, kind, source,
        )
        assert source.read_bytes() == original
        proposed = await opened(client, proposal)
        rows = (await revision_page(client, proposed))["revisions"]
        assert_native_metadata(rows, proposal)
        assert rows and all(row["author"] == AUTHOR and row["date"] == edit.get("date")
                            for row in rows)
        assert len({row["id"] for row in rows}) == len(rows)

        if kind in PARAGRAPH:
            mark, wrapper = rows
            native_kind = "insertion" if kind.startswith("insert") else "deletion"
            assert (mark["type"], mark["kind"], mark["body_index"], mark["inline_path"]) == (
                "paragraph-mark", native_kind, 1, [],
            )
            assert (wrapper["type"], wrapper["kind"], wrapper["inline_path"]) == (
                "inline-wrapper", native_kind, [0],
            )
            tag = qn("w:ins" if native_kind == "insertion" else "w:del")
            assert {node.get(qn("w:id")) for node in root.iter(tag)} == {mark["id"], wrapper["id"]}
            assert wrapper["children"][0]["text"] == (TEXT if native_kind == "insertion" else "Source")
            accepted = [TEXT, *BASE] if native_kind == "insertion" else TAIL
            rejected = BASE
            cases = [(mark, None, accepted, rejected)]
            if kind == "insert-paragraph-before":
                blocked = docx.Document(proposal)
                paragraph = blocked.paragraphs[0]
                paragraph._p.addnext(blocked.tables[0]._tbl)
                blocked_path = tmp_path / "protected-successor.docx"
                blocked.save(blocked_path)
                blocked_doc = await opened(client, blocked_path)
                blocked_mark = (await revision_page(client, blocked_doc))["revisions"][0]
                await failure(client, "preview_edits", "rejected_batch", document=blocked_doc,
                              operation=operation(blocked_mark["anchor"],
                                                  dict(kind="review-paragraph", action="reject")))
        elif kind == "set-bold":
            row, = rows
            assert (row["type"], row["kind"], row["inline_path"]) == (
                "run-properties", "runProperties", [0],
            )
            # The native inspector marks pending run properties opaque instead of
            # projecting the unreconciled direct toggle. Read it separately.
            assert row["run"]["bold"] is None and row["run"]["has_opaque_formatting"]
            assert row["run"]["text"] == "Source"
            assert docx.Document(proposal).paragraphs[0].runs[0].bold is False
            assert next(root.iter(qn("w:rPrChange"))).get(qn("w:id")) == row["id"]
            cases = [(row, None, BASE, BASE)]
        elif kind == "move-text":
            source_row, destination_row = rows
            assert [(row["kind"], row["body_index"], row["inline_path"]) for row in rows] == [
                ("moveSource", 1, [1]), ("moveDestination", 2, [2]),
            ]
            assert [row["children"][0]["text"] for row in rows] == ["Source", "Source"]
            assert_move_markup(root, rows)
            # Native review accepts either leg first.
            cases = [(destination_row, source_row, ["", "TargetSource", "Tail"], BASE)]
        else:
            expect = {
                "insert-text-after": [("insertion", [1], TEXT, ["Source" + TEXT, *TAIL], BASE)],
                "replace-text": [
                    ("deletion", [0], "Source", ["", *TAIL], BASE),
                    ("insertion", [1], TEXT, [TEXT, *TAIL], ["", *TAIL]),
                ],
                "delete-text": [("deletion", [0], "Source", ["", *TAIL], BASE)],
            }[kind]
            cases = []
            for row, (native_kind, path, text, accepted, rejected) in zip(rows, expect, strict=True):
                assert (row["type"], row["kind"], row["body_index"], row["inline_path"]) == (
                    "inline-wrapper", native_kind, 1, path,
                )
                child, = row["children"]
                assert child["text"] == text and not child["has_opaque_content"]
                assert "anchor" not in child
                cases.append((row, None, accepted, rejected))

        for index, (row, second, accepted, rejected) in enumerate(cases):
            for action, expected in [("accept", accepted), ("reject", rejected)]:
                review = "review-paragraph" if kind in PARAGRAPH else "review"
                second_anchor = second["anchor"] if second else None
                edit = operation(row["anchor"], dict(kind=review, action=action), second_anchor)
                coords = [str(row["body_index"]), *map(str, row["inline_path"])]
                if second:
                    coords += [str(second["body_index"]), *map(str, second["inline_path"])]
                output = tmp_path / f"{index}-{action}.docx"
                await compare_preview(client, proposed, edit, output, review, proposal, action, *coords)
                result = docx.Document(output)
                assert [p.text for p in result.paragraphs] == expected
                if kind == "set-bold":
                    assert result.paragraphs[0].runs[0].bold is (False if action == "accept" else None)
                reopened = await opened(client, output)
                remaining = (await revision_page(client, reopened))["revisions"]
                assert [entry["id"] for entry in remaining] == (
                    [entry["id"] for entry in rows if entry["id"] != row["id"]]
                    if kind == "replace-text" else []
                )
                await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=edit)
                await call(client, "close_document", document=reopened)


async def test_revision_view_paging_read_only_and_opaque(launch, tmp_path):
    source = tmp_path / "rich.docx"
    make_revision_source(source, rich=True)
    async with launch() as client:
        document = await opened(client, source)
        other = await opened(client, source)
        first = await revision_page(client, document, limit=1)
        cursor = first["next_cursor"]
        direct, = first["revisions"]
        assert direct["inline_path"] == [1] and direct["id"] == "005" and direct["date"] is None
        child, = direct["children"]
        assert child["has_opaque_content"] and child["text"] == "Pending"
        assert child["items"] == [dict(type="text", text="Pending"), dict(type="opaque", name="w:sym")]
        rest = await revision_page(client, document, limit=1, cursor=cursor)
        nested, = rest["revisions"]
        assert rest["next_cursor"] is None and "anchor" not in nested
        assert nested["inline_path"] == [2, 0] and nested["id"] == "006"
        assert nested["children"][0]["text"] == "Nested"
        assert (await revision_page(client, document))["revisions"] == [direct, nested]
        await failure(client, "inspect_document", "wrong_owner", document=other,
                      view=dict(kind="revisions", cursor=cursor))
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="entries", cursor=cursor))
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=operation(direct["anchor"], dict(kind="review", action="accept")))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[dict(kind="run-remove", anchor=direct["anchor"])])


async def test_revision_wrong_scope_pairing_and_retry(launch, tmp_path):
    source = tmp_path / "source.docx"
    original = make_revision_source(source)
    async with launch() as client:
        document = await opened(client, source)
        other = await opened(client, source)
        body = (await call(client, "inspect_document", document=document))["body"]
        foreign = (await call(client, "inspect_document", document=other))["body"][2]["runs"][0]["anchor"]
        source_run = body[1]["runs"][0]["anchor"]
        after = body[2]["runs"][0]["anchor"]
        move = dict(kind="move-text", name="Move1", author=AUTHOR, date=DATE)
        remove = dict(kind="delete-text", author=AUTHOR)
        for category, request in [
            ("wrong_anchor_type", operation(body[0]["cells"][0]["anchor"], remove)),
            ("wrong_anchor_type", operation(body[1]["anchor"], remove)),
            ("wrong_anchor_type", operation(source_run, move, body[1]["anchor"])),
            ("wrong_owner", operation(source_run, move, foreign)),
            ("rejected_batch", operation(source_run, move)),
            ("rejected_batch", operation(source_run, move, source_run)),
            ("rejected_batch", operation(source_run, remove, after)),
            ("rejected_batch", operation(source_run, dict(kind="replace-text", text="", author=AUTHOR))),
            ("rejected_batch", operation(source_run, dict(kind="set-bold", bold=True, author=" "))),
            ("rejected_batch", operation(body[1]["anchor"], dict(
                kind="insert-paragraph-before", text="two\nlines", author=AUTHOR,
            ))),
            ("rejected_batch", operation(body[3]["anchor"], dict(kind="delete-paragraph", author=AUTHOR))),
        ]:
            await failure(client, "preview_edits", category, document=document, operation=request)
        await failure(client, "preview_edits", "invalid_input", document=document,
                      operation=operation(source_run, remove),
                      edits=[dict(kind="run-remove", anchor=source_run)])
        assert list(tmp_path.iterdir()) == [source] and source.read_bytes() == original

        noop = tmp_path / "noop.docx"
        unchanged = dict(kind="replace-text", author=AUTHOR, text="Source")
        await compare_preview(client, document, operation(source_run, unchanged), noop,
                              "replace-identical", source)
        assert parts(noop) == parts(source)

        proposal = tmp_path / "move.docx"
        await compare_preview(client, document, operation(source_run, move, after), proposal,
                              "move-text", source)
        proposed = await opened(client, proposal)
        a, b = (await revision_page(client, proposed))["revisions"]
        review = dict(kind="review", action="accept")
        for category, request in [
            ("rejected_batch", operation(a["anchor"], review)),
            ("rejected_batch", operation(a["anchor"], review, a["anchor"])),
            ("wrong_owner", operation(a["anchor"], review, source_run)),
        ]:
            await failure(client, "preview_edits", category, document=proposed, operation=request)
        await compare_preview(client, proposed, operation(a["anchor"], review, b["anchor"]),
                              tmp_path / "accepted.docx", "review", proposal, "accept", "1", "1", "2", "2")
