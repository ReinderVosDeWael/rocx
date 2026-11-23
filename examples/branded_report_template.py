"""Create the independent, unsigned python-docx template used by the report."""
from docx import Document
from docx.shared import Pt, RGBColor


document = Document()
header = document.sections[0].header.paragraphs[0]
run = header.add_run("NORTHSTAR  |  SERVICE INSIGHTS")
run.bold = True
run.font.size = Pt(11)
run.font.color.rgb = RGBColor(30, 72, 112)
document.add_heading("{{title}}", level=0)
document.add_paragraph("Synthetic quarterly results")
document.save("docs/website/branded_report_template.docx")
