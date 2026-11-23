"""Independent executable probe of Python synchronous boundary patterns."""

import asyncio
from io import BytesIO

from docx import Document as WordDocument
from rocx import declarative as d


async def produce_text():
    await asyncio.sleep(0)
    return "Produced asynchronously"


def make_document():
    # A fresh coroutine is needed on each resolution.
    return d.Document(
        [d.Section([d.Paragraph(text=produce_text())])],
        timestamp="2026-11-21T00:00:00Z",
    )


async def render():
    return await make_document().to_bytes()


def render_from_script():
    """Illustrative application-local adapter, not a rocx API."""
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(render())
    raise RuntimeError("A loop is running; use 'await render()' instead")


async def notebook_analog(expected):
    try:
        render_from_script()
    except RuntimeError as exc:
        assert "use 'await render()'" in str(exc)
    else:
        raise AssertionError("synchronous adapter accepted a running loop")
    actual = await render()
    assert actual == expected, "async and synchronous DOCX bytes differ"
    return actual


if __name__ == "__main__":
    script_bytes = render_from_script()
    notebook_bytes = asyncio.run(notebook_analog(script_bytes))
    assert WordDocument(BytesIO(notebook_bytes)).paragraphs[0].text == (
        "Produced asynchronously"
    )
    print("PASS: script, running-loop guard, async producer, exact bytes, DOCX readback")
