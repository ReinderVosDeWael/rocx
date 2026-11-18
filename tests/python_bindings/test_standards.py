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
