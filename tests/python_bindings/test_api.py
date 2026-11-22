"""Public API invariants for the Python declarations."""

import pytest

from rocx import declarative as d


def test_paragraph_requires_one_content_form():
    with pytest.raises(ValueError):
        d.Paragraph()
    with pytest.raises(ValueError):
        d.Paragraph(text="text", children=[])


@pytest.mark.parametrize(
    "kind,side", [(d.TableBorder, "insideH"), (d.CellBorder, "start")]
)
def test_border_colors_are_normalized_to_uppercase_hex(kind, side):
    border = kind(side=side, color=(12, 34, 255))
    assert border.hex_color == "0C22FF"
