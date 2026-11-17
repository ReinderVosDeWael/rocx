"""Compatibility of constructors and dataclass fields with the actual pinned CMI package."""
import dataclasses
import hashlib
import inspect
import json
from pathlib import Path

import pytest
from cmi_docx import declarative as reference
from rv_docx import declarative as bindings


@pytest.mark.parametrize("name", reference.__all__)
def test_all_reference_exports_are_available(name):
    assert hasattr(bindings, name)


@pytest.mark.parametrize("name", [n for n in reference.__all__ if inspect.isclass(getattr(reference, n))])
def test_constructor_parameter_names_kinds_and_defaults_match(name):
    expected = inspect.signature(getattr(reference, name)).parameters
    actual = inspect.signature(getattr(bindings, name)).parameters
    assert list(actual) == list(expected)
    for field, parameter in expected.items():
        assert actual[field].kind == parameter.kind, (name, field)
        if parameter.default is inspect.Parameter.empty:
            assert actual[field].default is inspect.Parameter.empty
        elif callable(parameter.default):
            assert callable(actual[field].default)
            assert actual[field].default() == parameter.default()
        else:
            assert actual[field].default == parameter.default, (name, field)


@pytest.mark.parametrize("name", [n for n in reference.__all__ if dataclasses.is_dataclass(getattr(reference, n))])
def test_dataclass_field_names_order_and_keyword_only_contract_match(name):
    expected = dataclasses.fields(getattr(reference, name))
    actual = dataclasses.fields(getattr(bindings, name))
    assert [(f.name, f.kw_only, f.init) for f in actual] == [(f.name, f.kw_only, f.init) for f in expected]


def test_upstream_test_expectations_are_unchanged():
    directory = Path(__file__).parent / "upstream"
    origin = json.loads((directory / "ORIGIN.json").read_text())
    assert origin["revision"] == "3449b942034129ad74a28aeb253361035806e385"
    for name, digest in origin["sha256"].items():
        original = (directory / name).read_text().replace("rv_docx", "cmi_docx")
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
