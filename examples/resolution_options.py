"""Run with uv run --no-sync python examples/resolution_options.py <output.docx>."""
import asyncio
import sys

import rocx
from rocx import declarative as d


async def main() -> None:
    """Share resolved children and render with an explicit timestamp override."""
    source = d.Header(children=lambda: [d.Paragraph(text="Shared resolved content λ")])
    section = d.Section(children=source.deferred("children"), headers={"default": source},
                        properties=d.SectionProperties(columns=lambda: {"count": 2}))
    snapshot = await d.Document([section], title=lambda: "Resolution and rendering").resolve()
    options = rocx.RenderOptions(timestamp="2026-11-18T00:00:00Z")
    await snapshot.save(sys.argv[1] if len(sys.argv) > 1 else "resolution-options.docx", options=options)


if __name__ == "__main__":
    asyncio.run(main())
