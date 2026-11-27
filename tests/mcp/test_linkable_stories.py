"""Installed stdio discovery and relinking of unreferenced story parts."""

from xml.etree import ElementTree
from zipfile import ZipFile

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, assert_tool_failure, call, failure, launch, parts, validate
from test_stories import MAIN, open_document, save_preview, story_page, story_source


pytestmark = pytest.mark.anyio
OTHER_FOOTER = "word/custom/unused-footer.xml"
OVERRIDE = "word/footer2.xml"


def selector(story_kind, **options):
    return dict(kind="linkable-stories", story_kind=story_kind, **options)


async def linkable_page(client, document, story_kind, **options):
    page = await call(
        client, "inspect_document", document=document, view=selector(story_kind, **options),
    )
    assert page["document"] == document
    return page["linkable_stories"], page["next_cursor"]


async def test_linkable_story_schema(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    schema = schemas["inspect_document"]
    inspect = validator_for(schema)(schema)
    for kind in ("header", "footer"):
        assert inspect.is_valid(dict(document="doc", view=selector(kind, limit=1, cursor="next")))
        for include_referenced in (True, False):
            assert inspect.is_valid(dict(document="doc", view=selector(
                kind, include_referenced=include_referenced,
            )))
    for view in (
        dict(kind="linkable-stories"),
        selector("body"),
        selector("header", limit=0),
        selector("footer", limit=101),
        selector("footer", cursor=None),
        selector("header", include_referenced=None),
        selector("header", include_referenced="true"),
    ):
        assert not inspect.is_valid(dict(document="doc", view=view))


async def test_relink_story_without_effective_slot(launch, story_source):
    fixture = docx.Document(story_source)
    extra, _ = fixture.part.add_footer_part()
    extra.partname = PackURI(f"/{OTHER_FOOTER}")
    # A malformed later candidate cannot hide an earlier linkable part.
    fixture.part.relate_to("https://example.test/external-footer", RELATIONSHIP_TYPE.FOOTER,
                          is_external=True)
    fixture.save(story_source)
    source = story_source.read_bytes()

    async with launch() as client:
        initial = await open_document(client, story_source)
        original_slots, _ = await story_page(client, initial)
        assert original_slots[9]["owner"]["part_name"] == OVERRIDE
        assert await linkable_page(client, initial, "header") == ([], None)
        initial_candidates, cursor = await linkable_page(client, initial, "footer")
        assert cursor is None
        assert [(row["kind"], row["part_name"]) for row in initial_candidates] == [
            ("footer", OTHER_FOOTER),
        ]
        removed = await save_preview(client, initial, story_source, "remove", {
            "kind": "edit-story-reference", "anchor": original_slots[9]["anchor"],
            "edit": {"kind": "remove"},
        })
        reopened = await open_document(client, removed)
        slots, _ = await story_page(client, reopened)
        assert slots[9]["inherited"] and slots[9]["owner"] == slots[3]["owner"]

        first, cursor = await linkable_page(client, reopened, "footer", limit=1)
        assert [row["part_name"] for row in first] == [OTHER_FOOTER]
        assert cursor is not None
        for document, kind, error in (
            (reopened, "header", "invalid_cursor"),
            (initial, "footer", "wrong_owner"),
        ):
            await failure(client, "inspect_document", error, document=document,
                          view=selector(kind, cursor=cursor))
        second, end = await linkable_page(client, reopened, "footer", cursor=cursor)
        assert end is None
        candidate, = second
        assert (candidate["kind"], candidate["part_name"]) == ("footer", OVERRIDE)

        operation = dict(kind="edit-story-reference", anchor=slots[9]["anchor"],
                         edit=dict(kind="link", anchor=candidate["anchor"]))
        await failure(client, "preview_edits", "rejected_batch", document=reopened,
                      operation=dict(operation, anchor=slots[8]["anchor"]))
        await failure(client, "preview_edits", "wrong_owner", document=reopened,
                      operation=dict(operation, edit=dict(kind="link",
                                                          anchor=initial_candidates[0]["anchor"])))
        await failure(client, "preview_edits", "invalid_anchor", document=reopened,
                      operation=dict(operation, edit=dict(kind="link", anchor=f"{reopened}/{OVERRIDE}")))
        output = await save_preview(client, reopened, removed, "relink", operation)

        relinked = await open_document(client, output)
        final_slots, _ = await story_page(client, relinked)
        assert not final_slots[9]["inherited"] and final_slots[9]["owner"]["part_name"] == OVERRIDE
        final_candidates, cursor = await linkable_page(client, relinked, "footer")
        assert cursor is None and [row["part_name"] for row in final_candidates] == [OTHER_FOOTER]

    assert story_source.read_bytes() == source
    before, after = parts(removed), parts(output)
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {MAIN}
    assert docx.Document(output).sections[1].footer.paragraphs[0].text == "Explicit footer override"
    validate(output)


async def test_link_default_header_when_unrelated_footer_is_malformed(launch, story_source):
    content = parts(story_source)
    malformed = ElementTree.fromstring(content[OVERRIDE])
    malformed.tag = qn("w:hdr")
    content[OVERRIDE] = ElementTree.tostring(malformed)
    with ZipFile(story_source, "w") as archive:
        for name, payload in content.items():
            archive.writestr(name, payload)
    original = story_source.read_bytes()

    async with launch() as client:
        document = await open_document(client, story_source)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view={"kind": "stories"})
        headers = await call(client, "inspect_document", document=document,
                             view={"kind": "default-headers"})
        sections = await call(client, "inspect_document", document=document,
                              view={"kind": "sections"})
        slots = sections["sections"][1]["story_slots"]
        header, = [slot for slot in slots if (slot["kind"], slot["variant"]) == ("header", "even")]
        footer, = [slot for slot in slots if (slot["kind"], slot["variant"]) == ("footer", "even")]
        owner = headers["default_headers"][0]["owner"]["anchor"]
        operation = dict(kind="edit-story-reference", anchor=header["anchor"],
                         edit=dict(kind="link", anchor=owner))
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=dict(operation, anchor=footer["anchor"]))
        output = await save_preview(client, document, story_source, "link-default-only", operation)

    assert story_source.read_bytes() == original
    saved_parts = parts(output)
    assert saved_parts.keys() == content.keys()
    assert {name for name in content if content[name] != saved_parts[name]} == {MAIN}


async def test_link_referenced_first_header_without_reading_unrelated_footer(launch, tmp_path):
    source = tmp_path / "referenced-first-header.docx"
    fixture = docx.Document()
    first = fixture.sections[0]
    first.first_page_header.paragraphs[0].text = "First-page header"
    first.footer.paragraphs[0].text = "Wrong-root footer"
    fixture.add_section()
    # This reference is readable, but rewriting it must hit the native guard.
    reference = fixture.sections[0]._sectPr.find(qn("w:headerReference"))
    reference.set(qn("w:rsidR"), "00000001")
    first_header = "word/header1.xml"
    wrong_header, _ = fixture.part.add_header_part()
    unused, _ = fixture.part.add_header_part()
    unused.partname = PackURI("/word/custom/unused-header.xml")
    fixture.part.relate_to("https://example.test/external-header", RELATIONSHIP_TYPE.HEADER,
                          is_external=True)
    late, _ = fixture.part.add_header_part()
    late.partname = PackURI("/word/custom/late-header.xml")
    fixture.save(source)
    content = parts(source)
    for part, root in (("word/footer1.xml", "w:hdr"), (str(wrong_header.partname)[1:], "w:ftr")):
        malformed = ElementTree.fromstring(content[part])
        malformed.tag = qn(root)
        content[part] = ElementTree.tostring(malformed)
    with ZipFile(source, "w") as archive:
        for name, payload in content.items():
            archive.writestr(name, payload)
    original = source.read_bytes()

    async with launch() as client:
        document = await open_document(client, source)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="stories"))
        defaults = await call(client, "inspect_document", document=document, view=dict(kind="default-headers"))
        assert defaults["default_headers"] == []
        ordinary = await linkable_page(client, document, "header")
        assert ordinary == await linkable_page(client, document, "header", include_referenced=False)
        (unused_owner,), end = ordinary
        assert end is None and f"/{unused_owner['part_name']}" == unused.partname

        (owner,), cursor = await linkable_page(client, document, "header", include_referenced=True, limit=1)
        assert owner["part_name"] == first_header and cursor is not None
        (same_unused,), end = await linkable_page(
            client, document, "header", include_referenced=True, cursor=cursor,
        )
        assert end is None and same_unused["part_name"] == unused_owner["part_name"]
        assert same_unused["anchor"] != unused_owner["anchor"]
        other = await open_document(client, source)
        for handle, view, error in (
            (document, selector("header", cursor=cursor), "invalid_cursor"),
            (document, selector("footer", include_referenced=True, cursor=cursor), "invalid_cursor"),
            (other, selector("header", include_referenced=True, cursor=cursor), "wrong_owner"),
        ):
            await failure(client, "inspect_document", error, document=handle, view=view)

        sections = (await call(client, "inspect_document", document=document,
                               view=dict(kind="sections")))["sections"]
        protected = sections[0]["story_slots"][1]["anchor"]
        destination = sections[1]["story_slots"][2]["anchor"]
        operation = dict(kind="edit-story-reference", anchor=destination,
                         edit=dict(kind="link", anchor=owner["anchor"]))
        rejected = await client.call_tool("preview_edits", dict(
            document=document, operation=dict(operation, anchor=protected),
        ))
        assert_tool_failure(rejected, "rejected_batch")
        assert rejected.structuredContent["detail"] == "cannot change a revised section or opaque story reference"
        other_owner, _ = await linkable_page(client, other, "header", include_referenced=True)
        await failure(client, "preview_edits", "wrong_owner", document=document,
                      operation=dict(operation, edit=dict(kind="link", anchor=other_owner[0]["anchor"])))
        output = await save_preview(client, document, source, "link-first-only", operation)

    assert source.read_bytes() == original
    saved = parts(output)
    assert saved.keys() == content.keys()
    assert {name for name in content if content[name] != saved[name]} == {MAIN}
    assert docx.Document(output).sections[1].even_page_header.paragraphs[0].text == "First-page header"
