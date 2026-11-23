"""Generate the independent DOCX source for tests/imported_editor.rs."""

from pathlib import Path

from docx import Document
from docx.enum.section import WD_SECTION_START
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.shared import Pt


def main() -> None:
    document = Document()
    patch = document.add_paragraph()
    patch.add_run("Patch").bold = True
    patch.add_run(" me")
    patch.paragraph_format.space_after = Pt(12)
    document.add_paragraph("Insert table after me")

    header = document.sections[0].header.paragraphs[0]
    header.text = "Shared header "
    # python-docx 1.2 can read hyperlinks but has no public create method.
    link = OxmlElement("w:hyperlink")
    link.set(qn("r:id"), header.part.relate_to(
        "https://example.invalid/header", RELATIONSHIP_TYPE.HYPERLINK, is_external=True
    ))
    run = OxmlElement("w:r")
    text = OxmlElement("w:t")
    text.text = "link"
    run.append(text)
    link.append(run)
    header._p.append(link)
    second = document.add_section(WD_SECTION_START.NEW_PAGE)
    assert second.header.is_linked_to_previous
    document.add_paragraph("Second section")

    document.save(Path(__file__).with_name("imported-editor.docx"))


if __name__ == "__main__":
    main()
