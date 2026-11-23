"""Generate the independent DOCX source for table-cell inline anchor tests."""

from pathlib import Path

from docx import Document
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

from generate_imported_editor import add_link


def main() -> None:
    document = Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.add_paragraph("Cell label")
    table = document.add_table(rows=2, cols=2)
    cell = table.cell(0, 0)
    paragraph = cell.paragraphs[0]
    label = paragraph.add_run("Cell label")
    label.add_tab()
    label.add_break()
    start = OxmlElement("w:bookmarkStart")
    start.set(qn("w:id"), "0")
    start.set(qn("w:name"), "CellLink")
    paragraph._p.append(start)
    add_link(paragraph, "https://example.invalid/cell", "cell ", "link")
    end = OxmlElement("w:bookmarkEnd")
    end.set(qn("w:id"), "0")
    paragraph._p.append(end)
    cell.add_table(rows=1, cols=1).cell(0, 0).text = "Opaque nested text"
    cell.paragraphs[-1].add_run("Cell label")
    cell.add_paragraph()
    table.cell(0, 1).text = "Cell label"
    table.cell(1, 0).text = "Cell label"
    document.add_table(rows=1, cols=1).cell(0, 0).text = "Cell label"

    document.save(Path(__file__).with_name("imported-table-cells.docx"))


if __name__ == "__main__":
    main()
