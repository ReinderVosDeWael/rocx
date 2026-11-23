"""Create a compact synthetic python-docx viewer fixture, not a Word-authored file."""

import argparse
from pathlib import Path

from docx import Document
from docx.shared import Inches


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="output DOCX path")
    args = parser.parse_args()

    document = Document()
    document.add_heading("Viewer compatibility: compact synthetic fixture", level=1)
    document.add_paragraph("Unicode and readability: café 🌍")
    table = document.add_table(rows=2, cols=2)
    table.style = "Table Grid"
    for cell, text in zip(
        (cell for row in table.rows for cell in row.cells),
        ("Table header A", "Table header B", "Table cell 1", "Table cell 2"),
    ):
        cell.text = text
    document.add_paragraph("List item one", style="List Bullet")
    document.add_paragraph("List item two", style="List Bullet")
    document.add_paragraph("Image: synthetic red rectangle")
    image = Path(__file__).resolve().parents[1] / "tests/fixtures/images/sample.png"
    document.add_picture(str(image), width=Inches(1))
    document.save(args.output)


if __name__ == "__main__":
    main()
