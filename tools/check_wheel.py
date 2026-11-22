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
    """Check installed native output without optional packages."""
    import rocx
    from rocx import _native
    from rocx import declarative as d

    environment = Path(sys.prefix).resolve()
    assert Path(rocx.__file__).resolve().is_relative_to(environment)
    assert Path(_native.__file__).resolve().is_relative_to(environment)
    assert any(str(_native.__file__).endswith(suffix)
               for suffix in importlib.machinery.EXTENSION_SUFFIXES)
    assert _native.BACKEND == "rust"
    assert importlib.util.find_spec("docx") is None
    requirements = importlib.metadata.requires("rocx") or []
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
    overridden = await snapshot.to_bytes(options=rocx.RenderOptions(timestamp="2026-11-18T00:00:00Z"))
    with zipfile.ZipFile(io.BytesIO(overridden)) as package:
        assert b"2026-11-18T00:00:00Z" in package.read("docProps/core.xml")
    try:
        await snapshot.to_bytes(options=rocx.RenderOptions(profile="strict"))
    except ValueError as error:
        assert error.rule == "unsupported_profile"
        assert error.location == "options.profile"
    else:
        raise AssertionError("Strict output was unexpectedly accepted")
    assert calls == ["text"]
    try:
        await snapshot.to_docx()
    except ImportError as error:
        assert "rocx[interop]" in str(error)
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
    heights = await d.Document([d.Section(children=[
        d.Paragraph(text="exact", line_height=d.LineHeight.exact(14.25)),
        d.Paragraph(text="minimum", line_height=d.LineHeight.at_least(18)),
        d.Paragraph(text="multiple", line_height=d.LineHeight.multiple(1.5)),
    ])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(heights)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        assert [(node.get(f"{W}line"), node.get(f"{W}lineRule")) for node in root.iter(f"{W}spacing")] == [
            ("285", "exact"), ("360", "atLeast"), ("360", "auto")]
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
    tabs = await d.Document([d.Section(children=[d.Paragraph(text="tabs", tab_stops=[
        d.TabStop(108, "right", "dot"), d.TabStop(72, "clear"),
    ])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(tabs)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        stops = root.findall(f".//{W}tabs/{W}tab")
        assert [(node.get(f"{W}pos"), node.get(f"{W}val")) for node in stops] == [("1440", "clear"), ("2160", "right")]
        assert stops[1].get(f"{W}leader") == "dot"
    rows = await d.Document([d.Section(children=[d.Table(rows=[d.TableRow([
        d.TableCell([d.Paragraph(text="row")])], height=d.RowHeight.exact(14.25), repeat_header=True, cant_split=False)])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(rows)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        properties = root.find(f".//{W}trPr")
        assert properties.find(f"{W}trHeight").get(f"{W}val") == "285"
        assert properties.find(f"{W}trHeight").get(f"{W}hRule") == "exact"
        assert properties.find(f"{W}tblHeader").get(f"{W}val") == "1"
        assert properties.find(f"{W}cantSplit").get(f"{W}val") == "0"
    scripted = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("mixed", language=d.RunLanguage(bidi="ar-SA"),
        font_east_asia="East Asian", font_complex_script="Arabic", bold_complex_script=True, italic_complex_script=False,
        size_complex_script=14.5, rtl=True, complex_script=False)]), d.Table([d.TableRow([d.TableCell([d.Paragraph(text="cell")],
        margins=d.CellMargins(start=0, end=9), text_direction="tbRl")])])])],
        styles=[d.ParagraphStyleDefinition(name="Child", base_style="Later"), d.ParagraphStyleDefinition(name="Later")]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(scripted)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{W}lang").get(f"{W}bidi") == "ar-SA"
        assert root.find(f".//{W}rFonts").get(f"{W}cs") == "Arabic"
        assert root.find(f".//{W}szCs").get(f"{W}val") == "29"
        assert root.find(f".//{W}tcMar/{W}start").get(f"{W}w") == "0"
        assert root.find(f".//{W}textDirection").get(f"{W}val") == "tbRl"
        styles = ET.fromstring(package.read("word/styles.xml"))
        child = next(n for n in styles if n.get(f"{W}styleId") == "Child")
        assert child.find(f"{W}basedOn").get(f"{W}val") == "Later"
    characters = await d.Document([d.Section(children=[d.Paragraph(children=[d.TextRun("styled", style="Character", bold=False)], bidi=True)], properties=d.SectionProperties(bidi=True, rtl_gutter=False))],
        styles=[d.CharacterStyleDefinition(name="Character", italic=True)]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(characters)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{W}rStyle").get(f"{W}val") == "Character"
        assert root.find(f".//{W}bidi").get(f"{W}val") == "1"
        assert root.find(f".//{W}sectPr/{W}bidi").get(f"{W}val") == "1"
        assert root.find(f".//{W}sectPr/{W}rtlGutter").get(f"{W}val") == "0"
        styles = ET.fromstring(package.read("word/styles.xml"))
        character = next(n for n in styles if n.get(f"{W}styleId") == "Character")
        assert character.get(f"{W}type") == "character"
        assert character.find(f"{W}rPr/{W}i").get(f"{W}val") == "1"
    restarted = await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel()],
        start_overrides=[d.NumberingStartOverride(0, 7)])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(restarted)) as package:
        root = ET.fromstring(package.read("word/numbering.xml"))
        assert root.find(f".//{W}startOverride").get(f"{W}val") == "7"
    conditional = await d.Document([], styles=[d.TableStyleDefinition(name="Conditional",
        first_row=d.TableSectionFormat(line_height=d.LineHeight.exact(14.5), tab_stops=[d.TabStop(36)], bidi=False,
            language=d.RunLanguage(bidi="ar-SA"), rtl=False, bold_complex_script=True,
            italic_complex_script=False, complex_script=True, size_complex_script=13.5,
            font_east_asia="East", font_complex_script="Complex"))]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(conditional)) as package:
        root = ET.fromstring(package.read("word/styles.xml"))
        style = next(n for n in root if n.get(f"{W}styleId") == "Conditional")
        properties = style.find(f"{W}tblStylePr/{W}pPr")
        assert properties.find(f"{W}spacing").get(f"{W}line") == "290"
        assert properties.find(f"{W}tabs/{W}tab").get(f"{W}pos") == "720"
        assert properties.find(f"{W}bidi").get(f"{W}val") == "0"
        run = style.find(f"{W}tblStylePr/{W}rPr")
        assert run.find(f"{W}lang").get(f"{W}bidi") == "ar-SA"
        assert run.find(f"{W}rFonts").get(f"{W}cs") == "Complex"
        assert run.find(f"{W}rFonts").get(f"{W}eastAsia") == "East"
        for element, expected in [("rtl", "0"), ("bCs", "1"), ("iCs", "0"), ("cs", "1"), ("szCs", "27")]:
            assert run.find(f"{W}{element}").get(f"{W}val") == expected
    defaults = await d.Document([], defaults=d.DocumentDefaults(run=d.RunDefaults(size=13.5, bold=False),
        paragraph=d.ParagraphDefaults(line_height=d.LineHeight.exact(14), left_indent=12))).to_bytes()
    with zipfile.ZipFile(io.BytesIO(defaults)) as package:
        root = ET.fromstring(package.read("word/styles.xml"))
        run = root.find(f"{W}docDefaults/{W}rPrDefault/{W}rPr")
        assert run.find(f"{W}sz").get(f"{W}val") == "27"
        assert run.find(f"{W}b").get(f"{W}val") == "0"
        paragraph = root.find(f"{W}docDefaults/{W}pPrDefault/{W}pPr")
        assert paragraph.find(f"{W}spacing").get(f"{W}line") == "280"
        assert paragraph.find(f"{W}ind").get(f"{W}left") == "240"
    linked = await d.Document([], styles=[d.ParagraphStyleDefinition(name="Pair Paragraph", linked_style="Pair Character"),
        d.CharacterStyleDefinition(name="Pair Character", linked_style="Pair Paragraph")]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(linked)) as package:
        root = ET.fromstring(package.read("word/styles.xml"))
        for identity, target in [("PairParagraph", "PairCharacter"), ("PairCharacter", "PairParagraph")]:
            style = next(n for n in root if n.get(f"{W}styleId") == identity)
            assert style.find(f"{W}link").get(f"{W}val") == target
    replacement = await d.Document([], numbering=[d.NumberingDefinition(43, [d.NumberingLevel(is_legal=True, suffix="tab", restart=0, alignment="start")],
        start_overrides=[d.NumberingStartOverride(0, 7)],
        level_overrides=[d.NumberingLevel(0, 4, "upperRoman", "(%1)", is_legal=False, suffix="nothing", restart=1, alignment="end")])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(replacement)) as package:
        root = ET.fromstring(package.read("word/numbering.xml"))
        num = next(n for n in root if n.get(f"{W}numId") == "43")
        override = num.find(f"{W}lvlOverride")
        assert [n.tag for n in override] == [f"{W}startOverride", f"{W}lvl"]
        assert override.find(f"{W}lvl/{W}numFmt").get(f"{W}val") == "upperRoman"
        assert override.find(f"{W}startOverride").get(f"{W}val") == "7"
        assert override.find(f"{W}lvl/{W}isLgl").get(f"{W}val") == "0"
        assert root.find(f"{W}abstractNum/{W}lvl/{W}isLgl").get(f"{W}val") == "1"
        assert override.find(f"{W}lvl/{W}suff").get(f"{W}val") == "nothing"
        assert root.find(f"{W}abstractNum/{W}lvl/{W}suff").get(f"{W}val") == "tab"
        assert override.find(f"{W}lvl/{W}lvlRestart").get(f"{W}val") == "1"
        assert root.find(f"{W}abstractNum/{W}lvl/{W}lvlRestart").get(f"{W}val") == "0"
        abstract_id = num.find(f"{W}abstractNumId").get(f"{W}val")
        abstract = next(n for n in root if n.get(f"{W}abstractNumId") == abstract_id)
        assert abstract.find(f"{W}lvl/{W}lvlJc").get(f"{W}val") == "start"
        assert override.find(f"{W}lvl/{W}lvlJc").get(f"{W}val") == "end"
    looked = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="look")])])],
        alignment="end", look=d.TableLook(first_row=True, last_row=True, first_column=True, last_column=True,
            horizontal_banding=True, vertical_banding=True))])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(looked)) as package:
        root = ET.fromstring(package.read("word/document.xml"))
        look = root.find(f".//{W}tblLook")
        assert look.get(f"{W}val") == "01E0"
        assert look.get(f"{W}noHBand") == "0"
        assert look.get(f"{W}noVBand") == "0"
        assert look.get(f"{W}lastRow") == "1"
        assert root.find(f".//{W}tblPr/{W}jc").get(f"{W}val") == "end"
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
    with tempfile.TemporaryDirectory(prefix="rocx-wheel-") as temporary:
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
