"""Native Python equivalent of the JavaScript report, for exact byte parity."""
import asyncio
from pathlib import Path
import sys

from docx.enum.text import WD_ALIGN_PARAGRAPH

from rocx import declarative as d

IMAGE = Path(__file__).resolve().parents[1] / "tests/fixtures/images/sample.png"


def blocks() -> list:
    def text_cell(text: str) -> d.TableCell:
        return d.TableCell([d.Paragraph(text=text)])

    return [
        d.Paragraph(children=[
            d.TextRun("JavaScript authoring: ", bold=True, italic=False),
            d.TextRun("Café 東京", italic=True),
        ], alignment=WD_ALIGN_PARAGRAPH.CENTER),
        d.Table([
            d.TableRow([text_cell("Item"), text_cell("Details")]),
            d.TableRow([text_cell("Pages"), d.TableCell([
                d.Paragraph(children=[
                    d.TextRun("2", bold=True),
                    d.Tab(),
                    d.Hyperlink("https://example.org/report", [d.TextRun("Report details", italic=True)]),
                    d.Break(),
                    d.TextRun("Verified"),
                ]),
                d.Table([d.TableRow([text_cell("Nested detail")])]),
            ])]),
        ]),
        d.Paragraph(children=[
            d.ImageRun(
                IMAGE.read_bytes(),
                transformation={"width": 72, "height": 36},
                alt_text={"description": "JavaScript authoring sample image"},
            ),
        ]),
    ]


async def generate(output: Path) -> None:
    document = d.Document([d.Section(blocks())], timestamp="2026-11-21T00:00:00Z")
    output.write_bytes(await document.to_bytes())


if __name__ == "__main__":
    asyncio.run(generate(Path(sys.argv[1])))
