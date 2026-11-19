"""Bookmark/comment anchor pairing is story-scoped, not container nesting."""
import io
import pathlib
import zipfile

import docx
from lxml import etree
import pytest
from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


def declaration() -> d.Document:
    """Create overlapping bookmarks and a cross-paragraph body comment."""
    return d.Document([d.Section(children=[
        d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("beta"), [d.TextRun("jump ")]),
            d.RangeStart.bookmark("alpha"), d.TextRun("alpha "),
            d.RangeStart.comment("review", "range note", "Ada"), d.TextRun("beta")]),
        d.Paragraph(children=[d.RangeStart.bookmark("beta"), d.TextRun("gamma "),
            d.RangeEnd.bookmark("alpha"), d.TextRun("delta "), d.RangeEnd("review"),
            d.TextRun("epsilon"), d.RangeEnd.bookmark("beta")]),
        d.Paragraph(text="legacy", comment_text="legacy note")])])


async def test_overlapping_cross_paragraph_ranges_have_semantic_spans_and_no_external_link() -> None:
    snapshot = await declaration().resolve()
    data = await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert [p.text for p in output.paragraphs] == ["jump alpha beta", "gamma delta epsilon", "legacy"]
    assert output.paragraphs[0].hyperlinks[0].fragment == "beta"
    assert [(c.text, c.author) for c in output.comments] == [("range note", "Ada"), ("legacy note", "")]
    active = {}
    spans = {}
    names = {}
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert b"/hyperlink" not in package.read("word/_rels/document.xml.rels")
    for node in root.iter():
        if node.tag in (f"{W}bookmarkStart", f"{W}commentRangeStart"):
            kind = "bookmark" if node.tag == f"{W}bookmarkStart" else "comment"
            key = (kind, node.get(f"{W}id"))
            active[key] = ""
            names[key] = node.get(f"{W}name")
        elif node.tag == f"{W}t":
            for key in active:
                active[key] += node.text or ""
        elif node.tag in (f"{W}bookmarkEnd", f"{W}commentRangeEnd"):
            kind = "bookmark" if node.tag == f"{W}bookmarkEnd" else "comment"
            key = (kind, node.get(f"{W}id"))
            spans[key] = active.pop(key)
    assert not active
    assert {names[key]: value for key, value in spans.items() if key[0] == "bookmark"} == {
        "alpha": "alpha betagamma ", "beta": "gamma delta epsilon"}
    assert sorted(value for key, value in spans.items() if key[0] == "comment") == ["betagamma delta ", "legacy"]
    assert data == await snapshot.to_bytes()


@pytest.mark.parametrize("children", [
    [d.RangeStart.bookmark("dangling")],
    [d.RangeEnd.bookmark("missing")],
    [d.RangeEnd.bookmark("reverse"), d.RangeStart.bookmark("reverse")],
    [d.RangeStart.bookmark("duplicate"),d.RangeStart.bookmark("duplicate"),d.RangeEnd.bookmark("duplicate")],
    [d.RangeStart.comment("empty", "note"), d.RangeEnd("empty")],
])
async def test_invalid_ranges_expose_structural_locations(children: list[object]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=children)])]).to_bytes()
    assert error.value.rule == "invalid_content"
    assert error.value.location.startswith("sections[0].children[0].children[")


async def test_cross_story_ranges_and_header_comments_are_rejected() -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.RangeStart.bookmark("cross")])],
            headers={"default":d.Header(children=[d.Paragraph(children=[d.RangeEnd.bookmark("cross")])])})]).to_bytes()
    assert error.value.rule == "invalid_content"
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[],headers={"default":d.Header(children=[d.Paragraph(children=[
            d.RangeStart.comment("header", "no"),d.TextRun("header"),d.RangeEnd("header")])])})]).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_missing_internal_target_fails_without_a_dummy_relationship() -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("missing"),[])])])]).to_bytes()
    assert error.value.rule == "invalid_relationship"
    assert error.value.location == "sections[0].children[0].children[0].target"


async def test_range_fields_and_bookmark_target_names_accept_generic_references() -> None:
    source = d.RangeStart(id=lambda:"ref", kind=lambda:"bookmark",name=lambda:"deferred")
    start = d.RangeStart(source.deferred("id"),source.deferred("kind"),name=source.deferred("name"))
    target = d.BookmarkTarget(source.deferred("name"))
    hidden = d.RangeEnd(lambda: (_ for _ in ()).throw(AssertionError("hidden field ran")),condition=False)
    data = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Hyperlink(target,[d.TextRun("jump")]), start,d.TextRun("target"),d.RangeEnd(source.deferred("id")),hidden])])]).to_bytes()
    assert docx.Document(io.BytesIO(data)).paragraphs[0].hyperlinks[0].fragment == "deferred"


async def test_imported_high_ids_are_preserved_and_allocation_limits_apply_only_to_new_ids(tmp_path: pathlib.Path) -> None:
    source = await d.Document([d.Section(children=[d.Paragraph(children=[d.RangeStart.bookmark("old"),
        d.TextRun("old"),d.RangeEnd.bookmark("old")])])]).to_bytes()
    template = tmp_path / "high-ids.docx"
    with zipfile.ZipFile(io.BytesIO(source)) as original, zipfile.ZipFile(template,"w") as output:
        for name in original.namelist():
            payload = original.read(name)
            if name == "word/document.xml":
                payload = payload.replace(b'w:id="0"',b'w:id="4294967296"')
            output.writestr(name,payload)
    data = await d.Document([d.Section(children=[d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("old"),[d.TextRun("jump")])])])]).to_bytes(d.DocumentTemplate(template))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert b'w:id="4294967296"' in package.read("word/document.xml")
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.RangeStart.bookmark("new"),d.RangeEnd.bookmark("new")])])]).to_bytes(d.DocumentTemplate(template))
    assert error.value.rule == "invalid_content"


@pytest.mark.parametrize("identifier", ["-0", "+0", "000", " 0 "])
async def test_imported_decimal_id_spellings_reserve_their_numeric_identity(tmp_path: pathlib.Path, identifier: str) -> None:
    source = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.RangeStart.bookmark("old"),d.RangeEnd.bookmark("old")])])]).to_bytes()
    template = tmp_path / "decimal-ids.docx"
    with zipfile.ZipFile(io.BytesIO(source)) as original, zipfile.ZipFile(template,"w") as output:
        for name in original.namelist():
            payload = original.read(name)
            if name == "word/document.xml":
                payload = payload.replace(b'w:id="0"',f'w:id="{identifier}"'.encode())
            output.writestr(name,payload)
    data = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.RangeStart.bookmark("new"),d.RangeEnd.bookmark("new")])])]).to_bytes(d.DocumentTemplate(template))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    ids = [int(node.get(f"{W}id")) for node in root.iter(f"{W}bookmarkStart")]
    assert len(ids) == len(set(ids)) == 2
