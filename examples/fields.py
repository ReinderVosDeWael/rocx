"""Author instructions and cached placeholders; consumers calculate fields."""
import asyncio
import sys

from rv_docx import declarative as d


async def main() -> None:
    """Save typed bookmark references, a TOC request and footer page fields."""
    reference = d.Field(instruction=lambda: d.FieldInstruction.reference("Introduction"),
                        children=[d.TextRun("Introduction (cached)")], dirty=True)
    document = d.Document([d.Section(children=[
        d.Paragraph(children=[d.RangeStart.bookmark("Introduction"), d.TextRun("Introduction"),
                              d.RangeEnd.bookmark("Introduction")]),
        d.Paragraph(children=[reference]),
        d.Paragraph(children=[d.Field(d.FieldInstruction.table_of_contents(),
                                     [d.TextRun("Contents pending update")], form="complex", dirty=True)]),
    ], footers={"default": d.Footer(children=[d.Paragraph(children=[
        d.TextRun("Page "), d.Field(d.FieldInstruction.page(), [d.TextRun("1")]),
        d.TextRun(" of "), d.Field(d.FieldInstruction.num_pages(), [d.TextRun("1")]),
    ])])})])
    await document.save(sys.argv[1] if len(sys.argv) > 1 else "fields.docx")


if __name__ == "__main__":
    asyncio.run(main())
