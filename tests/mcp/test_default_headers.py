"""Installed stdio default-header pages compared with independent native readback."""

import json
import subprocess
from zipfile import ZipFile

import docx
from docx.opc.packuri import PackURI
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, call, executable, failure, launch, parts

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
        assert one["blocks"] == alias["blocks"]
        assert one["part_name"] != distinct["part_name"]
        main, opaque, empty = one["blocks"]
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
