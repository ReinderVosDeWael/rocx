"""Make independent inputs for the installed JavaScript acceptance tests."""

import argparse
from pathlib import Path

from docx import Document
from docx.oxml.ns import qn


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    output = parser.parse_args().output_dir
    output.mkdir(parents=True, exist_ok=True)

    source = Document()
    source.add_paragraph("Independent template without explicit page margins")
    # python-docx's seed lacks this required Transitional attribute.
    source.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    section = source.sections[0]._sectPr
    section.remove(section.find(qn("w:pgMar")))
    source.save(output / "no-margins.docx")


if __name__ == "__main__":
    main()
