"""Author a rich external link; no target is fetched during rendering."""
import asyncio
import sys

from rv_docx import declarative as d


async def main() -> None:
    """Resolve a target reference and save formatted hyperlink display children."""
    source = d.Hyperlink(target=lambda: "https://example.invalid/λ?q=a%20b&lang=日本#résumé", children=[])
    link = d.Hyperlink(source.deferred("target"),
                       lambda: [d.TextRun("Read ", bold=True), d.TextRun("more λ", underline=True)],
                       tooltip=lambda: "External resource", history=True)
    document = d.Document([d.Section(children=[d.Paragraph(children=[link])])])
    await document.save(sys.argv[1] if len(sys.argv) > 1 else "hyperlinks.docx")


if __name__ == "__main__":
    asyncio.run(main())
