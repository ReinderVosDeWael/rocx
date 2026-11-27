"""Regenerate the independent physical-layout DOCX inputs using python-docx."""

from pathlib import Path

from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Twips

out = Path(__file__).parent
doc = Document()
section = doc.sections[0]
section.page_width = Twips(1001)
section.page_height = Twips(1440)
section.left_margin = Twips(201)
section.right_margin = Twips(199)
section.top_margin = Twips(200)
section.bottom_margin = Twips(200)

paragraph = doc.add_paragraph()
paragraph.alignment = WD_ALIGN_PARAGRAPH.LEFT
paragraph.add_run("hel")
paragraph.add_run("lo")
doc.add_paragraph()
doc.add_paragraph("one\ntwo")
doc.add_paragraph("a😀a")
doc.save(out / "physical-plain.docx")

doc.add_table(1, 1).cell(0, 0).text = "unsupported"
doc.add_paragraph("must not move up")
doc.save(out / "physical-table.docx")
