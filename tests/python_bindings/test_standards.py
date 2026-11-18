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
