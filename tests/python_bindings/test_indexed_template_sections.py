"""Independent python-docx template and section readback for indexed insertion."""
import io

import docx
from docx.shared import Inches
import pytest

from rocx import declarative as d


@pytest.mark.parametrize("index,first,expected", [
    (0, "A", [["A", ""], ["B", "Before", "Original"]]),
    (1, "A", [["Before", "A", ""], ["B", "Original"]]),
    (99, "A", [["Before", "Original", "A", ""], ["B"]]),
    (1, None, [["Before", ""], ["B", "Original"]]),
])
async def test_indexed_sections_preserve_order_and_margins(tmp_path, index, first, expected):
    source = docx.Document()
    source.add_paragraph("Before")
    source.add_paragraph("Original")
    path = tmp_path / "source.docx"
    source.save(path)
    sections = [
        d.Section(
            [d.Paragraph(text=first)] if first else [],
            properties=d.SectionProperties(page_margins={"left": Inches(1)}),
        ),
        d.Section(
            [d.Paragraph(text="B")],
            properties=d.SectionProperties(page_margins={"left": Inches(2)}),
        ),
    ]
    data = await d.Document(sections).to_bytes(d.DocumentTemplate(path, paragraph_index=index))
    output = docx.Document(io.BytesIO(data))
    assert [[p.text for p in section.iter_inner_content()] for section in output.sections] == expected
    assert [section.left_margin for section in output.sections] == [Inches(1), Inches(2)]
