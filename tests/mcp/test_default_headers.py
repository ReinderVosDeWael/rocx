"""Installed stdio default-header pages compared with independent native readback."""

import json
import subprocess
from xml.etree import ElementTree
from zipfile import ZipFile

import docx
from docx.opc.packuri import PackURI
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches
from jsonschema.validators import validator_for
import pytest

from test_stdio import PNG, anyio_backend, call, executable, failure, launch, parts

pytestmark = pytest.mark.anyio


@pytest.fixture
def sources(tmp_path):
    clean = tmp_path / "clean.docx"
    document = docx.Document()
    section = document.sections[0]
    section.footer.paragraphs[0].text = "Unrelated footer"
    footer_part = str(section.footer.part.partname)[1:]
    section.first_page_header.paragraphs[0].text = "Unrelated first header"
    first_part = str(section.first_page_header.part.partname)[1:]
    header = document.add_section().header
    header.is_linked_to_previous = False
    header.part.partname = PackURI("/word/stories/relocated-header.xml")
    paragraph = header.paragraphs[0]
    run = paragraph.add_run("Header 😀")
    run.bold = True
    run.add_tab()
    run._r.append(OxmlElement("w:br"))
    run._r[-1].set(qn("w:type"), "page")
    for tag, attrs, text in (
        ("hyperlink", dict(anchor="target", tooltip="Hint", history="0"), "Linked"),
        ("ins", dict(id="007", author="Reviewer"), "Inserted"),
    ):
        node = OxmlElement(f"w:{tag}")
        for key, value in attrs.items():
            node.set(qn(f"w:{key}"), value)
        node.append(paragraph.add_run(text)._r)
        paragraph._p.append(node)
    header.add_table(1, 1, Inches(1))
    header.add_paragraph()
    header.add_paragraph("Editable header")
    document.add_section()  # Inherit the relocated part.
    last = document.add_section().header
    last.is_linked_to_previous = False
    last.paragraphs[0].text = "Distinct header"
    document.save(clean)

    independent = docx.Document(clean)
    assert independent.sections[1].header.paragraphs[0].runs[0].bold is True

    def corrupt(name, part):
        source = tmp_path / name
        with ZipFile(source, "w") as archive:
            for path, data in parts(clean).items():
                archive.writestr(path, b"<broken" if path == part else data)
        return source

    return clean, corrupt("bad-footer.docx", footer_part), corrupt("bad-first.docx", first_part)


def project_like(actual, expected):
    """Compare native payload fields while leaving MCP-only fields out."""
    if isinstance(expected, dict):
        return {key: project_like(actual[key], value) for key, value in expected.items()}
    if isinstance(expected, list):
        return [project_like(left, right) for left, right in zip(actual, expected, strict=True)]
    return actual


@pytest.mark.parametrize("case", [1, 2], ids=["malformed-footer", "malformed-first-header"])
async def test_native_default_headers(launch, sources, case):
    source = sources[case]
    original = source.read_bytes()
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_default_header_oracle")
    native = json.loads(subprocess.check_output([oracle, source], timeout=20))
    assert native["stories_ok"] is False

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]

        async def inspect(kind="default-headers", **options):
            return await call(client, "inspect_document", document=document,
                              view=dict(kind=kind, **options))

        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="stories"))
        first = await inspect(limit=1)
        rest = await inspect(limit=2, cursor=first["next_cursor"])
        assert rest["next_cursor"] is None
        whole = await inspect()
        rows = whole["default_headers"]
        assert first["default_headers"] + rest["default_headers"] == rows
        assert project_like(rows, native["default_headers"]) == native["default_headers"]
        one, alias, distinct = rows
        assert [(row["section_index"], row["inherited"]) for row in rows] == [
            (1, False), (2, True), (3, False),
        ]
        assert one["part_name"] == alias["part_name"] == "word/stories/relocated-header.xml"
        assert one["owner"] == alias["owner"]
        assert one["blocks"] == alias["blocks"]
        assert one["part_name"] != distinct["part_name"]
        main, opaque, empty, _editable = one["blocks"]
        assert opaque == dict(index=1, type="opaque", name="w:tbl")
        assert empty["inline_items"] == empty["inlines"] == []
        run, link, revision = main["inlines"]
        assert [node["inline_index"] for node in (run, link, revision)] == [0, 1, 2]
        assert run["inline_handle"] != distinct["blocks"][0]["inlines"][0]["inline_handle"]
        child = link["children"][0]
        lookup = await inspect("inline-at", inline_handle=child["inline_handle"])
        assert lookup["inline"] == child

        # Compact inspection retains its legacy dependence on all stories.
        await failure(client, "inspect_document", "inspection_failed", document=document)
    assert source.read_bytes() == original


async def test_default_headers_schema_and_cursor_scope(launch, sources):
    clean = sources[0]
    async with launch() as client:
        schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                      if tool.name == "inspect_document")
        valid = validator_for(schema)(schema).is_valid
        document = (await call(client, "open_document", path=clean.name))["document"]
        second = (await call(client, "open_document", path=clean.name))["document"]
        selector = dict(kind="default-headers")
        assert valid(dict(document=document, view=selector))
        for invalid in (dict(limit=0), dict(limit=101), dict(limit=None), dict(cursor=None),
                        dict(cursor=1), dict(extra=True)):
            view = dict(selector, **invalid)
            assert not valid(dict(document=document, view=view))
            await failure(client, "inspect_document", "invalid_input", document=document, view=view)

        async def cursor(kind):
            response = await call(client, "inspect_document", document=document,
                                  view=dict(kind=kind, limit=1))
            return response["next_cursor"]

        headers_cursor = await cursor("default-headers")
        stories_cursor = await cursor("stories")
        for owner, kind, token, category in (
            (document, "default-headers", stories_cursor, "invalid_cursor"),
            (document, "stories", headers_cursor, "invalid_cursor"),
            (document, "default-headers", f"{document}/default-headers:2", "invalid_cursor"),
            (second, "default-headers", headers_cursor, "wrong_owner"),
        ):
            await failure(client, "inspect_document", category, document=owner,
                          view=dict(kind=kind, cursor=token))


def with_wrong_footer_root(clean):
    footer_part = str(docx.Document(clean).sections[0].footer.part.partname)[1:]
    source = clean.with_name("wrong-footer-root.docx")
    wrong_root = b'<w:p xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>'
    with ZipFile(source, "w") as archive:
        for path, data in parts(clean).items():
            archive.writestr(path, wrong_root if path == footer_part else data)
    return source, footer_part


@pytest.fixture
def wrong_footer_source(sources):
    return with_wrong_footer_root(sources[0])


@pytest.fixture
def wrong_footer_slot_source(tmp_path):
    clean = tmp_path / "copyable-header.docx"
    document = docx.Document()
    section = document.sections[0]
    section.header.paragraphs[0].text = "Shared default header"
    section.footer.paragraphs[0].text = "Wrong-root footer fixture"
    document.add_paragraph("Unrelated body content")
    document.add_section()
    document.add_section()
    document.save(clean)
    return with_wrong_footer_root(clean)


async def test_section_slot_creates_header_despite_wrong_footer_root(launch, wrong_footer_source):
    source, footer_part = wrong_footer_source
    original = source.read_bytes()

    output = source.with_name("created-header.docx")
    native = source.with_name("native-created-header.docx")
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_default_header_oracle")
    subprocess.run([oracle, source, native], check=True, timeout=20)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="stories"))
        section_page = await call(client, "inspect_document", document=document,
                                  view=dict(kind="sections", limit=1))
        section, = section_page["sections"]
        slots = section["story_slots"]
        assert [(row["kind"], row["variant"]) for row in slots] == [
            (kind, variant) for kind in ("header", "footer") for variant in ("default", "first", "even")
        ]
        operation = dict(kind="edit-story-reference", anchor=slots[0]["anchor"],
                         edit=dict(kind="create", paragraphs=["From section slot 😀"]))
        await failure(client, "preview_edits", "invalid_anchor", document=document,
                      operation=dict(operation, anchor=f"{document}/story-slot:6"))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      operation=dict(operation, anchor=section["anchor"]))
        preview = await call(client, "preview_edits", document=document, operation=operation)
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)

        reopened = (await call(client, "open_document", path=output.name))["document"]
        view = await call(client, "inspect_document", document=reopened, view=dict(kind="default-headers"))
        created = view["default_headers"][0]
        assert created["section_index"] == 0 and created["inherited"] is False
        assert created["blocks"][0]["text"] == "From section slot 😀"

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() < after.keys()
    assert set(after) - set(before) == {created["part_name"]}
    assert {name for name in before if before[name] != after[name]} == {
        "word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml",
    }
    assert after[footer_part] == before[footer_part]
    assert ElementTree.fromstring(after[footer_part]).tag == qn("w:p")


async def test_section_slot_keys_match_stories_without_granting_header_image_access(launch, sources):
    clean = sources[0]
    async with launch() as client:
        document = (await call(client, "open_document", path=clean.name))["document"]

        async def inspect(kind):
            return (await call(client, "inspect_document", document=document, view=dict(kind=kind)))[kind]

        sections = await inspect("sections")
        image = dict(kind="header-image", anchor=sections[1]["story_slots"][0]["anchor"],
                     mode="shared", bytes=list(PNG.read_bytes()))
        await failure(client, "preview_edits", "wrong_anchor_type", document=document, edits=[image])

        stories = await inspect("stories")
        assert [slot for row in sections for slot in row["story_slots"]] == [
            {key: row[key] for key in ("anchor", "kind", "variant")} for row in stories
        ]
        assert await inspect("sections") == sections
        # Reissuing through sections does not invalidate a prior stories-issued image anchor.
        assert (await call(client, "preview_edits", document=document, edits=[image]))["preview"]


@pytest.mark.parametrize("edit", ["text", "format"])
async def test_default_header_run_edits_ignore_wrong_footer_root(launch, wrong_footer_source, edit):
    source, _ = wrong_footer_source
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_default_header_oracle")
    native = source.with_name("native-edited-header.docx")
    subprocess.run([oracle, source, native, edit], check=True, timeout=20)
    output = source.with_name("edited-header.docx")

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="stories"))

        async def headers(doc):
            return (await call(client, "inspect_document", document=doc,
                               view=dict(kind="default-headers")))["default_headers"]

        header, alias, foreign = await headers(document)
        assert header["owner"] == alias["owner"]
        assert header["owner"]["anchor"] != foreign["owner"]["anchor"]
        run, child = alias["blocks"][0]["run_targets"]
        direct, = alias["blocks"][3]["run_targets"]
        assert run["inline_index"] == 0
        assert (child["inline_index"], child["child_index"]) == (1, 0)
        assert "anchor" not in run and "anchor" not in child
        owner = header["owner"]["anchor"]
        if edit == "text":
            operation = dict(kind="edit-story-text", anchor=owner, edits=[
                dict(anchor=direct["anchor"], text="Edited header 😀"),
            ])
            wrong_type = dict(operation, edits=[dict(anchor=child["format_target"]["anchor"], text="Gated")])
        else:
            operation = dict(kind="edit-run-format", anchor=owner, edits=[
                dict(anchor=item["format_target"]["anchor"], edit=dict(kind="set-bold", value=value))
                for item, value in [(run, False), (child, True)]
            ])
            wrong_type = dict(operation, edits=[
                dict(anchor=direct["anchor"], edit=dict(kind="set-bold", value=False)),
            ])
        await failure(client, "preview_edits", "wrong_anchor_type", document=document, operation=wrong_type)
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      operation=dict(operation, anchor=foreign["owner"]["anchor"]))
        preview = await call(client, "preview_edits", document=document, operation=operation)
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)

        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=operation)
        updated = (await headers(reopened))[1]
        if edit == "text":
            assert updated["blocks"][3]["text"] == "Edited header 😀"
        else:
            run, hyperlink, _ = updated["blocks"][0]["inlines"]
            assert run["bold"] is False and hyperlink["children"][0]["bold"] is True
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {path for path in before if before[path] != after[path]} == {header["part_name"]}


@pytest.mark.parametrize("case", ["image", "blocks"])
@pytest.mark.parametrize("mode", ["shared", "section-local"])
async def test_default_header_slot_edits_ignore_wrong_footer_root(
    launch, wrong_footer_slot_source, case, mode,
):
    source, footer = wrong_footer_slot_source
    original = source.read_bytes()
    native = source.with_name("native-slot-edit.docx")
    output = source.with_name("slot-edit.docx")
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_default_header_oracle")
    subprocess.run([oracle, source, native, case, mode, *([PNG] if case == "image" else [])],
                   check=True, timeout=20)

    def request(anchor):
        if case == "image":
            return dict(edits=[dict(kind="header-image", anchor=anchor, mode=mode,
                                    bytes=list(PNG.read_bytes()))])
        return dict(operation=dict(kind="edit-story-blocks", anchor=anchor, edit=dict(
            mode=mode, index=1, remove=0, blocks=[
                dict(kind="paragraph", paragraph=dict(inlines=[dict(kind="text", text="Added block 😀")])),
            ],
        )))

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="stories"))
        sections = (await call(client, "inspect_document", document=document,
                               view=dict(kind="sections")))["sections"]
        first = await call(client, "inspect_document", document=document,
                           view=dict(kind="default-headers", limit=1))
        later = await call(client, "inspect_document", document=document, view=dict(
            kind="default-headers", limit=1, cursor=first["next_cursor"],
        ))
        one, = first["default_headers"]
        alias, = later["default_headers"]
        assert alias["section_index"] == 1 and alias["inherited"] is True
        assert alias["owner"] == one["owner"]
        anchor = alias["image_slot"]["anchor"]
        assert anchor != one["image_slot"]["anchor"]
        for wrong in (alias["owner"]["anchor"], sections[1]["story_slots"][0]["anchor"]):
            await failure(client, "preview_edits", "wrong_anchor_type", document=document, **request(wrong))
        await failure(client, "preview_edits", "invalid_anchor", document=document,
                      **request(f"{document}/default-header-slot:2"))
        other = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=other, **request(anchor))
        preview = await call(client, "preview_edits", document=document, **request(anchor))
        await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    changed = {name for name in before if before[name] != after[name]}
    if mode == "section-local":
        assert changed == {"word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"}
    else:
        assert changed == {alias["part_name"]} | ({"[Content_Types].xml"} if case == "image" else set())
    assert after[footer] == before[footer]

    independent = docx.Document(output)
    observed = []
    for section in independent.sections:
        header = section.header
        if case == "image":
            observed.append(any(p._p.find(f".//{qn('w:drawing')}") is not None
                                for p in header.paragraphs))
        else:
            observed.append(header.paragraphs[-1].text == "Added block 😀")
    assert observed == ([True, True, True] if mode == "shared" else [False, True, False])
