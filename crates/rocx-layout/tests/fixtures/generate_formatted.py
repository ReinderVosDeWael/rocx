"""Regenerate the direct paragraph formatting fixtures using python-docx."""

from pathlib import Path

from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml.ns import qn
from docx.shared import Pt, Twips

out = Path(__file__).parent
doc = Document()
section = doc.sections[0]
section.page_width = Pt(55)
section.page_height = Pt(180)
section.left_margin = section.right_margin = Pt(10)
section.top_margin = section.bottom_margin = Pt(10)

center = doc.add_paragraph("hello a")
center.alignment = WD_ALIGN_PARAGRAPH.CENTER
center.paragraph_format.space_before = Twips(21)
center.paragraph_format.space_after = Pt(7)

right = doc.add_paragraph("two")
right.alignment = WD_ALIGN_PARAGRAPH.RIGHT
right.paragraph_format.space_before = Pt(11)
right.paragraph_format.space_after = Pt(2)

left = doc.add_paragraph("one")
left.alignment = WD_ALIGN_PARAGRAPH.LEFT
left.paragraph_format.space_before = Twips(21)
doc.save(out / "physical-formatted.docx")

# Mix a supported measurement with an unsupported competing unit.
center._p.pPr.spacing.set(qn("w:afterLines"), "100")
doc.save(out / "physical-unsupported-spacing.docx")
