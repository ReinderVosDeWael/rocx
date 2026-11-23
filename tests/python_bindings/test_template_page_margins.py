"""Page margin patches on independently generated Word templates."""

import importlib.util
import io
import os
from pathlib import Path
from zipfile import ZipFile

import docx
from docx.oxml.ns import qn
from docx.shared import Inches
from lxml import etree
import pytest

from rocx import declarative as d


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ooxml_validator", ROOT / "tools/validate_ooxml.py")
validation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validation)


@pytest.fixture(scope="module")
def validator():
    directory = validation.default_directory()
    if not (directory / "wml.xsd").exists():
        if os.environ.get("ROCX_REQUIRE_SCHEMAS") == "1":
            pytest.fail("Required official schemas missing")
        pytest.skip("Set ROCX_SCHEMA_DIR to the pinned Transitional schemas")
    return validation.Validator(directory)


@pytest.mark.parametrize("existing_margins", [False, True], ids=["absent", "existing"])
async def test_partial_margin_patch_produces_valid_package(tmp_path, validator, existing_margins):
    source = docx.Document()
    source.add_paragraph("Independent template")
    # python-docx's bundled template omits this XSD-required settings attribute.
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    section = source.sections[0]
    if existing_margins:
        section.left_margin = Inches(2)
        section.right_margin = Inches(1.5)
        section.bottom_margin = Inches(2.5)
        section.header_distance = Inches(0.3)
        section.footer_distance = Inches(0.4)
        section._sectPr.pgMar.set(qn("w:gutter"), "123")
    else:
        section._sectPr.remove(section._sectPr.pgMar)
    path = tmp_path / "source.docx"
    source.save(path)
    assert "word/document.xml" in validator.validate(path.read_bytes())

    data = await d.Document([
        d.Section([], properties=d.SectionProperties(page_margins={"top": Inches(1)}))
    ]).to_bytes(d.DocumentTemplate(path))

    assert "word/document.xml" in validator.validate(data)
    result = docx.Document(io.BytesIO(data)).sections[-1]
    assert result.top_margin == Inches(1)
    if existing_margins:
        assert result.left_margin == Inches(2)
        assert result.right_margin == Inches(1.5)
        assert result.bottom_margin == Inches(2.5)
        assert result.header_distance == Inches(0.3)
        assert result.footer_distance == Inches(0.4)
        assert result._sectPr.pgMar.get(qn("w:gutter")) == "123"
    else:
        with ZipFile(io.BytesIO(data)) as package:
            root = etree.fromstring(package.read("word/document.xml"))
        margins = root.find(f".//{{{validation.W}}}body/{{{validation.W}}}sectPr/{{{validation.W}}}pgMar")
        assert {name: margins.get(qn(f"w:{name}")) for name in (
            "top", "right", "bottom", "left", "header", "footer", "gutter"
        )} == {
            "top": "1440", "right": "1800", "bottom": "1440", "left": "1800",
            "header": "720", "footer": "720", "gutter": "0",
        }
