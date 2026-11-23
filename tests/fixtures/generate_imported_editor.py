"""Generate the independent DOCX source for tests/imported_editor.rs."""

from pathlib import Path

from docx import Document
from docx.enum.section import WD_SECTION_START
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.shared import Pt


def add_link(paragraph, url, *labels):
    # python-docx 1.2 can read hyperlinks but has no public create method.
    link = OxmlElement("w:hyperlink")
    link.set(qn("r:id"), paragraph.part.relate_to(
        url, RELATIONSHIP_TYPE.HYPERLINK, is_external=True
    ))
    for label in labels:
        # Let python-docx construct the run and preserve significant whitespace.
        link.append(paragraph.add_run(label)._r)
    paragraph._p.append(link)


def main() -> None:
    document = Document()
    # python-docx's bundled template omits this required Transitional attribute.
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    patch = document.add_paragraph()
    patch.add_run("Patch").bold = True
    start = OxmlElement("w:bookmarkStart")
    start.set(qn("w:id"), "0")
    start.set(qn("w:name"), "BodyLink")
    patch._p.append(start)
    patch.add_run()
    add_link(patch, "https://example.invalid/body", " ", "me")
    end = OxmlElement("w:bookmarkEnd")
    end.set(qn("w:id"), "0")
    patch._p.append(end)
    patch.paragraph_format.space_after = Pt(12)
    document.add_paragraph("Insert table after me")

    header = document.sections[0].header.paragraphs[0]
    header.text = "Shared header "
    add_link(header, "https://example.invalid/header", "link")
    second = document.add_section(WD_SECTION_START.NEW_PAGE)
    assert second.header.is_linked_to_previous
    document.add_paragraph("Second section")

    document.save(Path(__file__).with_name("imported-editor.docx"))


if __name__ == "__main__":
    main()
