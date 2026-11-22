"""Write the same introductory document as examples/plain.rs."""
# docs:start quickstart
import asyncio
import sys

from rocx import declarative as d


async def main() -> None:
    """Resolve and save a reproducible introductory document."""
    document = d.Document([d.Section([
        d.Paragraph(text="Hello, world!"),
        d.Paragraph(text="A second paragraph, with Unicode: café 🌍"),
    ])], timestamp="2026-11-21T00:00:00Z")
    await document.save(sys.argv[1] if len(sys.argv) > 1 else "examples/plain.docx")


if __name__ == "__main__":
    asyncio.run(main())
# docs:end quickstart
