"""Native editor diagnostics over the installed stdio transport."""

import json
import subprocess
from zipfile import ZipFile

import docx
from docx.oxml import OxmlElement
from docx.oxml.ns import nsdecls, qn
from lxml import etree
import pytest

from test_stdio import anyio_backend, call, executable, launch, make_entry_source, parts  # noqa: F401

pytestmark = pytest.mark.anyio

BATCH_ACTION = "Check the edit targets and values, then retry the batch."
BATCH_HINT = "revise the operation and retry"
IMPORT_ACTION = "Check that the input is a valid DOCX and fits the supplied import limits."
IO_ACTION = "Check the server-host path and permissions before retrying."
INSPECTION_ACTION = "Check that the input is a valid DOCX."


def native_rejection(result, detail, hint=BATCH_HINT):
    assert result.isError
    assert result.structuredContent == {
        "category": "rejected_batch",
        "message": f"{detail}; {hint}",
        "action": BATCH_ACTION,
        "rule": "unsupported_edit",
        "detail": detail,
    }


def write_source(path):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    source.add_paragraph("Original")
    source.save(path)


def replace_part(source, target, part, payload):
    payloads = parts(source)
    assert part in payloads
    payloads[part] = payload
    with ZipFile(target, "w") as archive:
        for name, data in payloads.items():
            archive.writestr(name, data)


def inspection_diagnostic(detail, *, rule="unsupported_edit", path=None):
    expected = {
        "category": "inspection_failed",
        "message": detail if path is None else f"{path}: {detail}",
        "action": INSPECTION_ACTION,
        "rule": rule,
        "detail": detail,
    }
    if path is not None:
        expected["path"] = path
    return expected


async def assert_inspection_error(client, document, expected, view=None):
    args = dict(document=document)
    if view is not None:
        args["view"] = view
    result = await client.call_tool("inspect_document", args)
    assert result.isError
    assert result.structuredContent == expected


@pytest.mark.parametrize(
    ("part", "detail", "views"),
    [
        ("word/settings.xml", "invalid settings root", [
            dict(kind="resources"),
            dict(kind="resource-value", query=dict(kind="document-defaults")),
        ]),
        ("word/numbering.xml", "invalid numbering root", [
            dict(kind="numbering-abstracts", limit=1),
            dict(kind="numbering-instances", limit=1),
            dict(kind="resource-value", query=dict(kind="abstract-starts")),
            dict(kind="resource-value", query=dict(kind="numbering-instances")),
        ]),
        ("word/styles.xml", "invalid styles root", [
            dict(kind="styles", limit=1),
            dict(kind="resource-value", query=dict(kind="document-defaults")),
        ]),
    ],
    ids=["settings", "numbering", "styles"],
)
async def test_malformed_inspection_catalogs(launch, tmp_path, part, detail, views):
    clean = tmp_path / "source.docx"
    malformed = tmp_path / "malformed.docx"
    write_source(clean)
    replace_part(clean, malformed, part, etree.tostring(OxmlElement("w:p")))

    async with launch() as client:
        document = (await call(client, "open_document", path=malformed.name))["document"]
        for view in views * 2:  # A cached native failure keeps the same diagnostic.
            await assert_inspection_error(client, document, inspection_diagnostic(detail), view)
        body = await call(client, "inspect_document", document=document, view=dict(kind="body"))
        assert body["body"][0]["text"] == "Original"


async def test_story_native_diagnostic_keeps_path_and_isolated_views(launch, tmp_path):
    clean = tmp_path / "source.docx"
    malformed = tmp_path / "malformed.docx"
    write_source(clean)
    fixture = docx.Document(clean)
    fixture.sections[0].header.paragraphs[0].text = "Header text"
    fixture.save(clean)
    part = "word/header1.xml"
    replace_part(clean, malformed, part, (
        f'<w:hdr {nsdecls("w")} '
        'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
        'mc:MustUnderstand="missing"><w:p/></w:hdr>'
    ).encode())
    expected = inspection_diagnostic(
        'unbound prefix "missing" in mc:MustUnderstand',
        rule="unsupported_profile", path=part,
    )

    async with launch() as client:
        document = (await call(client, "open_document", path=malformed.name))["document"]
        for view in (None, dict(kind="stories"), dict(kind="default-headers"),
                     dict(kind="physical-tables"), None):
            await assert_inspection_error(client, document, expected, view)
        formats = await call(client, "inspect_document", document=document,
                             view=dict(kind="format-runs"))
        unavailable, = [row for row in formats["format_runs"] if row["kind"] == "unavailable"]
        assert unavailable == {
            "kind": "unavailable",
            "catalog": "stories",
            "error": expected["message"],
            "diagnostic": {key: expected[key] for key in ("rule", "path", "detail")},
        }
        sections = await call(client, "inspect_document", document=document,
                              view=dict(kind="sections"))
        assert sections["sections"][0]["story_slots"]
        body = await call(client, "inspect_document", document=document, view=dict(kind="body"))
        assert body["body"][0]["text"] == "Original"


async def test_malformed_entry_catalog_retains_diagnostic(launch, tmp_path):
    clean = tmp_path / "entries.docx"
    malformed = tmp_path / "malformed.docx"
    make_entry_source(clean)
    replace_part(clean, malformed, "word/notes/footnotes.xml", etree.tostring(OxmlElement("w:p")))
    expected = inspection_diagnostic("invalid footnotes root")
    async with launch() as client:
        document = (await call(client, "open_document", path=malformed.name))["document"]
        for view in (
            dict(kind="entries"), dict(kind="entry-runs"), dict(kind="entry-inlines"),
            dict(kind="numbering-paragraphs", scope="entries"),
        ):
            await assert_inspection_error(client, document, expected, view)


async def test_inspection_native_rejection_is_distinct_from_anchor_and_wire_errors(launch, tmp_path):
    write_source(tmp_path / "source.docx")
    async with launch() as client:
        document = (await call(client, "open_document", path="source.docx"))["document"]
        inlines = await call(client, "inspect_document", document=document,
                             view=dict(kind="body-inlines"))
        run = inlines["body_inlines"][0]
        await assert_inspection_error(client, document, inspection_diagnostic(
            "expected a workbook-free, single-series column chart with indexed literal data",
        ), dict(kind="chart-series", anchor=run["anchor"]))
        body = await call(client, "inspect_document", document=document, view=dict(kind="body"))
        await assert_inspection_error(client, document, {
            "category": "wrong_anchor_type",
            "message": "expected a compact body paragraph anchor",
            "action": "Inspect the document again and use an anchor of the required type.",
        }, dict(kind="paragraph-numbering", anchor=body["body"][0]["runs"][0]["anchor"]))
        await assert_inspection_error(client, document, {
            "category": "invalid_cursor",
            "message": "restart the selected view",
            "action": "Omit the cursor to restart, or use a next_cursor from the same view.",
        }, dict(kind="styles", cursor="unknown"))
        wire = await client.call_tool("inspect_document", dict(
            document=document, view=dict(kind="paragraph-numbering"),
        ))
        assert wire.isError
        assert set(wire.structuredContent) == {"category", "message", "action"}
        assert wire.structuredContent["category"] == "invalid_input"
        assert wire.structuredContent["action"] == (
            "Use tools/list to check required fields and allowed values."
        )
        decoded = await call(client, "inspect_document", document=document, view=dict(
            kind="inline-at", inline_handle=run["inline_handle"],
        ))
        assert decoded["inline"]["text"] == "Original"


async def test_partial_readbacks_keep_native_diagnostics_without_failing(launch, tmp_path):
    clean = tmp_path / "source.docx"
    source = tmp_path / "partial.docx"
    write_source(clean)
    fixture = docx.Document(clean)
    body = fixture.paragraphs[0]
    body._p.append(OxmlElement("w:fldSimple"))  # Deliberately missing w:instr.
    cell = fixture.add_table(1, 1).cell(0, 0).paragraphs[0]
    cell.text = "Cell"
    for paragraph in (body, cell):
        paragraph._p.get_or_add_pPr().get_or_add_numPr().get_or_add_numId().val = 1
    defaults = fixture.styles.element.find("w:docDefaults/w:pPrDefault/w:pPr", fixture.styles.element.nsmap)
    defaults.get_or_add_jc().set(qn("w:val"), "start")
    fixture.save(source)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        resources = await call(client, "inspect_document", document=document,
                               view=dict(kind="resources"))
        assert resources["default_alignment"] is None
        assert resources["default_alignment_error"] == "unsupported default alignment"
        assert resources["default_alignment_diagnostic"] == {
            "rule": "unsupported_edit", "detail": resources["default_alignment_error"],
        }
        fields = await call(client, "inspect_document", document=document, view=dict(kind="fields"))
        field, = fields["fields"]
        assert field["editable"] is False
        assert field["diagnostic"] == {"rule": "unsupported_edit", "detail": field["reason"]}
        numbering = await call(client, "inspect_document", document=document,
                               view=dict(kind="numbering-paragraphs", scope="cells"))
        paragraph, = numbering["paragraphs"]
        detail = "incomplete direct numbering reference"
        assert paragraph["unavailable"] == detail
        assert paragraph["diagnostic"] == {"rule": "unsupported_edit", "detail": detail}
        assert "anchor" not in paragraph
        scoped = await call(client, "inspect_document", document=document, view=dict(kind="body"))
        await assert_inspection_error(client, document, inspection_diagnostic(detail), dict(
            kind="paragraph-numbering", anchor=scoped["body"][0]["anchor"],
        ))


async def test_revision_native_diagnostics_and_retry(launch, tmp_path):
    write_source(tmp_path / "source.docx")

    async with launch() as client:
        document = (await call(client, "open_document", path="source.docx"))["document"]
        body = (await call(client, "inspect_document", document=document))["body"]
        paragraph = body[0]
        anchor = paragraph["runs"][0]["anchor"]

        async def preview(target=anchor, **fields):
            return await client.call_tool("preview_edits", dict(
                document=document,
                operation=dict(kind="edit-revision", anchor=target, edit={
                    "kind": "replace-text", "text": "Updated", "author": "Ada", **fields,
                }),
            ))

        missing_author = await preview(author=" ")
        assert missing_author.isError
        assert missing_author.structuredContent == {
            "category": "rejected_batch",
            "message": f"author: revision author cannot be empty; {BATCH_HINT}",
            "action": BATCH_ACTION,
            "rule": "invalid_content",
            "path": "author",
            "detail": "revision author cannot be empty",
        }

        native_file = tmp_path / "invalid-xml-author.json"
        revision_oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_revision_oracle")
        subprocess.run([revision_oracle, "invalid-xml-author", native_file, tmp_path / "source.docx"],
                       check=True, timeout=20, cwd=tmp_path)
        native = json.loads(native_file.read_bytes())
        assert native == {
            "character": "\0", "byte_offset": 6, "rule": "invalid_xml", "path": None,
            "message": "invalid XML 1.0 character U+0000 at byte offset 6",
        }
        invalid_xml = await preview(author="é😀\0")
        assert invalid_xml.isError
        assert invalid_xml.structuredContent == {
            "category": "rejected_batch",
            "message": f"{native['message']}; {BATCH_HINT}",
            "action": BATCH_ACTION,
            "rule": native["rule"],
            "character": native["character"],
            "byte_offset": native["byte_offset"],
        }

        multiline = await preview(text="First\nSecond")
        assert multiline.isError
        assert multiline.structuredContent == {
            "category": "rejected_batch",
            "message": f"tracked text must be nonempty single-line text; {BATCH_HINT}",
            "action": BATCH_ACTION,
            "rule": "unsupported_edit",
            "detail": "tracked text must be nonempty single-line text",
        }

        unpaired = await client.call_tool("preview_edits", dict(
            document=document,
            operation=dict(kind="edit-revision", anchor=anchor, edit=dict(
                kind="move-text", name="Move1", author="Ada", date="2030-02-03T04:05:06Z",
            )),
        ))
        assert unpaired.isError
        assert unpaired.structuredContent == {
            "category": "rejected_batch",
            "message": f"move-text requires paired_anchor; {BATCH_HINT}",
            "action": BATCH_ACTION,
        }

        wrong_target = await preview(target=paragraph["anchor"])
        assert wrong_target.isError
        assert wrong_target.structuredContent == {
            "category": "wrong_anchor_type",
            "message": "revision operation expects a direct body run",
            "action": "Inspect the document again and use an anchor of the required type.",
        }

        accepted = await preview()
        assert not accepted.isError
        assert accepted.structuredContent["preview"]


async def test_ordinary_batch_native_diagnostic_and_retry(launch, tmp_path):
    write_source(tmp_path / "source.docx")
    async with launch() as client:
        document = (await call(client, "open_document", path="source.docx"))["document"]
        body = (await call(client, "inspect_document", document=document))["body"]
        anchor = body[0]["anchor"]
        alignment = dict(kind="alignment", anchor=anchor, alignment="center")
        detail = "multiple alignment edits target one paragraph"
        conflict = await client.call_tool("preview_edits", dict(
            document=document, edits=[alignment, dict(alignment, alignment="right")],
        ))
        assert conflict.isError
        assert conflict.structuredContent == {
            "category": "rejected_batch",
            "message": f"preview: {detail}; revise the edits and retry",
            "action": BATCH_ACTION,
            "rule": "unsupported_edit",
            "detail": detail,
        }

        wrong_target = await client.call_tool("preview_edits", dict(
            document=document, edits=[dict(kind="run-remove", anchor=anchor)],
        ))
        assert wrong_target.isError
        assert wrong_target.structuredContent == {
            "category": "wrong_anchor_type",
            "message": "edit 1: expected run",
            "action": "Inspect the document again and use an anchor of the required type.",
        }
        assert (await call(client, "preview_edits", document=document, edits=[alignment]))["preview"]


@pytest.mark.parametrize("absolute", [False, True], ids=["relative", "absolute"])
async def test_open_native_diagnostics_do_not_change_filesystem_errors(launch, tmp_path, absolute):
    write_source(tmp_path / "source.docx")
    (tmp_path / "malformed.docx").write_bytes(b"not a DOCX")
    replace_part(tmp_path / "source.docx", tmp_path / "wrong-relationships.docx",
                 "_rels/.rels", b'<Relationships xmlns="urn:wrong-package-relationships"/>')
    (tmp_path / "directory.docx").mkdir()

    def path(name):
        return str(tmp_path / name) if absolute else name

    async with launch() as client:
        detail = "package part count exceeds read limit"
        limited = await client.call_tool("open_document", dict(
            path=path("source.docx"), limits=dict(max_parts="1"),
        ))
        assert limited.isError
        assert limited.structuredContent == {
            "category": "invalid_document_data",
            "message": f"package: {detail}",
            "action": IMPORT_ACTION,
            "rule": "invalid_package",
            "path": "package",
            "detail": detail,
        }

        malformed = await client.call_tool("open_document", dict(path=path("malformed.docx")))
        assert malformed.isError
        assert malformed.structuredContent["message"].startswith("DOCX packaging failed: ")
        assert set(malformed.structuredContent) == {"category", "message", "action", "rule"}
        assert malformed.structuredContent["category"] == "invalid_document_data"
        assert malformed.structuredContent["rule"] == "invalid_package"
        assert malformed.structuredContent["action"] == IMPORT_ACTION

        relationship = await client.call_tool("open_document", dict(path=path("wrong-relationships.docx")))
        assert relationship.isError
        assert relationship.structuredContent == {
            "category": "invalid_document_data",
            "message": "wrong relationship-part namespace",
            "action": IMPORT_ACTION,
            "rule": "invalid_relationship",
            "detail": "wrong relationship-part namespace",
        }

        for name, prefix in (
            ("missing.docx", "cannot open source: "),
            ("directory.docx", "cannot read source: "),
        ):
            failure = await client.call_tool("open_document", dict(path=path(name)))
            assert failure.isError
            assert failure.structuredContent["message"].startswith(prefix)
            assert set(failure.structuredContent) == {"category", "message", "action"}
            assert failure.structuredContent["category"] == "io"
            assert failure.structuredContent["action"] == IO_ACTION

        invalid = await client.call_tool("open_document", dict(path="/" if absolute else ""))
        assert invalid.isError
        assert invalid.structuredContent == {
            "category": "invalid_path",
            "message": "supply a path with a file name",
            "action": IO_ACTION,
        }

        assert (await call(client, "open_document", path=path("source.docx")))["document"]


async def test_scoped_entry_native_rejection(launch, tmp_path):
    source = tmp_path / "entries.docx"
    make_entry_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        entries = await call(client, "inspect_document", document=document,
                             view=dict(kind="entries"))
        comment = next(row["anchor"] for row in entries["entries"] if row["kind"] == "comment")
        result = await client.call_tool("preview_edits", dict(document=document, operation=dict(
            kind="edit-entry", anchor=comment, edit=dict(kind="remove-paragraph", index=99),
        )))
        native_rejection(result, "invalid block index or protected first note paragraph")


async def test_scoped_layout_and_run_format_native_rejections(launch, tmp_path):
    write_source(tmp_path / "source.docx")
    async with launch() as client:
        document = (await call(client, "open_document", path="source.docx"))["document"]
        layout = await call(client, "inspect_document", document=document, view=dict(kind="layout"))
        paragraph = next(row["anchor"] for row in layout["layout"] if row["kind"] == "paragraph")
        align = dict(anchor=paragraph, field="alignment", value="center")
        layout_failure = await client.call_tool("preview_edits", dict(
            document=document, operation=dict(kind="edit-layout", edits=[align, align]),
        ))
        native_rejection(layout_failure, "duplicate layout field", "revise the layout edits and retry")

        formats = await call(client, "inspect_document", document=document,
                             view=dict(kind="format-runs"))
        run = formats["format_runs"][0]
        bold = dict(anchor=run["anchor"], edit=dict(kind="set-bold", value=True))
        operation = dict(kind="edit-run-format", anchor=run["owner"]["anchor"], edits=[bold, bold])
        result = await client.call_tool("preview_edits", dict(document=document, operation=operation))
        native_rejection(result, "duplicate or conflicting run format property")

        retry = await call(client, "preview_edits", document=document,
                           operation=dict(operation, edits=[bold]))
        assert retry["preview"]


async def test_story_reference_blocks_and_text_native_rejection_retry(launch, tmp_path):
    source = tmp_path / "story.docx"
    write_source(source)
    fixture = docx.Document(source)
    fixture.sections[0].header.paragraphs[0].text = "Header text"
    fixture.save(source)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]

        async def reject_and_retry(operation, retry, detail):
            result = await client.call_tool("preview_edits", dict(
                document=document, operation=operation,
            ))
            native_rejection(result, detail)
            assert (await call(client, "preview_edits", document=document,
                               operation=retry))["preview"]

        sections = await call(client, "inspect_document", document=document,
                              view=dict(kind="sections"))
        footer = next(slot for slot in sections["sections"][0]["story_slots"]
                      if (slot["kind"], slot["variant"]) == ("footer", "default"))
        reference = dict(kind="edit-story-reference", anchor=footer["anchor"])
        await reject_and_retry(
            dict(reference, edit=dict(kind="remove")),
            dict(reference, edit=dict(kind="create", paragraphs=["New footer"])),
            "section slot has no explicit reference",
        )

        stories = await call(client, "inspect_document", document=document,
                             view=dict(kind="stories"))
        header = next(row for row in stories["stories"]
                      if (row["kind"], row["variant"]) == ("header", "default"))
        block = dict(mode="shared", index=0, remove=1, blocks=[
            dict(kind="paragraph", paragraph=dict(inlines=[dict(kind="text", text="New header")])),
        ])
        splice = dict(kind="edit-story-blocks", anchor=header["anchor"], edit=block)
        await reject_and_retry(dict(splice, edit=dict(block, index=99)), splice,
                               "story block index out of bounds")

        run = header["blocks"][0]["inlines"][0]
        text = dict(anchor=run["anchor"], text="Updated header")
        edit_text = dict(kind="edit-story-text", anchor=header["owner"]["anchor"], edits=[text])
        await reject_and_retry(dict(edit_text, edits=[text, text]), edit_text,
                               "duplicate story text target")


async def test_entry_run_story_text_native_rejection_retry(launch, tmp_path):
    source = tmp_path / "entry-runs.docx"
    make_entry_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        page = await call(client, "inspect_document", document=document,
                          view=dict(kind="entry-runs"))
        row = next(row for row in page["entry_runs"] if row["kind"] == "comment")
        run = next(run for run in row["runs"] if "anchor" in run)
        text = dict(anchor=run["anchor"], text="Updated comment")
        operation = dict(kind="edit-story-text", anchor=row["owner"]["anchor"], edits=[text, text])
        result = await client.call_tool("preview_edits", dict(document=document, operation=operation))
        native_rejection(result, "duplicate story text target")
        assert (await call(client, "preview_edits", document=document,
                           operation=dict(operation, edits=[text])))["preview"]
