"""Current API invariants and provenance of historical comparison fixtures."""
import hashlib
import json
from pathlib import Path

import pytest
from cmi_docx import declarative as reference
from rocx import declarative as bindings


def test_upstream_test_expectations_are_unchanged():
    directory = Path(__file__).parent / "upstream"
    origin = json.loads((directory / "ORIGIN.json").read_text())
    assert origin["revision"] == "3449b942034129ad74a28aeb253361035806e385"
    for name, digest in origin["sha256"].items():
        original = (directory / name).read_text().replace("rocx", "cmi_docx")
        assert hashlib.sha256(original.encode()).hexdigest() == digest, name


@pytest.mark.parametrize("api", [reference, bindings])
def test_paragraph_rejects_missing_or_conflicting_content(api):
    with pytest.raises(ValueError):
        api.Paragraph()
    with pytest.raises(ValueError):
        api.Paragraph(text="text", children=[])


@pytest.mark.parametrize("name,side", [("TableBorder", "insideH"), ("CellBorder", "start")])
def test_border_hex_color_matches_reference(name, side):
    expected = getattr(reference, name)(side=side, color=(12, 34, 255))
    actual = getattr(bindings, name)(side=side, color=(12, 34, 255))
    assert actual.hex_color == expected.hex_color == "0C22FF"
