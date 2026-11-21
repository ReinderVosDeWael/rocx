"""Balanced field encodings, cached display content, ownership and edit boundaries."""
import io
import pathlib
import zipfile

import docx
from lxml import etree
import pytest
from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
XML = "{http://www.w3.org/XML/1998/namespace}"


def declaration() -> d.Document:
    """Include both forms, nested results, a typed REF and a TOC request."""
    return d.Document([d.Section(children=[d.Paragraph(children=[
        d.RangeStart.bookmark("target"), d.TextRun("Target"), d.RangeEnd.bookmark("target"),
        d.Field(d.FieldInstruction.page(), [d.TextRun("7", bold=True)]),
        d.Field(d.FieldInstruction.raw('  IF 1 = 1 "yes" "no"  '), [d.TextRun("cached "),
            d.Field(d.FieldInstruction.num_pages(), [d.TextRun("12")], form="complex")],
            form="complex", dirty=True, locked=False),
        d.Field(d.FieldInstruction.reference("target"), [d.TextRun("Target")]),
        d.Field(d.FieldInstruction.table_of_contents(), [d.TextRun("Contents pending")], dirty=True),
    ])])], timestamp="2026-11-18T00:00:00Z")


async def test_field_encodings_preserve_flags_whitespace_and_nested_cached_results() -> None:
    snapshot = await declaration().resolve()
    data = await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert [n.text for n in root.iter(f"{W}instrText")] == ['  IF 1 = 1 "yes" "no"  ', ' NUMPAGES ']
    assert all(n.get(f"{XML}space") == "preserve" for n in root.iter(f"{W}instrText"))
    marks = list(root.iter(f"{W}fldChar"))
    assert [n.get(f"{W}fldCharType") for n in marks] == ["begin", "separate", "begin", "separate", "end", "end"]
    assert marks[0].get(f"{W}dirty") == "1"
    assert marks[0].get(f"{W}fldLock") == "0"
    simple = list(root.iter(f"{W}fldSimple"))
    assert [n.get(f"{W}instr") for n in simple] == [" PAGE ", " REF target ", ' TOC \\o "1-3" \\h \\z \\u ']
    assert simple[0].get(f"{W}dirty") is None
    assert simple[0].find(f".//{W}b") is not None
    assert "".join(n.text or "" for n in root.iter(f"{W}t")) == "Target7cached 12TargetContents pending"
    # python-docx retains the field XML even though its paragraph.text omits fldSimple results.
    output = docx.Document(io.BytesIO(data))
    assert len(list(output._element.iter(f"{W}fldSimple"))) == 3
    assert data == await snapshot.to_bytes()


async def test_field_properties_use_generic_deferred_references_and_hidden_fields_do_not_run() -> None:
    source = d.Field(instruction=lambda: d.FieldInstruction.page(), children=lambda: [d.TextRun("captured")],
                     form=lambda: "complex", dirty=lambda: False, locked=lambda: True)
    shared = d.Field(source.deferred("instruction"), source.deferred("children"),
                     form=source.deferred("form"), dirty=source.deferred("dirty"), locked=source.deferred("locked"))
    hidden = d.Field(lambda: (_ for _ in ()).throw(AssertionError("hidden field ran")))
    hidden.condition = False
    data = await d.Document([d.Section(children=[d.Paragraph(children=[shared, hidden])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    begin = next(root.iter(f"{W}fldChar"))
    assert begin.get(f"{W}dirty") == "0"
    assert begin.get(f"{W}fldLock") == "1"


@pytest.mark.parametrize("value", [
    d.Field(d.FieldInstruction.reference("absent")),
    d.Field(d.FieldInstruction.raw("  ")),
    d.Field(d.FieldInstruction.raw("PAGE\0")),
    d.Field(d.FieldInstruction.page(), form="invalid"),
    d.Field(d.FieldInstruction.page(), [d.RangeStart.bookmark("inside")]),
    d.Hyperlink("https://example.invalid", [d.Field(d.FieldInstruction.page())]),
])
async def test_invalid_instructions_targets_and_container_boundaries_fail(value: object) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[value])])]).to_bytes()
    assert error.value.location


@pytest.mark.parametrize("form", ["simple", "complex"])
async def test_imported_field_results_are_protected_from_template_replacements(tmp_path: pathlib.Path, form: str) -> None:
    source = d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("outside"),
        d.Field(d.FieldInstruction.page(), [d.TextRun("cached")], form=form)])])])
    template = tmp_path / "fields.docx"
    template.write_bytes(await source.to_bytes())
    with pytest.raises(ValueError) as error:
        await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"cached": "changed"}))
    assert error.value.rule == "unsupported_edit"
    data = await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"outside": "changed"}))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    assert [n.text for n in root.iter(f"{W}t")] == ["changed", "cached"]


@pytest.mark.parametrize("form", ["simple", "complex"])
async def test_raw_instruction_xml_whitespace_is_not_normalized(form: str) -> None:
    instruction = ' \tIF\r\n "λ < & >" '
    field = d.Field(d.FieldInstruction.raw(instruction), form=form)
    data = await d.Document([d.Section(children=[d.Paragraph(children=[field])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
    if form == "simple":
        assert next(root.iter(f"{W}fldSimple")).get(f"{W}instr") == instruction
    else:
        assert next(root.iter(f"{W}instrText")).text == instruction


async def test_typed_reference_can_target_imported_bookmarks_but_not_ambiguous_names(tmp_path: pathlib.Path) -> None:
    import test_notes
    template = tmp_path / "bookmark.docx"
    source = await declaration().to_bytes()
    template.write_bytes(source)
    document = d.Document([d.Section(children=[d.Paragraph(children=[
        d.Field(d.FieldInstruction.reference("target"), [d.TextRun("Imported target")])])])])
    assert await document.to_bytes(d.DocumentTemplate(template))
    def duplicate(root: etree._Element) -> None:
        paragraph = root.find(f".//{W}p")
        etree.SubElement(paragraph, f"{W}bookmarkStart", {f"{W}id": "999", f"{W}name": "target"})
        etree.SubElement(paragraph, f"{W}bookmarkEnd", {f"{W}id": "999"})
    template.write_bytes(test_notes.rewrite(source, "word/document.xml", duplicate))
    with pytest.raises(ValueError) as error:
        await document.to_bytes(d.DocumentTemplate(template))
    assert error.value.rule == "invalid_relationship"
    assert "ambiguous" in str(error.value)


async def test_cached_fields_capture_images_and_keep_header_resources_with_the_owner(tmp_path: pathlib.Path) -> None:
    image = tmp_path / "field.png"
    expected = (pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    image.write_bytes(expected)
    field = d.Field(d.FieldInstruction.page(), [d.ImageRun(data=image),
        d.Hyperlink("https://example.invalid/field", [d.TextRun("cached link")])])
    snapshot = await d.Document([d.Section(children=[], headers={"default": d.Header(children=[
        d.Paragraph(children=[field])])})]).resolve()
    image.unlink()
    data = await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/header1.xml"))
        assert root.find(f".//{W}fldSimple/{W}hyperlink") is not None
        rels = etree.fromstring(package.read("word/_rels/header1.xml.rels"))
        assert any(n.get("Target") == "https://example.invalid/field" for n in rels)
        assert any(package.read(f"word/{n.get('Target')}") == expected for n in rels if n.get("Type").endswith("/image"))
    assert data == await snapshot.to_bytes()


async def test_imported_complex_fields_spanning_paragraphs_still_protect_cached_results(tmp_path: pathlib.Path) -> None:
    import test_notes
    source = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Field(d.FieldInstruction.page(), [d.TextRun("cached")], form="complex")]),
        d.Paragraph(text="outside")])]).to_bytes()
    def split_field(root: etree._Element) -> None:
        body = root.find(f"{W}body")
        first = body.find(f"{W}p")
        second = etree.Element(f"{W}p")
        for run in list(first)[3:]:
            first.remove(run)
            second.append(run)
        etree.SubElement(etree.SubElement(second, f"{W}r"), f"{W}t").text = "suffix"
        body.insert(1, second)
    template = tmp_path / "multi-paragraph-field.docx"
    template.write_bytes(test_notes.rewrite(source, "word/document.xml", split_field))
    with pytest.raises(ValueError) as error:
        await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"cached": "changed"}))
    assert error.value.rule == "unsupported_edit"
    assert await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"outside": "changed"}))
    assert await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"suffix": "changed"}))


async def test_imported_field_depth_does_not_leak_between_notes_sharing_a_part(tmp_path: pathlib.Path) -> None:
    import test_notes
    source = await d.Document([], notes=[
        d.Note.footnote(1, [d.Paragraph(children=[d.Field(d.FieldInstruction.page(), [d.TextRun("first")], form="complex")])]),
        d.Note.footnote(2, [d.Paragraph(text="second")]),
    ]).to_bytes()
    def unclose(root: etree._Element) -> None:
        end = next(n for n in root.iter(f"{W}fldChar") if n.get(f"{W}fldCharType") == "end")
        end.getparent().remove(end)
    template = tmp_path / "note-fields.docx"
    template.write_bytes(test_notes.rewrite(source, "word/footnotes.xml", unclose))
    with pytest.raises(ValueError):
        await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"first": "changed"}, scope="supported_stories"))
    data = await d.Document([]).to_bytes(d.DocumentTemplate(template, replacements={"second": "changed"}, scope="supported_stories"))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert b"changed" in package.read("word/footnotes.xml")


async def test_nested_cached_fields_still_validate_references_and_story_contexts() -> None:
    nested = d.Field(d.FieldInstruction.page(), [d.Field(d.FieldInstruction.reference("absent"))])
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[nested])])]).to_bytes()
    assert "children[0].children[0]" in error.value.location
    note = d.Note.footnote(0, [d.Paragraph(children=[d.Field(d.FieldInstruction.page(), [d.NoteReference.footnote(0)])])])
    with pytest.raises(ValueError) as error:
        await d.Document([], notes=[note]).to_bytes()
    assert error.value.rule == "invalid_content"
