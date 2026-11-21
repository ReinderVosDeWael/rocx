"""Paired range markers allow overlap without nesting containers."""
import asyncio
import sys

from rocx import declarative as d


async def main() -> None:
    """Write an internal link, bookmark and overlapping cross-paragraph comment."""
    first = d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("chapter"),[d.TextRun("Jump to chapter. ")]),
        d.RangeStart.bookmark("chapter"),d.TextRun("Chapter begins. "),
        d.RangeStart.comment("review","Review this range","Ada"),d.TextRun("First paragraph.")])
    second = d.Paragraph(children=[d.TextRun("Second paragraph. "),d.RangeEnd.bookmark("chapter"),
        d.TextRun("Review ends here."),d.RangeEnd("review")])
    document = d.Document([d.Section(children=[first,second])])
    await document.save(sys.argv[1] if len(sys.argv)>1 else "ranges.docx")


if __name__ == "__main__":
    asyncio.run(main())
