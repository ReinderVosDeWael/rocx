"""Body block edits and section inspection through the installed stdio binary."""

import subprocess

import docx
from docx.enum.section import WD_SECTION_START
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio


def make_source(path):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    keep = document.add_paragraph("keep")
    keep.paragraph_format.keep_with_next = True
    keep.paragraph_format.widow_control = True
    for text in ("replace", "remove", "original run"):
        document.add_paragraph(text)
    first = document.sections[0]
    first.header.paragraphs[0].text = "inherited header"
    first.different_first_page_header_footer = True
    first.start_type = WD_SECTION_START.CONTINUOUS
    middle = document.add_section()
    middle.different_first_page_header_footer = False
    document.add_paragraph("section start")
    document.add_paragraph("section end")
    document.add_section(WD_SECTION_START.ODD_PAGE)
    document.add_paragraph("section properties")
    document.add_paragraph("far section")
    document.add_table(1, 1).cell(0, 0).text = "unchanged table"
    document.save(path)
    return path.read_bytes()


def paragraph(text, *, bold=False, alignment=None):
    inline = dict(kind="text", text=text)
    if bold:
        inline["format"] = dict(bold=True)
    payload = dict(inlines=[inline])
    if alignment:
        payload["format"] = dict(alignment=alignment)
    return payload


def block(anchor, kind, **payload):
    return dict(kind="block", anchor=anchor, edit=dict(kind=kind, **payload))


def run_text(row, text):
    (run,) = row["runs"]
    return dict(kind="run-text", anchor=run["anchor"], text=text)


async def sections(client, document, **view):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="sections", **view))


async def open_document(client, path):
    return (await call(client, "open_document", path=path.name))["document"]


def ranges(page):
    return [(row["body_start"], row["body_end"]) for row in page["sections"]]


def native_bytes(source, output, mode):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_block_oracle")
    subprocess.run([oracle, source, output, mode], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


async def save(client, document, draft, output):
    saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                       output=output.name)
    assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())


def check_package(source, output, mode, original):
    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, output.with_name("native.docx"), mode)
    before, after = parts(source), parts(output)
    assert after.keys() == before.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    validate(source, output)
    independent = docx.Document(output)
    assert independent.tables[0].cell(0, 0).text == "unchanged table"
    assert not independent.sections[0].header.is_linked_to_previous
    assert all(section.header.is_linked_to_previous for section in independent.sections[1:])
    assert all(section.header.paragraphs[0].text == "inherited header"
               for section in independent.sections)
    return independent


async def test_block_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
    preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])
    assert inspect.is_valid(dict(document="doc", view=dict(kind="sections", limit=1, cursor="page")))
    assert not inspect.is_valid(dict(document="doc", view=dict(kind="sections", limit=0)))

    def accepted(kind, **payload):
        return preview.is_valid(dict(document="doc", edits=[block("anchor", kind, **payload)]))

    for kind, payload in [
        ("split-section", dict(offset=1)),
        ("merge-section", {}),
        ("replace-paragraph", dict(paragraph=paragraph("bold", bold=True, alignment="center"))),
        ("remove-paragraph", {}),
        ("replace-section", dict(blocks=[dict(kind="paragraph", paragraph=paragraph("next"))])),
        ("keep-with-next", dict(value=False)),
        ("paragraph-flag", dict(field="widowControl", value=None)),
        ("section-flag", dict(field="titlePage", value=None)),
        ("section-type", dict(value=None)),
        ("section-type", dict(value="evenPage")),
    ]:
        assert accepted(kind, **payload), (kind, payload)
    for kind, payload in [
        ("split-section", dict(offset=-1)),
        ("merge-section", dict(offset=1)),
        ("replace-paragraph", dict(paragraph=dict(text="not an inline list"))),
        ("paragraph-flag", dict(field="widowControl")),
        ("paragraph-flag", dict(field="titlePage", value=True)),
        ("section-flag", dict(field="titlePage")),
        ("section-type", {}),
        ("section-type", dict(value="unknown")),
    ]:
        assert not accepted(kind, **payload), (kind, payload)


async def test_mixed_block_preview_rejects_overlap_then_matches_native(launch, tmp_path):
    source, output = tmp_path / "source.docx", tmp_path / "mixed.docx"
    original = make_source(source)
    async with launch() as client:
        document = await open_document(client, source)
        body = (await call(client, "inspect_document", document=document))["body"]
        inspection = await sections(client, document)
        first = await sections(client, document, limit=1)
        remainder = await sections(client, document, limit=2, cursor=first["next_cursor"])
        assert remainder["next_cursor"] is None
        assert first["sections"] + remainder["sections"] == inspection["sections"]
        assert ranges(inspection) == [(0, 5), (5, 8), (8, 11)]
        initial, middle, final = inspection["sections"]
        assert len({initial["anchor"], middle["anchor"], final["anchor"]}) == 3
        assert initial["flags"] == {"titlePage": True}
        assert initial["section_type"] == initial["section_type_xml"] == "continuous"
        assert final["section_type"] == final["section_type_xml"] == "oddPage"

        def p(index):
            return body[index]["anchor"]

        replacement = block(p(1), "replace-paragraph",
                            paragraph=paragraph("new & bold", bold=True, alignment="center"))
        edits = [
            block(p(5), "replace-section",
                  blocks=[dict(kind="paragraph", paragraph=paragraph("new section"))]),
            block(p(2), "remove-paragraph"),
            replacement,
            block(p(0), "keep-with-next", value=False),
            block(p(0), "paragraph-flag", field="widowControl", value=None),
            block(p(0), "section-flag", field="titlePage", value=None),
            block(p(8), "section-flag", field="titlePage", value=False),
            block(p(0), "section-type", value="evenPage"),
            block(p(8), "section-type", value=None),
            run_text(body[3], "edited original run"),
        ]
        for index in (1, 6):
            await failure(client, "preview_edits", "rejected_batch", document=document,
                          edits=[*edits, run_text(body[index], "overlaps replacement")])
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[dict(replacement, anchor=initial["anchor"])])
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="layout", cursor=first["next_cursor"]))
        for edit in [block(p(0), "paragraph-flag", field="widowControl"),
                     block(p(0), "section-flag", field="titlePage"),
                     block(p(0), "section-type")]:
            await failure(client, "preview_edits", "invalid_input", document=document, edits=[edit])
        assert await sections(client, document) == inspection
        assert list(tmp_path.iterdir()) == [source]
        draft = await call(client, "preview_edits", document=document, edits=edits)
        assert draft["changes"]["block_changes"] == [0, 0, 0, 0, 1, 2, 5, 8, 8]
        assert list(tmp_path.iterdir()) == [source]
        await save(client, document, draft, output)
        assert await sections(client, document) == inspection
        assert (await call(client, "inspect_document", document=document))["body"] == body

        reopened = await open_document(client, output)
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="sections", cursor=first["next_cursor"]))
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[replacement])
        changed = await sections(client, reopened)
        assert ranges(changed) == [(0, 4), (4, 6), (6, 9)]
        initial, _, final = changed["sections"]
        assert initial["flags"] == {}
        assert initial["section_type"] == initial["section_type_xml"] == "evenPage"
        assert final["flags"] == {"titlePage": False}
        assert final["section_type"] is final["section_type_xml"] is None

    independent = check_package(source, output, "mixed", original)
    assert [p.text for p in independent.paragraphs] == [
        "keep", "new & bold", "edited original run", "", "new section", "",
        "section properties", "far section",
    ]
    keep, replacement = independent.paragraphs[:2]
    assert keep.paragraph_format.keep_with_next is False
    assert keep.paragraph_format.widow_control is None
    assert replacement.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert replacement.runs[0].bold is True
    assert independent.sections[0].start_type == WD_SECTION_START.EVEN_PAGE
    assert not independent.sections[-1].different_first_page_header_footer


@pytest.mark.parametrize(
    "mode, kind, payload, change, expected_ranges, expected_texts",
    [
        ("split", "split-section", dict(offset=2), 2, [(0, 3), (3, 6), (6, 9), (9, 12)],
         ["keep", "replace", "", "remove", "original run", "", "section start", "section end", ""]),
        ("merge", "merge-section", {}, 4, [(0, 7), (7, 10)],
         ["keep", "replace", "remove", "original run", "section start", "section end", ""]),
    ],
    ids=["split", "merge"],
)
async def test_section_lifecycle_matches_native(
    launch, tmp_path, mode, kind, payload, change, expected_ranges, expected_texts,
):
    source, output = tmp_path / "source.docx", tmp_path / f"{mode}.docx"
    original = make_source(source)
    async with launch() as client:
        document = await open_document(client, source)
        inspection = await sections(client, document)
        initial, _, final = inspection["sections"]
        body = (await call(client, "inspect_document", document=document))["body"]
        edit = block(initial["anchor"], kind, **payload)
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[block(body[0]["anchor"], kind, **payload)])
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      edits=[edit, run_text(body[0], "overlaps affected section")])
        invalid = (block(initial["anchor"], kind, offset=5) if mode == "split"
                   else block(final["anchor"], kind))
        await failure(client, "preview_edits", "rejected_batch", document=document, edits=[invalid])
        draft = await call(client, "preview_edits", document=document,
                           edits=[edit, run_text(body[9], "edited far section")])
        assert draft["changes"]["block_changes"] == [change]
        assert list(tmp_path.iterdir()) == [source]
        await save(client, document, draft, output)
        assert await sections(client, document) == inspection
        reopened = await open_document(client, output)
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[edit])
        assert ranges(await sections(client, reopened)) == expected_ranges

    independent = check_package(source, output, mode, original)
    assert [p.text for p in independent.paragraphs] == [
        *expected_texts, "section properties", "edited far section",
    ]
    assert independent.sections[0].start_type == (
        WD_SECTION_START.CONTINUOUS if mode == "split" else WD_SECTION_START.NEW_PAGE
    )
    if mode == "split":
        assert independent.sections[1].start_type == WD_SECTION_START.CONTINUOUS
    assert independent.sections[-1].start_type == WD_SECTION_START.ODD_PAGE
