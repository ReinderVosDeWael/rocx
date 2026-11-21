"""Deferred note references and structured base-schema comments, without Office."""
import asyncio
import sys

from rocx import declarative as d


async def main() -> None:
    """Save footnote/endnote stories and an authored structured comment."""
    footnote=d.Note(kind=lambda:"footnote",id=lambda:0,children=lambda:[d.Paragraph(text="Footnote body λ")])
    comment=d.Comment([d.Paragraph(children=[d.TextRun("Structured review λ",bold=True)])],
                      author=lambda:"Ada",date="2026-11-18T00:00:00Z",initials="AL")
    document=d.Document([d.Section(children=[
        d.Paragraph(children=[d.TextRun("Text with a footnote"),d.NoteReference(footnote.deferred("kind"),footnote.deferred("id")),
                              d.TextRun(" and endnote"),d.NoteReference.endnote(0)]),
        d.Paragraph(children=[d.RangeStart.structured_comment("review",comment),d.TextRun("Reviewed text"),d.RangeEnd("review")])])],
        notes=[footnote,d.Note.endnote(0,[d.Paragraph(text="Endnote body Ω")])])
    await document.save(sys.argv[1] if len(sys.argv)>1 else "notes.docx")


if __name__=="__main__":
    asyncio.run(main())
