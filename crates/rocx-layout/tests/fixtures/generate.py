"""Regenerate the independent two-paragraph input using python-docx."""

from pathlib import Path

from docx import Document

doc = Document()
for _ in range(2):
    paragraph = doc.add_paragraph()
    paragraph.add_run("hel")
    paragraph.add_run("lo")
doc.save(Path(__file__).with_name("split-runs.docx"))
