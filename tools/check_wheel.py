"""Build and verify a wheel in an isolated uv environment, without test extras."""

import argparse
import asyncio
import importlib.metadata
import importlib.machinery
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Final
import xml.etree.ElementTree as ET
import zipfile

W: Final = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
TEXT: Final = "Wheel — café Ελληνικά 日本語 🦀"


async def verify_install() -> None:
    """Check installed native output without either reference implementation."""
    import rv_docx
    from rv_docx import _native
    from rv_docx import declarative as d

    environment = Path(sys.prefix).resolve()
    assert Path(rv_docx.__file__).resolve().is_relative_to(environment)
    assert Path(_native.__file__).resolve().is_relative_to(environment)
    assert any(str(_native.__file__).endswith(suffix)
               for suffix in importlib.machinery.EXTENSION_SUFFIXES)
    assert _native.BACKEND == "rust"
    assert importlib.util.find_spec("docx") is None
    assert importlib.util.find_spec("cmi_docx") is None
    requirements = importlib.metadata.requires("rv-docx-rs") or []
    assert len(requirements) == 1, requirements
    requirement = requirements[0].lower().replace(" ", "").replace("'", '"')
    assert requirement == 'python-docx==1.2.0;extra=="interop"', requirements

    calls = []

    def text() -> str:
        calls.append("text")
        return TEXT

    declaration = d.Document([d.Section(children=[d.Paragraph(children=[
        d.TextRun(text=text, bold=True, italic=True),
    ])])])
    snapshot = await declaration.resolve()
    first = await snapshot.to_bytes()
    assert first == await snapshot.to_bytes()
    output = Path("installed.docx")
    await snapshot.save(output)
    assert output.read_bytes() == first
    assert calls == ["text"]
    assert callable(declaration.sections[0].children[0].children[0].text)
    with zipfile.ZipFile(io.BytesIO(first)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        assert "".join(root.itertext()) == TEXT
        run = root.find(f".//{W}r")
        assert run is not None
        assert run.find(f"{W}rPr/{W}b") is not None
        assert run.find(f"{W}rPr/{W}i") is not None
    overridden = await snapshot.to_bytes(options=rv_docx.RenderOptions(timestamp="2026-11-18T00:00:00Z"))
    with zipfile.ZipFile(io.BytesIO(overridden)) as package:
        assert b"2026-11-18T00:00:00Z" in package.read("docProps/core.xml")
    try:
        await snapshot.to_bytes(options=rv_docx.RenderOptions(profile="strict"))
    except ValueError as error:
        assert error.rule == "unsupported_profile"
        assert error.location == "options.profile"
    else:
        raise AssertionError("Strict output was unexpectedly accepted")
    assert calls == ["text"]
    try:
        await snapshot.to_docx()
    except ImportError as error:
        assert "rv-docx-rs[interop]" in str(error)
    else:
        raise AssertionError("to_docx() unexpectedly worked without interop")
    target = "https://example.invalid/λ?q=a%20b&lang=日本#résumé"
    linked = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Hyperlink(lambda: target, [d.TextRun("Link λ", bold=True)])])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(linked)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        link = root.find(f".//{W}hyperlink")
        assert link is not None
        assert "".join(link.itertext()) == "Link λ"
        identifier = link.get("{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id")
        relations = ET.fromstring(package.read("word/_rels/document.xml.rels"))
        relation = next(node for node in relations if node.get("Id") == identifier)
        assert relation.get("Target") == target
        assert relation.get("TargetMode") == "External"
    ranged = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Hyperlink(d.BookmarkTarget("target"), [d.TextRun("jump")]),
        d.RangeStart.bookmark("target"),d.TextRun("body"),d.RangeEnd.bookmark("target")])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(ranged)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        start = root.find(f".//{W}bookmarkStart")
        end = root.find(f".//{W}bookmarkEnd")
        assert start is not None and end is not None
        assert start.get(f"{W}id") == end.get(f"{W}id")
        link = root.find(f".//{W}hyperlink")
        assert link is not None and link.get(f"{W}anchor") == "target"
        assert b"/hyperlink" not in package.read("word/_rels/document.xml.rels")
    content=d.Comment([d.Paragraph(children=[d.TextRun("Rich comment",bold=True)])],author="Ada",date="2026-11-18T00:00:00Z")
    noted=await d.Document([d.Section(children=[d.Paragraph(children=[d.NoteReference.footnote(0),
        d.RangeStart.structured_comment("review",content),d.TextRun("anchor"),d.RangeEnd("review")])])],
        notes=[d.Note.footnote(0,[d.Paragraph(text="Footnote λ")])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(noted)) as package:
        root=ET.fromstring(package.read("word/footnotes.xml"))
        note=next(node for node in root if node.get(f"{W}id")=="0")
        assert "".join(note.itertext())=="Footnote λ"
        assert note.find(f".//{W}footnoteRef") is not None
        comments=ET.fromstring(package.read("word/comments.xml"))
        assert comments[0].get(f"{W}author")=="Ada"
        assert "".join(comments[0].itertext())=="Rich comment"
    fields = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Field(d.FieldInstruction.page(), [d.TextRun("7")]),
        d.Field(d.FieldInstruction.num_pages(), [d.TextRun("12")], form="complex", dirty=True),
    ])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(fields)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{W}fldSimple").get(f"{W}instr") == " PAGE "
        assert root.find(f".//{W}instrText").text == " NUMPAGES "
        assert [n.get(f"{W}fldCharType") for n in root.iter(f"{W}fldChar")] == ["begin", "separate", "end"]
        assert "".join(n.text or "" for n in root.iter(f"{W}t")) == "712"
    print("Installed wheel: native imports, metadata, Unicode, formatting and snapshots verified")


def main() -> None:
    """Build a fresh wheel and run this probe outside the source tree."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", action="store_true", help=argparse.SUPPRESS)
    options = parser.parse_args()
    if options.probe:
        asyncio.run(verify_install())
        return
    root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="rv-docx-wheel-") as temporary:
        directory = Path(temporary)
        subprocess.run(["uv", "build", "--wheel", "--out-dir", str(directory / "dist")],
                       cwd=root, check=True)
        wheel, = (directory / "dist").glob("*.whl")
        subprocess.run(["uv", "venv", "--python", sys.executable, str(directory / "env")], check=True)
        python = directory / "env" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
        subprocess.run(["uv", "pip", "install", "--python", str(python), "--no-deps", str(wheel)], check=True)
        subprocess.run([str(python), "-I", str(Path(__file__).resolve()), "--probe"],
                       cwd=directory, check=True)


if __name__ == "__main__":
    main()
