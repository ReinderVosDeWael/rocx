"""Independent XSD validation using official, edition-pinned schemas (dev only)."""
import importlib.util
import io
import os
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

from lxml import etree
import pytest
from rv_docx import declarative as d

ROOT = Path(__file__).parents[2]
spec = importlib.util.spec_from_file_location("ooxml_validator", ROOT / "tools/validate_ooxml.py")
validation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validation)


@pytest.fixture(scope="module")
def validator():
    directory = validation.default_directory()
    if not (directory / "wml.xsd").exists():
        if os.environ.get("RV_DOCX_REQUIRE_SCHEMAS") == "1":
            pytest.fail("Required official schemas missing; run tools/validate_ooxml.py --download-schemas")
        pytest.skip("Download pinned schemas with tools/validate_ooxml.py --download-schemas or set RV_DOCX_SCHEMA_DIR")
    return validation.Validator(directory)


async def test_official_schema_accepts_native_content_and_detects_invalid_order(validator):
    document = d.Document([d.Section(children=[d.Paragraph(text="schema", style="Normal", alignment="center")])])
    data = await document.to_bytes()
    assert "word/document.xml" in validator.validate(data)
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/document.xml":
                tree = etree.fromstring(payload)
                properties = tree.find(f".//{{{validation.W}}}pPr")
                properties[:] = list(reversed(properties))
                payload = etree.tostring(tree)
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(output.getvalue())


async def test_official_schema_accepts_features_and_rejects_missing_relationship_targets(validator):
    image = ROOT / "tests/fixtures/images/sample.png"
    document = d.Document([d.Section(children=[d.Paragraph(children=[d.ImageRun(data=image)])],
        headers={"even": d.Header(children=[d.Paragraph(text="even")])})])
    data = await document.to_bytes()
    assert len(validator.validate(data)) >= 5
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/_rels/document.xml.rels":
                payload = payload.replace(b"media/image1.png", b"media/missing.png")
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="missing relationship target"):
        validator.validate(output.getvalue())


async def test_numbering_uses_the_shared_native_domain(validator):
    document = d.Document([d.Section(children=[d.Paragraph(text="numbered", numbering=d.NumberingReference(42))])],
        numbering=[d.NumberingDefinition(42, [d.NumberingLevel(start=3)])])
    data = await document.to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    with ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{{{validation.W}}}numId").get(f"{{{validation.W}}}val") == "42"
        root = etree.fromstring(package.read("word/numbering.xml"))
        assert root.find(f"{{{validation.W}}}num[@{{{validation.W}}}numId='42']") is not None


async def test_simple_and_nested_complex_fields_pass_schemas_and_balance_checks(validator):
    import test_fields
    import test_notes
    data = await test_fields.declaration().to_bytes()
    assert "word/document.xml" in validator.validate(data)
    image = ROOT / "tests/fixtures/images/sample.png"
    for form in ["simple", "complex"]:
        cached = d.Field(d.FieldInstruction.raw(' PAGE \\* MERGEFORMAT '), [
            d.TextRun("cached", bold=True), d.Tab(), d.Break(), d.ImageRun(data=image),
            d.Hyperlink("https://example.invalid/field", [d.TextRun("link")]),
        ], form=form, dirty=False, locked=True)
        rich = d.Document([d.Section(children=[d.Paragraph(children=[
            d.NoteReference.footnote(0), d.RangeStart.structured_comment("review", d.Comment([d.Paragraph(children=[cached])])),
            d.TextRun("body"), d.RangeEnd("review"),
        ])], headers={"default": d.Header(children=[d.Paragraph(children=[cached])])})],
            notes=[d.Note.footnote(0, [d.Paragraph(children=[cached])])])
        assert {"word/header1.xml", "word/footnotes.xml", "word/comments.xml"} <= set(validator.validate(await rich.to_bytes()))
    def unclosed(root):
        ends = [n for n in root.iter(f"{{{validation.W}}}fldChar") if n.get(f"{{{validation.W}}}fldCharType") == "end"]
        ends[-1].getparent().remove(ends[-1])
    def duplicate_separator(root):
        marks = list(root.iter(f"{{{validation.W}}}fldChar"))
        marks[2].set(f"{{{validation.W}}}fldCharType", "separate")
    for mutation in [unclosed, duplicate_separator]:
        with pytest.raises(validation.ValidationError, match="field"):
            validator.validate(test_notes.rewrite(data, "word/document.xml", mutation))


async def test_external_hyperlinks_and_display_runs_pass_official_content_models(validator):
    def paragraph(with_comment=False):
        return d.Paragraph(children=[d.Hyperlink("https://example.invalid/λ?q=a%20b&x=2#frag", [
            d.TextRun("link", bold=True, comment_text="link note" if with_comment else None), d.Tab(), d.Break(),
            d.ImageRun(data=ROOT / "tests/fixtures/images/sample.png")], tooltip="A <&> λ", history=True)])
    data = await d.Document([d.Section(children=[paragraph(True), d.Table(rows=[d.TableRow(children=[
        d.TableCell(children=[paragraph()])])])], headers={"default": d.Header(children=[paragraph()])},
        footers={"default": d.Footer(children=[paragraph()])})]).to_bytes()
    covered = validator.validate(data)
    assert "word/document.xml" in covered
    assert "word/header1.xml" in covered
    assert "word/footer1.xml" in covered
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(payload)
                link = root.find(f".//{{{validation.W}}}hyperlink")
                link.append(etree.Element(f"{{{validation.W}}}p"))
                payload = etree.tostring(root)
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(output.getvalue())


async def test_overlapping_bookmarks_cross_paragraph_comments_and_internal_links_pass_schemas(validator):
    data = await d.Document([d.Section(children=[
        d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("beta"), [d.TextRun("jump")]),
            d.RangeStart.bookmark("alpha"), d.TextRun("A"), d.RangeStart.comment("review", "note", "Ada")]),
        d.Paragraph(children=[d.RangeStart.bookmark("beta"), d.TextRun("B"),d.RangeEnd.bookmark("alpha"),
            d.TextRun("C"),d.RangeEnd("review"),d.RangeEnd.bookmark("beta")]),
        d.Table(rows=[d.TableRow(children=[
            d.TableCell(children=[d.Paragraph(children=[d.RangeStart.bookmark("table"),d.TextRun("first")])]),
            d.TableCell(children=[d.Paragraph(children=[d.TextRun("second"),d.RangeEnd.bookmark("table")])])])])],
        headers={"default":d.Header(children=[d.Paragraph(children=[d.RangeStart.bookmark("header"),
            d.TextRun("header"),d.RangeEnd.bookmark("header")])])})]).to_bytes()
    covered = validator.validate(data)
    assert "word/document.xml" in covered
    assert "word/comments.xml" in covered
    assert "word/header1.xml" in covered


async def test_notes_and_structured_comments_pass_schemas_and_semantic_graph_checks(validator):
    import copy
    import test_notes
    data=await test_notes.declaration().to_bytes()
    covered=validator.validate(data)
    assert {"word/footnotes.xml","word/endnotes.xml","word/comments.xml"} <= set(covered)
    assert "word/footnotes.xml" in validator.validate(test_notes.large_id_package(data))
    extra=io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source,ZipFile(extra,"w",ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload=source.read(name)
            if name=="word/_rels/document.xml.rels":
                root=etree.fromstring(payload)
                etree.SubElement(root,f"{{{validation.REL}}}Relationship",Id="customRelatedNotes",Type="urn:custom/footnotes",Target="custom.xml")
                payload=etree.tostring(root)
            target.writestr(name,payload)
        target.writestr("word/custom.xml",b"<extension/>")
    assert "word/footnotes.xml" in validator.validate(extra.getvalue())
    def missing(root):
        root.remove(next(n for n in root if n.get(f"{{{validation.W}}}id")=="0"))
    def duplicate(root):
        note=copy.deepcopy(next(n for n in root if n.get(f"{{{validation.W}}}id")=="0"))
        note.set(f"{{{validation.W}}}id","+0")
        root.append(note)
    def special(root):
        next(n for n in root if n.get(f"{{{validation.W}}}id")=="0").set(f"{{{validation.W}}}type","separator")
    def nested(root):
        note=next(n for n in root if n.get(f"{{{validation.W}}}id")=="0")
        run=etree.SubElement(note.find(f"{{{validation.W}}}p"),f"{{{validation.W}}}r")
        etree.SubElement(run,f"{{{validation.W}}}endnoteReference").set(f"{{{validation.W}}}id","0")
    for change in [missing,duplicate,special,nested]:
        mutated=test_notes.rewrite(data,"word/footnotes.xml",change)
        with pytest.raises(validation.ValidationError,match="note"):
            validator.validate(mutated)
    def ordinary_settings_target(root):
        root.find(f"{{{validation.W}}}footnotePr/{{{validation.W}}}footnote").set(f"{{{validation.W}}}id","0")
    with pytest.raises(validation.ValidationError,match="special"):
        validator.validate(test_notes.rewrite(data,"word/settings.xml",ordinary_settings_target))
