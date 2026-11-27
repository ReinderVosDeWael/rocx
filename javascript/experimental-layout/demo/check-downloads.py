"""Independently check actual browser downloads from the DOCX demo smoke."""

from pathlib import Path
import sys
from zipfile import ZipFile

from docx import Document


def payloads(path):
    with ZipFile(path) as package:
        return {entry: package.read(entry) for entry in package.namelist()}


def main(directory):
    fixture = (
        Path(__file__).resolve().parents[3]
        / "crates/rocx-layout/tests/fixtures/physical-plain.docx"
    )
    original = fixture.read_bytes()
    for name in ("original", "after-reset"):
        assert (directory / f"{name}.docx").read_bytes() == original, name
    assert (directory / "after-rejection.docx").read_bytes() == (
        directory / "delete.docx"
    ).read_bytes()

    expected_original = ["hello", "", "one\ntwo", "a😀a"]
    assert [p.text for p in Document(fixture).paragraphs] == expected_original
    other_payloads = payloads(fixture)
    original_document = other_payloads.pop("word/document.xml")
    for name, first in (("insert", "Ahello"), ("replace", "Xello"), ("delete", "ello")):
        path = directory / f"{name}.docx"
        assert [p.text for p in Document(path).paragraphs] == [first, *expected_original[1:]], name
        edited_payloads = payloads(path)
        assert edited_payloads.pop("word/document.xml") != original_document, name
        assert edited_payloads == other_payloads, name
    print("Firefox DOCX downloads: insert, replace, delete, rollback, and unchanged source/other parts")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
