"""Behavioral checks for release artifact smoke helpers."""

import importlib.util
from pathlib import Path
import zipfile

import pytest

SPEC = importlib.util.spec_from_file_location(
    "smoke_release_artifacts", Path(__file__).resolve().parents[1] / "tools/smoke_release_artifacts.py"
)
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


def test_extracts_current_published_quickstarts():
    markdown = smoke.DOC.read_text(encoding="utf-8")
    assert 'TextRun::new("Hello, world!")' in smoke.quickstart(markdown, "rust")
    assert 'TextRun(text="Hello, world!")' in smoke.quickstart(markdown, "python")


def test_rejects_missing_or_duplicate_snippets():
    with pytest.raises(ValueError, match="found 0"):
        smoke.quickstart("no snippets", "python")
    block = "    ```python\n    print('hi')\n    ```\n"
    with pytest.raises(ValueError, match="found 2"):
        smoke.quickstart(block + "\n" + block, "python")


def test_document_requires_expected_paragraph(tmp_path):
    path = tmp_path / "report.docx"
    def write(text):
        with zipfile.ZipFile(path, "w") as package:
            package.writestr("word/document.xml",
                f'<w:document xmlns:w="{smoke.W[1:-1]}"><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:document>')
    write("Hello, world!")
    smoke.verify_document(path)
    write("Wrong")
    with pytest.raises(ValueError, match="expected quickstart paragraph"):
        smoke.verify_document(path)


def test_rejects_editable_wheel_before_install(tmp_path):
    wheel = tmp_path / "rocx.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("rocx.pth", "/checkout/python")
    with pytest.raises(ValueError, match="not a release artifact"):
        smoke.wheel_smoke(wheel, tmp_path, "0.3.0", "")
