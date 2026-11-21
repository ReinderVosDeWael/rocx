"""Run with `uv run --locked examples/property_dependencies.py report.docx`."""
import asyncio
import sys
from rocx.declarative import Document, Paragraph, Section


async def main(destination):
    source = Paragraph(text=asyncio.sleep(0, result="42"))

    async def summarize():
        result = await source.deferred("text")
        return f"The result is {result}"

    summary = Paragraph(text=summarize())
    document = Document([Section(children=[summary, source])])
    await document.save(destination)


if __name__ == "__main__":
    asyncio.run(main(sys.argv[1] if len(sys.argv) > 1 else "report.docx"))
