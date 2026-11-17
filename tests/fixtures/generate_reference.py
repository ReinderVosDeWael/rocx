"""Regenerate XML fixtures from an existing checkout of the pinned CMI source.

Usage: uv run --locked --group test python tests/fixtures/generate_reference.py /path/to/cmi-docx
Uses the repository's uv environment; fixture generation itself needs no network.
"""

import asyncio
import pathlib
import subprocess
import sys

REVISION = "3449b942034129ad74a28aeb253361035806e385"
reference = pathlib.Path(sys.argv[1]).resolve()
actual = subprocess.check_output(["git", "-C", str(reference), "rev-parse", "HEAD"], text=True).strip()
if actual != REVISION:
    raise SystemExit(f"Expected {REVISION}, found {actual}")
sys.path.insert(0, str(reference / "src"))

from cmi_docx.declarative import Document, Paragraph, Section, TextRun


async def async_text():
    return "Deferred text"


def skipped_text():
    raise AssertionError("Skipped text must not be evaluated")


async def main():
    cases = {
        "runs": Document(sections=[Section(children=[
            Paragraph(children=[
                TextRun(text="First "), TextRun(text=""),
                TextRun(text="hidden", condition=lambda: False),
                TextRun(text="café & <world>\t\n"),
            ]),
            Paragraph(children=lambda: [
                TextRun(text=lambda: "Lazy "), TextRun(text=async_run_text()),
            ]),
            Paragraph(children=[]), Paragraph(text=""),
        ])]),
        "empty": Document(sections=[]),
        "empty_section": Document(sections=[Section(children=[])]),
        "plain": Document(sections=[Section(children=[
            Paragraph(text="Hello, world!"), Paragraph(text=""),
            Paragraph(text="café Ελληνικά 日本語 🌍"),
        ])]),
        "whitespace": Document(sections=[Section(children=[
            Paragraph(text="  Leading  and trailing  "),
            Paragraph(text="<tag> & \"quotes\" 'apostrophes' ]]> "),
            Paragraph(text="\tfirst\t\tlast\t"),
            Paragraph(text="a\nb\rc\r\nd\n"),
        ])]),
        "sections": Document(sections=[
            Section(children=[Paragraph(text="one")]),
            Section(children=[]),
            Section(children=[Paragraph(text="two"), Paragraph(text="three")]),
        ]),
        "conditional": Document(sections=[
            Section(condition=lambda: False, children=lambda: skipped_text()),
            Section(children=[
                Paragraph(text="visible"),
                Paragraph(text=skipped_text, condition=lambda: False),
                Paragraph(text="also visible"),
            ]),
        ]),
        "deferred": Document(sections=[Section(children=lambda: [
            Paragraph(text=async_text()),
            Paragraph(text=lambda: "Lazy text"),
        ])]),
    }
    for name, declaration in cases.items():
        document = await declaration.to_docx()
        (pathlib.Path(__file__).parent / f"{name}.xml").write_bytes(document.element.xml.encode("utf-8"))


async def async_run_text():
    return "async"


asyncio.run(main())
