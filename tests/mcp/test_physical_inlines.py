"""Installed stdio reads of typed header/footer and physical body-cell inlines."""

import json
import subprocess

import docx
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml.ns import qn
from docx.shared import Inches, Pt, RGBColor
from jsonschema.validators import validator_for
import pytest

from test_drawings import IMAGES
from test_entry_inlines import relation
from test_fields import opened, word
from test_ranges import inspect
from test_stdio import anyio_backend, executable, failure, launch, validate

pytestmark = pytest.mark.anyio


def add_rich_inlines(paragraph, label, bookmark_id):
    run = paragraph.add_run(f"{label} 😀")
    run.add_tab()
    run.add_break()
    run.bold, run.italic, run.font.size = True, False, Pt(13.5)
    run.font.color.rgb = RGBColor(17, 51, 119)
    paragraph._p.append(word("bookmarkStart", id=str(bookmark_id), name=f"Mark_{label}"))

    rid = paragraph.part.relate_to(
        f"https://example.test/{label}", RELATIONSHIP_TYPE.HYPERLINK, is_external=True,
    )
    link = word("hyperlink", tooltip=f"Tooltip {label}", history="0")
    link.set(qn("r:id"), rid)
    for text in ("Linked ", label):
        child = paragraph.add_run(text)
        child.italic = True
        link.append(child._r)
    paragraph._p.append(link)
    paragraph._p.append(word("bookmarkEnd", id=str(bookmark_id)))

    picture = paragraph.add_run().add_picture(str(IMAGES / "sample.png"), width=Inches(1))
    picture._inline.docPr.set("descr", f"Picture {label}")
    paragraph.add_run(f"End {label}")


@pytest.fixture
def physical_inline_source(tmp_path):
    source = tmp_path / "physical-inlines.docx"
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.add_paragraph("Body decoy")
    table = document.add_table(1, 3)
    leading = table.cell(0, 0).merge(table.cell(0, 1))
    leading.text = "Spanning cell"
    leading.paragraphs[0].add_run(" second inline")
    cell = table.cell(0, 2)
    cell.text = "Cell lead"
    cell.add_table(1, 1).cell(0, 0).text = "Nested cell decoy"
    add_rich_inlines(cell.paragraphs[-1], "cell", 3)
    cell.add_paragraph()  # Direct blocks: lead, opaque table, rich, empty.

    first = document.sections[0]
    for i, label in enumerate(("header", "footer"), start=1):
        story = getattr(first, label)
        story.paragraphs[0].text = f"Lead {label}"
        story.paragraphs[0].add_run(" second inline")
        story.add_table(1, 1, Inches(1)).cell(0, 0).text = f"Nested {label} decoy"
        add_rich_inlines(story.add_paragraph(), label, i)
        story.add_paragraph()
    document.add_section()  # Both effective default slots inherit.
    document.save(source)
    return source


def native_inspection(source):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_physical_inline_oracle")
    return json.loads(subprocess.check_output([oracle, source], timeout=20))


def assert_projection(actual, expected):
    """Match native fields without imposing a full transport response shape."""
    if isinstance(expected, dict):
        for key, value in expected.items():
            assert_projection(actual[key], value)
    elif isinstance(expected, list):
        assert len(actual) == len(expected)
        for item, value in zip(actual, expected, strict=True):
            assert_projection(item, value)
    else:
        assert actual == expected


async def test_physical_inline_selector_discovery(launch):
    async with launch() as client:
        schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                      if tool.name == "inspect_document")
    valid = validator_for(schema)(schema).is_valid
    for selector in (
        dict(kind="story-inlines", anchor="story", block_index=2),
        dict(kind="table-cell-inlines", anchor="table", row_index=0, cell_index=1, block_index=2),
    ):
        assert valid(dict(document="d", view=selector))
        assert valid(dict(document="d", view=dict(selector, limit=1, cursor="next")))
        for key in selector.keys() - {"kind"}:
            invalid = {k: v for k, v in selector.items() if k != key}
            assert not valid(dict(document="d", view=invalid))
        for invalid in (dict(limit=0), dict(limit=101), dict(cursor=None), dict(block_index=-1)):
            assert not valid(dict(document="d", view={**selector, **invalid}))


async def test_physical_inline_pages_match_native(launch, physical_inline_source):
    source = physical_inline_source
    original = source.read_bytes()
    native = native_inspection(source)
    independent = docx.Document(source)
    assert independent.sections[1].header.is_linked_to_previous
    assert independent.sections[1].footer.is_linked_to_previous
    assert independent.tables[0].cell(0, 2).tables[0].cell(0, 0).text == "Nested cell decoy"
    validate(source)

    async with launch() as client:
        document = await opened(client, source)
        stories = (await inspect(client, document, "stories", limit=100))["stories"]
        tables = (await inspect(client, document, "body-tables"))["body_tables"]
        (table,) = tables
        owners = {}
        for label in ("header", "footer"):
            first, second = [s for s in stories if s["kind"] == label and s["variant"] == "default"]
            assert not first["inherited"] and second["inherited"]
            assert first["anchor"] != second["anchor"]
            assert first["owner"] == second["owner"]
            owners[label] = first["owner"]
        assert owners["header"] != owners["footer"]
        selectors = {
            label: dict(kind="story-inlines", anchor=owner["anchor"], block_index=2)
            for label, owner in owners.items()
        }
        selectors["cell"] = dict(kind="table-cell-inlines", anchor=table["anchor"],
                                 row_index=0, cell_index=1, block_index=2)
        selectors["adjacent_cell"] = dict(selectors["cell"], cell_index=0, block_index=0)
        cursors = {}
        for label, selector in selectors.items():
            whole = await inspect(client, document, **selector, limit=100)
            assert whole["next_cursor"] is None
            first = await inspect(client, document, **selector, limit=1)
            cursor = first["next_cursor"]
            assert cursor is not None
            cursors[label] = cursor
            rest = await inspect(client, document, **selector, cursor=cursor, limit=100)
            assert rest["next_cursor"] is None
            assert len(first["inlines"]) == 1
            assert first["inlines"] + rest["inlines"] == whole["inlines"]
            assert_projection(whole["inlines"], native[label])
            if label == "adjacent_cell":
                continue

            rich, start, link, end, picture, tail = whole["inlines"]
            assert rich["items"] == [
                dict(kind="text", text=f"{label} 😀"),
                dict(kind="tab"),
                dict(kind="break", break_type="text-wrapping"),
            ]
            assert rich["bold"] is True and rich["italic"] is False
            assert rich["format"]["size"] == 13.5 and rich["format"]["color"] == [17, 51, 119]
            assert (start["name"], end["name"]) == ("w:bookmarkStart", "w:bookmarkEnd")
            assert link["tooltip"] == f"Tooltip {label}"
            part = owners[label]["part_name"] if label in owners else "word/document.xml"
            assert relation(source, part, link["relationship_id"]) == f"https://example.test/{label}"
            drawing = picture["drawing"]
            assert drawing["kind"] == "image" and drawing["description"] == f"Picture {label}"
            assert relation(source, part, drawing["relationship_id"]) == (IMAGES / "sample.png").read_bytes()
            assert tail["items"] == [dict(kind="text", text=f"End {label}")]
            empty = await inspect(client, document, **dict(selector, block_index=3))
            assert empty["inlines"] == [] and empty["next_cursor"] is None
            for invalid in (dict(block_index=1), dict(block_index=4)):
                await failure(client, "inspect_document", "inspection_failed", document=document,
                              view={**selector, **invalid})

        for invalid in (dict(row_index=1), dict(cell_index=2)):
            await failure(client, "inspect_document", "inspection_failed", document=document,
                          view={**selectors["cell"], **invalid})

        for target in (selectors["footer"], selectors["cell"], dict(selectors["header"], block_index=0)):
            await failure(client, "inspect_document", "invalid_cursor", document=document,
                          view=dict(target, cursor=cursors["header"]))
        for target in (selectors["adjacent_cell"], dict(selectors["cell"], block_index=0)):
            await failure(client, "inspect_document", "invalid_cursor", document=document,
                          view=dict(target, cursor=cursors["cell"]))
        for target, wrong_anchor in (
            (selectors["header"], table["anchor"]),
            (selectors["header"], stories[0]["anchor"]),
            (selectors["cell"], owners["header"]["anchor"]),
        ):
            await failure(client, "inspect_document", "wrong_anchor_type", document=document,
                          view=dict(target, anchor=wrong_anchor))
        reopened = await opened(client, source)
        other_stories = (await inspect(client, reopened, "stories", limit=100))["stories"]
        other_header = other_stories[0]["owner"]["anchor"]
        await failure(client, "inspect_document", "wrong_owner", document=reopened, view=selectors["header"])
        await failure(client, "inspect_document", "wrong_owner", document=reopened, view={
            **selectors["header"], "anchor": other_header, "cursor": cursors["header"],
        })
    assert source.read_bytes() == original
