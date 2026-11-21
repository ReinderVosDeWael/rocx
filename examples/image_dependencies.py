"""Run with uv run --no-sync python examples/image_dependencies.py <output.docx>."""

import asyncio
import pathlib
import sys

from rocx import declarative as d


async def main() -> None:
    """Resolve shared image properties and save without requiring interop."""
    source = d.ImageRun(
        data=pathlib.Path(__file__).resolve().parents[1] / "tests/fixtures/images/sample.png",
        transformation=lambda: {"width": 36},
        alt_text=lambda: {"title": "Shared image options λ"},
    )
    image = d.ImageRun(data=source.deferred("data"),
                       transformation=source.deferred("transformation"),
                       alt_text=source.deferred("alt_text"))
    snapshot = await d.Document([d.Section(children=[d.Paragraph(children=[image])])]).resolve()
    await snapshot.save(sys.argv[1] if len(sys.argv) > 1 else "image-dependencies.docx")


if __name__ == "__main__":
    asyncio.run(main())
