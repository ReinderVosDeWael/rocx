"""Regenerate the direct indent fixtures with python-docx."""

from pathlib import Path

from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml.ns import qn
from docx.shared import Pt, Twips

out = Path(__file__).parent
doc = Document()
section = doc.sections[0]
section.page_width = section.page_height = Pt(70)
section.left_margin = section.right_margin = Pt(10)
section.top_margin = section.bottom_margin = Pt(10)

first = doc.add_paragraph("hello a").paragraph_format
first.left_indent = Pt(4)
first.right_indent = Pt(5)
first.first_line_indent = Twips(141)

hanging = doc.add_paragraph("hello hello hello").paragraph_format
hanging.left_indent = Pt(8)
hanging.right_indent = Pt(4)
hanging.first_line_indent = Twips(-41)

right = doc.add_paragraph("one")
right.alignment = WD_ALIGN_PARAGRAPH.RIGHT
right.paragraph_format.right_indent = Twips(41)
doc.save(out / "physical-indents.docx")

# The public APIs use one first-line offset and cannot create this conflict.
hanging._element.pPr.ind.set(qn("w:firstLine"), "1")
doc.save(out / "physical-ambiguous-indent.docx")
