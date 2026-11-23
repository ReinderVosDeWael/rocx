"""Native Python equivalent of the JavaScript report, for exact byte parity."""
import asyncio
from pathlib import Path
import sys

from rocx import declarative as d

IMAGE = Path(__file__).resolve().parents[1] / "tests/fixtures/images/sample.png"


def blocks() -> list:
    return [
        d.Paragraph(children=[
            d.TextRun("JavaScript authoring: ", bold=True, italic=False),
            d.TextRun("Café 東京", italic=True),
        ]),
        d.Table([
            d.TableRow([d.TableCell([d.Paragraph(text=text)]) for text in row])
            for row in [("Item", "Count"), ("Pages", "2")]
        ]),
        d.Paragraph(children=[
            d.ImageRun(IMAGE.read_bytes(), alt_text={"description": "JavaScript authoring sample image"}),
        ]),
    ]


async def generate(output: Path) -> None:
    document = d.Document([d.Section(blocks())], timestamp="2026-11-21T00:00:00Z")
    output.write_bytes(await document.to_bytes())


if __name__ == "__main__":
    asyncio.run(generate(Path(sys.argv[1])))
