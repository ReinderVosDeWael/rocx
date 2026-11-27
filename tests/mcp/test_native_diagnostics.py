"""Native revision diagnostics over the installed stdio transport."""

import docx
from docx.oxml.ns import qn
import pytest

from test_stdio import anyio_backend, call, launch  # noqa: F401 (shared fixtures)

pytestmark = pytest.mark.anyio

BATCH_ACTION = "Check the edit targets and values, then retry the batch."
BATCH_HINT = "revise the operation and retry"
IMPORT_ACTION = "Check that the input is a valid DOCX and fits the supplied import limits."
IO_ACTION = "Check the server-host path and permissions before retrying."


def write_source(path):
    source = docx.Document()
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    source.add_paragraph("Original")
    source.save(path)


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
