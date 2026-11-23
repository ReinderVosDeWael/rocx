"""Fill an independent template with a small synthetic report."""
import asyncio
import sys
from pathlib import Path

import rocx
from rocx import declarative as d

TEMPLATE = Path("docs/website/branded_report_template.docx")
DATA = Path("examples/branded_report_data.tsv")


async def main() -> None:
    output = Path(sys.argv[1] if len(sys.argv) > 1 else "examples/branded_report_output.docx")
    if output.resolve() == TEMPLATE.resolve():
        raise ValueError("output must differ from template")
    rows = []
    for index, line in enumerate(DATA.read_text(encoding="utf-8").splitlines()):
        cells = [
            d.TableCell(
                [d.Paragraph(children=[d.TextRun(text, bold=index == 0)])],
                background_color=(220, 234, 245) if index == 0 else None,
            )
            for text in line.split("\t")
        ]
        rows.append(d.TableRow(cells))
    document = d.Document([d.Section([d.Table(rows)])])
    template = d.DocumentTemplate(
        TEMPLATE, replacements={"{{title}}": "Quarterly service report"},
        paragraph_index=1, scope="body",
    )
    await (await document.resolve()).save(
        output, template, options=rocx.RenderOptions(timestamp="2026-01-01T00:00:00Z")
    )


if __name__ == "__main__":
    asyncio.run(main())
