"""Integration check for the two branded-report examples."""
import subprocess
import sys
from pathlib import Path
from zipfile import ZipFile

from docx import Document


ROOT = Path(__file__).resolve().parents[1]
TEMPLATE = ROOT / "docs/website/branded_report_template.docx"
ASSET = ROOT / "docs/website/branded_report_output.docx"


def test_branded_report(tmp_path):
    before = TEMPLATE.read_bytes()
    rust = tmp_path / "rust.docx"
    python = tmp_path / "python.docx"
    subprocess.run(["cargo", "run", "--locked", "--example", "branded_report", "--", str(rust)],
                   cwd=ROOT, check=True)
    subprocess.run([sys.executable, "examples/branded_report.py", str(python)],
                   cwd=ROOT, check=True)
    assert TEMPLATE.read_bytes() == before
    assert rust.read_bytes() == python.read_bytes() == ASSET.read_bytes()

    source = Document(TEMPLATE)
    result = Document(python)
    assert "{{title}}" in source.paragraphs[0].text
    assert result.paragraphs[0].text == "Quarterly service report"
    assert source.sections[0].header.paragraphs[0].text == "NORTHSTAR  |  SERVICE INSIGHTS"
    assert result.sections[0].header.paragraphs[0].text == source.sections[0].header.paragraphs[0].text
    assert [[cell.text for cell in row.cells] for row in result.tables[0].rows] == [
        ["Measure", "Result"],
        ["On-time deliveries", "96%"],
        ["Customer satisfaction", "4.7 / 5"],
        ["Orders fulfilled", "1,240"],
    ]
    assert result.tables[0].cell(0, 0).paragraphs[0].runs[0].bold
    with ZipFile(TEMPLATE) as original, ZipFile(python) as edited:
        headers = [name for name in original.namelist()
                   if name.startswith("word/header") and name.endswith(".xml")]
        assert headers
        assert all(original.read(name) == edited.read(name) for name in headers)
        assert not any(name.startswith("_xmlsignatures/") for name in original.namelist())


def test_examples_reject_template_as_output():
    before = TEMPLATE.read_bytes()
    for command in (
        ["cargo", "run", "--locked", "--example", "branded_report", "--", str(TEMPLATE)],
        [sys.executable, "examples/branded_report.py", str(TEMPLATE)],
    ):
        result = subprocess.run(command, cwd=ROOT, capture_output=True)
        assert result.returncode != 0
        assert b"output must differ from template" in result.stderr
        assert TEMPLATE.read_bytes() == before
