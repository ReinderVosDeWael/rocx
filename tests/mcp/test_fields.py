"""Field discovery and ordinary mixed edits through the installed stdio client."""

import subprocess

import docx
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio


def word(tag, text=None, **attributes):
    element = OxmlElement(f"w:{tag}")
    for key, value in attributes.items():
        element.set(qn(f"w:{key}"), value)
    if text is not None:
        element.text = text
    return element


def simple(instruction, value, **flags):
    node = word("fldSimple", instr=instruction, **flags)
    run = word("r")
    run.append(word("t", value))
    node.append(run)
    return node


def append_run(paragraph, child):
    run = word("r")
    run.append(child)
    paragraph._p.append(run)


def make_source(path):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.sections[0].header.paragraphs[0].text = "untouched header"
    document.add_table(1, 1).cell(0, 0).text = "untouched table"

    first = document.add_paragraph("before")
    first.paragraph_format.keep_with_next = True  # Not an inline slot.
    first._p.append(simple(" PAGE ", "old simple", dirty="true", fldLock="on"))
    first._p.append(simple(" PAGE ", "replace me"))

    start = document.add_paragraph("lead")
    append_run(start, word("fldChar", fldCharType="begin", dirty="on", fldLock="false"))
    for text in (" PA", "GE "):
        code = word("instrText", text)
        code.set(qn("xml:space"), "preserve")
        append_run(start, code)
    append_run(start, word("fldChar", fldCharType="separate"))
    start.add_run("old start")
    start._p.append(simple(" DATE ", "nested cache"))
    document.add_paragraph()

    end = document.add_paragraph("old end")
    append_run(end, word("fldChar", fldCharType="end"))
    end.add_run("after")
    end._p.append(simple(" SECTION ", "remove me"))

    opaque = simple(" AUTHOR ", "protected display")
    opaque.insert(0, word("bookmarkStart", id="42", name="ProtectedDisplay"))
    opaque.append(word("bookmarkEnd", id="42"))
    document.add_paragraph()._p.append(opaque)
    document.add_paragraph("ordinary")
    document.save(path)
    return path.read_bytes()


def edit(anchor, kind, **payload):
    return dict(kind="field", anchor=anchor, edit=dict(kind=kind, **payload))


def text(value, **payload):
    return dict(kind="text", text=value, **payload)


def replacement():
    return dict(kind="field", instruction=dict(kind="page"), complex=True, dirty=True,
                children=[text("replacement display")])


async def fields(client, document, **view):
    return await call(client, "inspect_document", document=document, view=dict(kind="fields", **view))


async def opened(client, path):
    return (await call(client, "open_document", path=path.name))["document"]


async def test_field_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    inspect_schema = schemas["inspect_document"]
    inspect = validator_for(inspect_schema)(inspect_schema)
    preview_schema = schemas["preview_edits"]
    preview = validator_for(preview_schema)(preview_schema)
    assert inspect.is_valid(dict(document="doc", view=dict(kind="fields", limit=1, cursor="page")))
    assert not inspect.is_valid(dict(document="doc", view=dict(kind="fields", limit=101)))
    for kind, payload in [
        ("instruction", dict(value=" PAGE ")),
        ("dirty", dict(value=False)),
        ("dirty", dict(value=None)),
        ("locked", dict(value=True)),
        ("locked", dict(value=None)),
        ("cache", dict(children=[])),
        ("replace", dict(inline=replacement())),
        ("remove", {}),
    ]:
        assert preview.is_valid(dict(document="doc", edits=[edit("anchor", kind, **payload)]))
    for kind, payload in [
        ("instruction", dict(value=None)),
        ("dirty", {}),
        ("locked", dict(value="true")),
        ("cache", dict(children=[dict(kind="unknown")])),
        ("replace", {}),
        ("remove", dict(value=False)),
    ]:
        assert not preview.is_valid(dict(document="doc", edits=[edit("anchor", kind, **payload)]))


async def test_fields_mixed_batch_matches_native_after_rejection(launch, tmp_path):
    source, output = tmp_path / "source.docx", tmp_path / "fields.docx"
    original = make_source(source)
    async with launch() as client:
        document = await opened(client, source)
        body = (await call(client, "inspect_document", document=document))["body"]
        inspection = await fields(client, document)
        first = await fields(client, document, limit=2)
        middle = await fields(client, document, limit=2, cursor=first["next_cursor"])
        tail = await fields(client, document, limit=2, cursor=middle["next_cursor"])
        assert tail["next_cursor"] is None
        assert first["fields"] + middle["fields"] + tail["fields"] == inspection["fields"]
        assert [(row["body_index"], row["inline_index"]) for row in inspection["fields"]] == [
            (1, 1), (1, 2), (2, 1), (4, 3), (5, 0),
        ]
        simple_row, replace_row, complex_row, remove_row, protected = inspection["fields"]
        assert len({row["anchor"] for row in inspection["fields"]}) == 5
        assert simple_row == {
            "anchor": simple_row["anchor"], "body_index": 1, "inline_index": 1,
            "complex": False, "instruction": " PAGE ", "dirty_xml": "true", "locked_xml": "on",
            "inline_count": 1, "editable": True,
        }
        assert complex_row == {
            "anchor": complex_row["anchor"], "body_index": 2, "inline_index": 1,
            "complex": True, "instruction": " PAGE ", "dirty_xml": "on", "locked_xml": "false",
            "inline_count": 8, "editable": True,
        }
        assert protected["editable"] is False and protected["complex"] is False
        assert protected["reason"] and "instruction" not in protected

        a, b = simple_row["anchor"], complex_row["anchor"]
        change = edit(b, "instruction", value=" NUMPAGES ")
        edits = [
            edit(a, "dirty", value=False),
            edit(a, "locked", value=None),
            edit(a, "cache", children=[
                dict(kind="hyperlink", target=dict(kind="url", url="https://example.test/cache"),
                     children=[text("linked display", format=dict(bold=True))]),
            ]),
            change,
            edit(b, "dirty", value=None),
            edit(b, "locked", value=True),
            edit(b, "cache", children=[text("complex display")]),
            edit(replace_row["anchor"], "replace", inline=replacement()),
            edit(remove_row["anchor"], "remove"),
            dict(kind="run-text", anchor=body[6]["runs"][0]["anchor"], text="ordinary edited"),
        ]
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[edit(body[2]["anchor"], "remove")])
        for rejected in [
            [edit(protected["anchor"], "remove")],
            [edit(a, "replace", inline=text("not a field"))],
            [*edits, change],
            [*edits, dict(kind="run-text", anchor=body[4]["runs"][0]["anchor"], text="overlap")],
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=rejected)
        await failure(client, "preview_edits", "invalid_input", document=document,
                      edits=[edit(a, "dirty")])
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="sections", cursor=first["next_cursor"]))
        assert await fields(client, document) == inspection
        assert list(tmp_path.iterdir()) == [source]
        draft = await call(client, "preview_edits", document=document, edits=edits)
        saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())
        assert await fields(client, document) == inspection

        reopened = await opened(client, output)
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=[change])
        await failure(client, "inspect_document", "wrong_owner", document=reopened,
                      view=dict(kind="fields", cursor=first["next_cursor"]))
        result = (await fields(client, reopened))["fields"]
        assert [(row.get("instruction"), row.get("dirty_xml"), row.get("locked_xml"))
                for row in result] == [
            (" PAGE ", "0", None), (" PAGE ", "1", None), (" NUMPAGES ", None, "1"),
            (None, None, None),
        ]
        assert [row["complex"] for row in result] == [False, True, True, False]

    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_field_oracle")
    native = tmp_path / "native.docx"
    subprocess.run([oracle, source, native], check=True, cwd=tmp_path, timeout=20)
    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {name for name in before if before[name] != after[name]} == {
        "word/document.xml", "word/_rels/document.xml.rels",
    }
    validate(source, output)

    result = docx.Document(output)
    assert result.tables[0].cell(0, 0).text == "untouched table"
    assert result.sections[0].header.paragraphs[0].text == "untouched header"
    assert [p.text for p in result.paragraphs] == [
        "beforereplacement display", "leadcomplex display", "", "after", "", "ordinary edited",
    ]
    simple_after, protected_after = etree.fromstring(after["word/document.xml"]).findall(
        f".//{qn('w:fldSimple')}",
    )
    link = simple_after.find(qn("w:hyperlink"))
    assert link.find(f".//{qn('w:t')}").text == "linked display"
    assert link.find(f".//{qn('w:b')}") is not None
    assert result.part.rels[link.get(qn("r:id"))].target_ref == "https://example.test/cache"
    assert protected_after.find(qn("w:bookmarkStart")).get(qn("w:name")) == "ProtectedDisplay"
    assert protected_after.find(f".//{qn('w:t')}").text == "protected display"
