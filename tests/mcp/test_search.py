"""Exercise bounded text discovery through the installed MCP stdio server."""

from zipfile import ZipFile

import docx
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for

from test_stdio import anyio_backend, call, failure, launch, pytestmark


async def test_sparse_search_uses_snapshot_order_and_bounded_unicode_context(launch, tmp_path):
    source = tmp_path / "sparse.docx"
    fixture = docx.Document()
    noise = "Unrelated paragraph " + "Z" * 150
    for index in range(1500):
        paragraph = fixture.add_paragraph()
        if index == 11:
            paragraph.add_run("😀" * 70 + "green ").bold = True
            paragraph.add_run("planet").italic = True
            paragraph.add_run("🪐" * 70)
        elif index == 1499:
            paragraph.add_run("green planet, 🤖green planet")
        else:
            paragraph.add_run(noise)
    fixture.save(source)
    with ZipFile(source) as package:
        total = sum(member.file_size for member in package.infolist())

    async with launch() as client:
        await failure(client, "open_document", "invalid_document_data", path=source.name,
                      limits={"max_expanded_bytes": str(total - 1)})
        document = (await call(client, "open_document", path=source.name,
                               limits={"max_expanded_bytes": str(total)}))["document"]
        source.unlink()  # Search the retained snapshot, not the source path.
        search = {"document": document, "query": "green planet"}
        first = await call(client, "search_document", **search, limit=1)
        assert first["document"] == document
        assert first["hits"] == [{
            "body_index": 11, "start": 70, "end": 82,
            "snippet_start": 46, "snippet": "😀" * 24 + "green planet" + "🪐" * 24,
        }]
        assert first["next_cursor"]
        second = await call(client, "search_document", **search, limit=1,
                            cursor=first["next_cursor"])
        assert second["hits"] == [{
            "body_index": 1499, "start": 0, "end": 12,
            "snippet_start": 0, "snippet": "green planet, 🤖green planet",
        }]
        assert second["next_cursor"]
        last = await call(client, "search_document", **search, limit=100,
                          cursor=second["next_cursor"])
        assert last["hits"] == [{
            "body_index": 1499, "start": 15, "end": 27,
            "snippet_start": 0, "snippet": "green planet, 🤖green planet",
        }]
        assert last["next_cursor"] is None
        assert await call(client, "search_document", **search, limit=3) == {
            "document": document,
            "hits": first["hits"] + second["hits"] + last["hits"],
            "next_cursor": None,
        }
        assert await call(client, "search_document", document=document, query="Green planet") == {
            "document": document, "hits": [], "next_cursor": None,
        }


def add_inline(paragraph, tag, text, attributes=None):
    element = OxmlElement(tag)
    for name, value in (attributes or {}).items():
        element.set(qn(name), value)
    if tag == "w:sdt":
        content = OxmlElement("w:sdtContent")
        element.append(content)
    else:
        content = element
    run = OxmlElement("w:r")
    value = OxmlElement("w:t")
    value.text = text
    run.append(value)
    content.append(run)
    paragraph._p.append(element)


async def test_search_uses_only_compact_body_paragraph_projection(launch, tmp_path):
    source = tmp_path / "scope.docx"
    fixture = docx.Document()
    fixture.sections[0].header.paragraphs[0].text = "header-only"
    fixture.sections[0].footer.paragraphs[0].text = "footer-only"
    left = fixture.add_paragraph("left boundary")
    fixture.add_comment(left.runs, text="comment-only", author="Tester")
    fixture.add_paragraph("right boundary")
    fixture.add_table(rows=1, cols=1).cell(0, 0).text = "table-only"
    paragraph = fixture.add_paragraph("Go ")
    add_inline(paragraph, "w:hyperlink", "home", {"w:anchor": "destination"})
    paragraph.add_run("\t[.]\nGo\u0301")
    add_inline(paragraph, "w:ins", "revision-only", {"w:id": "7", "w:author": "tester"})
    add_inline(paragraph, "w:sdt", "control-only")
    wrapped = fixture.add_paragraph("wrapped-only")
    control = OxmlElement("w:sdt")
    content = OxmlElement("w:sdtContent")
    control.append(content)
    wrapped._p.addprevious(control)
    content.append(wrapped._p)
    fixture.save(source)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        for query in ("boundaryright", "boundary\nright", "table-only", "header-only",
                      "footer-only", "comment-only", "revision-only", "control-only",
                      "wrapped-only", "Gó"):
            assert await call(client, "search_document", document=document, query=query) == {
                "document": document, "hits": [], "next_cursor": None,
            }

        compact = await call(client, "inspect_document", document=document, view={"kind": "body"})
        paragraph = compact["body"][3]
        assert paragraph["text"] == "Go home\t[.]\nGo\u0301"
        for query, start in (("Go home", 0), ("\t[.]\n", 7), ("Go\u0301", 12), (" ", 2)):
            page = await call(client, "search_document", document=document, query=query)
            hit = next(hit for hit in page["hits"] if hit["body_index"] == 3)
            assert hit == {
                "body_index": paragraph["index"], "start": start, "end": start + len(query),
                "snippet_start": 0, "snippet": paragraph["text"],
            }


async def test_search_discovery_pagination_and_cursor_guards(launch):
    async with launch() as client:
        listing = await client.list_tools()
        tool = next(tool for tool in listing.tools if tool.name == "search_document")
        assert tool.annotations.readOnlyHint and not tool.annotations.openWorldHint
        schema = tool.inputSchema
        assert schema["properties"]["query"]["minLength"] == 1
        assert schema["properties"]["query"]["maxLength"] == 128
        assert schema["properties"]["limit"]["default"] == 25
        assert schema["properties"]["limit"]["minimum"] == 1
        assert schema["properties"]["limit"]["maximum"] == 100
        validate = validator_for(schema)(schema)

        paragraphs = ["a" * 220, "aa"]
        original = await call(client, "create_document", paragraphs=paragraphs)
        document = original["document"]
        other = (await call(client, "create_document", paragraphs=paragraphs))["document"]
        search = {"document": document, "query": "aa"}
        for dispatch in (
            {"document": document}, {"query": "aa"}, {**search, "query": ""},
            {**search, "query": "😀" * 129}, {**search, "query": 3}, {**search, "limit": 0},
            {**search, "limit": 101}, {**search, "limit": 1.5}, {**search, "cursor": None},
            {**search, "path": "ignored.docx"},
        ):
            assert not validate.is_valid(dispatch)
            await failure(client, "search_document", "invalid_input", **dispatch)
        maximum = {**search, "query": "😀" * 128}
        assert validate.is_valid(maximum)
        assert not (await call(client, "search_document", **maximum))["hits"]

        first = await call(client, "search_document", **search)
        assert [hit["start"] for hit in first["hits"]] == list(range(0, 50, 2))
        capped = await call(client, "search_document", **search, limit=100)
        assert [hit["start"] for hit in capped["hits"]] == list(range(0, 200, 2))
        cursor = first["next_cursor"]
        assert cursor and capped["next_cursor"]
        await failure(client, "search_document", "invalid_cursor", **{**search, "query": "AA"},
                      cursor=cursor)
        await failure(client, "search_document", "wrong_owner", **{**search, "document": other},
                      cursor=cursor)
        await failure(client, "search_document", "invalid_cursor", **search, cursor="garbage")
        await failure(client, "search_document", "invalid_cursor", **search,
                      cursor=f"{document}/00000000-0000-0000-0000-000000000000")
        inspection = await call(client, "inspect_document", document=document,
                                view={"kind": "body-inlines", "limit": 1})
        assert inspection["next_cursor"]
        await failure(client, "search_document", "invalid_cursor", **search,
                      cursor=inspection["next_cursor"])
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view={"kind": "body-inlines", "cursor": cursor})
        rest = await call(client, "search_document", **search, limit=100, cursor=cursor)
        assert [hit["start"] for hit in rest["hits"]] == [*range(50, 220, 2), 0]
        assert rest["hits"][-1]["body_index"] == 1 and rest["next_cursor"] is None

        await call(client, "close_document", document=document)
        await failure(client, "search_document", "invalid_document", **search, cursor=cursor)
