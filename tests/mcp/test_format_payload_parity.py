"""Installed stdio parity for formatted paragraph replacements and rich insertions."""

from copy import deepcopy
import subprocess

import docx
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Pt
from jsonschema.validators import validator_for
import pytest

from test_stdio import (
    anyio_backend, assert_tool_failure, call, digest, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio

# The left column is MCP, the right column is WordprocessingML.
ALIGNMENTS = [
    ("justify-medium", "mediumKashida"),
    ("center", "center"),
    ("justify", "both"),
    ("distribute", "distribute"),
    ("left", "left"),
    ("right", "right"),
    ("justify-high", "highKashida"),
    ("justify-low", "lowKashida"),
    ("thai-justify", "thaiDistribute"),
]
SWATCHES = [
    "dark1", "light1", "dark2", "light2", "accent1", "accent3", "accent4", "accent5",
    "accent6", "hyperlink", "followedHyperlink", "background1", "text1", "background2", "text2",
]
UNDERLINES = ["none", "single", "dotted", "dash", "wave", "words"]


def text_run(text, **formatting):
    return dict(kind="text", text=text, **({"format": formatting} if formatting else {}))


def declarations():
    runs = [
        text_run(
            "Rich run", style="PayloadCharacter", bold=False, italic=True, underline="double",
            font="Latin Font", size=11.5, color=dict(theme="accent2", tint=59),
            superscript=True, subscript=False, strike=True, all_caps=False, small_caps=True,
            snap_to_grid=False, language=dict(latin="en-US", east_asia="ja-JP", bidi="ar-SA"),
            bold_complex_script=True, italic_complex_script=False, size_complex_script=13.5,
            rtl=False, complex_script=True, font_east_asia="East Asian Font",
            font_complex_script="Complex Font",
        ),
        text_run(" Subscript", color=[1, 128, 255], superscript=False, subscript=True),
        text_run(" Automatic", color="auto"),
    ]
    for index, theme in enumerate(SWATCHES):
        color = dict(theme=theme, **({"shade": 150} if index == 0 else {}))
        underline = {"underline": UNDERLINES[index]} if index < len(UNDERLINES) else {}
        runs.append(text_run(f" Swatch {index}", color=color, **underline))

    tabs = [
        dict(position=70, alignment="end"),
        dict(position=60, alignment="start", leader="middleDot"),
        dict(position=50, alignment="clear"),
        dict(position=40, alignment="bar", leader="heavy"),
        dict(position=30, alignment="decimal", leader="underscore"),
        dict(position=20, alignment="right", leader="hyphen"),
        dict(position=10, alignment="center", leader="dot"),
        dict(position=-9.5, alignment="left", leader="none"),
    ]
    formats = [
        dict(
            bidi=True, style="PayloadParagraph", spacing_before=4.25, spacing_after=6.5,
            line_height=dict(kind="at-least", value=12.25), left_indent=-3.5,
            right_indent=8.25, first_line_indent=-2.5, keep_together=True, keep_with_next=False,
            page_break_before=True, widow_control=False, suppress_line_numbers=True,
            contextual_spacing=False, mirror_indents=True, snap_to_grid=False, tab_stops=tabs,
        ),
        dict(heading=2, style="PayloadParagraph", line_spacing=1.25, first_line_indent=3.5),
        dict(line_height=dict(kind="exact", value=13.5)),
        dict(line_height=dict(kind="multiple", value=1.5)),
        {}, {}, {}, {}, {},
    ]
    return [
        dict(
            inlines=runs if index == 0 else [text_run(f"Replacement {index}")],
            format=dict(formatting, alignment=wire),
        )
        for index, ((wire, _), formatting) in enumerate(zip(ALIGNMENTS, formats, strict=True))
    ]


def write_source(path):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.styles.add_style("PayloadParagraph", WD_STYLE_TYPE.PARAGRAPH)
    fixture.styles.add_style("PayloadCharacter", WD_STYLE_TYPE.CHARACTER)
    for index in range(len(ALIGNMENTS)):
        fixture.add_paragraph(f"Original {index}")
    fixture.add_paragraph("Untouched body").runs[0].bold = True
    fixture.sections[0].header.paragraphs[0].text = "Untouched header"
    fixture.save(path)


def replacement(anchor, paragraph):
    return dict(kind="block", anchor=anchor, edit=dict(kind="replace-paragraph", paragraph=paragraph))


def attributes(element, path):
    """Require exactly one element and return its complete Word attribute set."""
    (node,) = element.xpath(path)
    namespace = qn("w:val").removesuffix("val")
    assert all(name.startswith(namespace) for name in node.attrib)
    return {name.removeprefix(namespace): value for name, value in node.attrib.items()}


def check_docx(output):
    document = docx.Document(output)
    rich, heading, exact, multiple, *_ = document.paragraphs
    assert len(document.paragraphs) == len(ALIGNMENTS) + 1
    for index, (paragraph, (_, xml)) in enumerate(zip(document.paragraphs[:-1], ALIGNMENTS, strict=True)):
        assert attributes(paragraph._p, "./w:pPr/w:jc") == {"val": xml}
        if index:
            assert paragraph.text == f"Replacement {index}"
    assert document.paragraphs[-1].text == "Untouched body"
    assert document.paragraphs[-1].runs[0].bold is True
    assert document.sections[0].header.paragraphs[0].text == "Untouched header"
    assert rich.style.style_id == "PayloadParagraph"
    assert heading.style.style_id == "Heading2"
    assert heading.paragraph_format.first_line_indent == Pt(3.5)

    flags = dict(
        bidi="1", keepLines="1", keepNext="0", pageBreakBefore="1", widowControl="0",
        suppressLineNumbers="1", contextualSpacing="0", mirrorIndents="1", snapToGrid="0",
    )
    for flag, value in flags.items():
        assert attributes(rich._p, f"./w:pPr/w:{flag}") == {"val": value}
    assert attributes(rich._p, "./w:pPr/w:ind") == {"left": "-70", "right": "165", "hanging": "50"}
    for paragraph, expected in [
        (rich, dict(before="85", after="130", line="245", lineRule="atLeast")),
        (heading, dict(line="300", lineRule="auto")),
        (exact, dict(line="270", lineRule="exact")),
        (multiple, dict(line="360", lineRule="auto")),
    ]:
        assert attributes(paragraph._p, "./w:pPr/w:spacing") == expected
    assert len(rich._p.xpath("./w:pPr/w:tabs/w:tab")) == 8
    for index, (position, alignment, leader) in enumerate([
        ("-190", "left", "none"), ("200", "center", "dot"),
        ("400", "right", "hyphen"), ("600", "decimal", "underscore"),
        ("800", "bar", "heavy"), ("1000", "clear", None),
        ("1200", "start", "middleDot"), ("1400", "end", None),
    ], 1):
        expected = dict(pos=position, val=alignment, **({"leader": leader} if leader else {}))
        assert attributes(rich._p, f"./w:pPr/w:tabs/w:tab[{index}]") == expected

    rich_run, subscript, automatic, *swatches = rich.runs
    assert [rich_run.text, subscript.text, automatic.text] == ["Rich run", " Subscript", " Automatic"]
    assert rich_run.style.style_id == "PayloadCharacter"
    assert rich_run.font.size == Pt(11.5) and rich_run.font.name == "Latin Font"
    properties = {
        **{key: {"val": value} for key, value in dict(
            b="0", i="1", u="double", sz="23", vertAlign="superscript", strike="1",
            caps="0", smallCaps="1", snapToGrid="0", bCs="1", iCs="0", szCs="27", rtl="0", cs="1",
        ).items()},
        "rFonts": dict(ascii="Latin Font", hAnsi="Latin Font", eastAsia="East Asian Font",
                       cs="Complex Font"),
        "lang": dict(val="en-US", eastAsia="ja-JP", bidi="ar-SA"),
        "color": dict(val="auto", themeColor="accent2", themeTint="3B"),
    }
    for name, expected in properties.items():
        assert attributes(rich_run._r, f"./w:rPr/w:{name}") == expected
    assert attributes(subscript._r, "./w:rPr/w:vertAlign") == {"val": "subscript"}
    assert attributes(subscript._r, "./w:rPr/w:color") == {"val": "0180FF"}
    assert attributes(automatic._r, "./w:rPr/w:color") == {"val": "auto"}
    for index, (run, theme) in enumerate(zip(swatches, SWATCHES, strict=True)):
        assert run.text == f" Swatch {index}"
        expected = dict(val="auto", themeColor=theme, **({"themeShade": "96"} if index == 0 else {}))
        assert attributes(run._r, "./w:rPr/w:color") == expected
        if index < len(UNDERLINES):
            assert attributes(run._r, "./w:rPr/w:u") == {"val": UNDERLINES[index]}
        else:
            assert not run._r.xpath("./w:rPr/w:u")


async def check_guards(client, validator, request):
    # Each invalid edit shares a batch with valid edits. Reject it twice without
    # issuing a preview, and leave the same original anchors usable afterward.
    cases = [
        ("paragraph", {"line_spacing": 2}, "mutually exclusive"),
        ("paragraph", {"heading": 10}, "heading must be in 0..=9"),
        ("paragraph", {"tab_stops": [
            dict(position=1, alignment="left"), dict(position=1.01, alignment="right"),
        ]}, "tab stops collide after rounding to twips"),
        ("run", {"size": -1}, "size cannot be negative"),
        ("run", {"size_complex_script": -1}, "size_complex_script cannot be negative"),
        ("run", {"language": {"eastAsia": "ja-JP"}}, None),
        ("paragraph", {"indent": 1}, None),
    ]
    for scope, changes, detail in cases:
        invalid = deepcopy(request)
        paragraph = invalid["edits"][0]["edit"]["paragraph"]
        target = paragraph if scope == "paragraph" else paragraph["inlines"][0]
        target["format"].update(changes)
        assert validator.is_valid(invalid) is (detail is not None), changes
        if detail is None:
            await failure(client, "preview_edits", "invalid_input", **invalid)
            continue
        results = [await client.call_tool("preview_edits", invalid) for _ in range(2)]
        for result in results:
            assert result.isError, changes
            data = result.structuredContent
            assert data["category"] == "rejected_batch" and "preview" not in data
            assert detail in data["message"]
        assert results[0].structuredContent == results[1].structuredContent


async def test_declaration_format_fields_native_bytes_and_guards(launch, tmp_path):
    source, output, native = (tmp_path / name for name in ("source.docx", "output.docx", "native.docx"))
    write_source(source)
    original = source.read_bytes()
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_format_payload_oracle")
    subprocess.run([oracle, source, native], check=True, cwd=tmp_path, timeout=20)

    async with launch() as client:
        (tool,) = [item for item in (await client.list_tools()).tools if item.name == "preview_edits"]
        validator = validator_for(tool.inputSchema)(tool.inputSchema)
        document = (await call(client, "open_document", path=source.name))["document"]
        before = (await call(client, "inspect_document", document=document))["body"]
        edits = [replacement(row["anchor"], payload) for row, payload in
                 zip(before[:-1], declarations(), strict=True)]
        request = dict(document=document, edits=edits)
        validator.validate(request)
        await check_guards(client, validator, request)
        assert (await call(client, "inspect_document", document=document))["body"] == before

        preview = await call(client, "preview_edits", **request)
        repeated = await call(client, "preview_edits", **request)
        saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == preview["output_snapshot"] == repeated["output_snapshot"]
        assert saved["output_snapshot"] == digest(output.read_bytes())

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {part for part in before if before[part] != after[part]} == {"word/document.xml"}
    check_docx(output)
    validate(source, output)


def write_insertion_source(path):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.add_paragraph("Before")
    for kind, text in [("bookmarkStart", "Range starts"), ("bookmarkEnd", "Range ends")]:
        paragraph = fixture.add_paragraph(text)
        marker = OxmlElement(f"w:{kind}")
        marker.set(qn("w:id"), "7")
        if kind == "bookmarkStart":
            marker.set(qn("w:name"), "Spanned")
            paragraph._p.insert(0, marker)
        else:
            paragraph._p.append(marker)
    fixture.add_paragraph("Later original")
    fixture.save(path)


def insertion_payload():
    tab = dict(kind="tab", format=dict(bold=True))
    breaks = [dict(kind="break", type=kind, format=dict(italic=True))
              for kind in ("line", "page", "column", "text-wrapping")]
    return dict(
        format=dict(alignment="center"),
        inlines=[
            text_run("Inserted 😀", bold=True),
            tab, *breaks,
            dict(kind="fraction", numerator="x + 1", denominator="2"),
            dict(kind="hyperlink", target=dict(kind="url", url="https://example.org/insert"),
                 tooltip="Insertion link", history=False,
                 children=[text_run("link", underline="single"), tab, breaks[0]]),
        ],
    )


def check_inserted_docx(output):
    before, rich, start, end, empty, later = docx.Document(output).paragraphs
    assert [p.text for p in (before, start, end, empty, later)] == [
        "Before", "Range starts", "Range ends", "", "Later original",
    ]
    assert len(empty._p) == 0
    assert attributes(start._p, "./w:bookmarkStart") == {"id": "7", "name": "Spanned"}
    assert attributes(end._p, "./w:bookmarkEnd") == {"id": "7"}
    assert attributes(later._p, "./w:pPr/w:jc") == {"val": "right"}
    assert attributes(rich._p, "./w:pPr/w:jc") == {"val": "center"}
    text, tab, *breaks = rich.runs
    assert text.text == "Inserted 😀" and text.bold is True
    assert tab.text == "\t" and tab.bold is True
    assert [attributes(run._r, "./w:br") for run in breaks] == [
        {"type": kind} for kind in ("textWrapping", "page", "column", "textWrapping")
    ]
    assert all(run.italic is True for run in breaks)
    assert rich._p.xpath("./m:oMath/m:f/*/m:r/m:t/text()") == ["x + 1", "2"]
    (link,) = rich.hyperlinks
    assert link.url == "https://example.org/insert"
    assert link._element.get(qn("w:tooltip")) == "Insertion link"
    assert link._element.get(qn("w:history")) == "0"
    link_text, link_tab, link_break = link.runs
    assert link_text.text == "link" and link_text.underline is True
    assert link_tab.text == "\t" and link_tab.bold is True
    assert attributes(link_break._r, "./w:br") == {"type": "textWrapping"}
    assert link_break.italic is True


async def test_rich_paragraph_insert_native_bytes_original_coordinates_and_retry(launch, tmp_path):
    source, output, native = (tmp_path / name for name in ("source.docx", "output.docx", "native.docx"))
    write_insertion_source(source)
    original = source.read_bytes()
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_format_payload_oracle")
    subprocess.run([oracle, source, native, "insert"], check=True, cwd=tmp_path, timeout=20)

    async with launch() as client:
        (tool,) = [item for item in (await client.list_tools()).tools if item.name == "preview_edits"]
        validator = validator_for(tool.inputSchema)(tool.inputSchema)
        document = (await call(client, "open_document", path=source.name))["document"]
        body = (await call(client, "inspect_document", document=document))["body"]
        alignment = dict(kind="alignment", anchor=body[3]["anchor"], alignment="right")
        empty = dict(kind="insert-rich-paragraph", anchor=body[2]["anchor"], paragraph=dict(inlines=[]))
        rich = dict(kind="insert-rich-paragraph", anchor=body[0]["anchor"], paragraph=insertion_payload())
        internal_link = deepcopy(rich)
        internal_link["paragraph"]["inlines"][-1]["target"] = dict(kind="bookmark", name="Spanned")
        for edits, message in [
            ([alignment, dict(empty, anchor=body[1]["anchor"]), rich],
             "paragraph insertion point is inside an active or ambiguous range"),
            ([alignment, empty, internal_link], "unsupported inline content in an inserted paragraph"),
        ]:
            request = dict(document=document, edits=edits)
            validator.validate(request)
            result = await client.call_tool("preview_edits", request)
            assert_tool_failure(result, "rejected_batch")
            assert message in result.structuredContent["message"]

        assert (await call(client, "inspect_document", document=document))["body"] == body
        request = dict(document=document, edits=[alignment, empty, rich])
        validator.validate(request)
        preview = await call(client, "preview_edits", **request)
        assert preview["changes"]["paragraph_insertions"] == [0, 2]
        assert preview["changes"]["changes"] == [{"target": 3, "before": None, "after": "Right"}]
        assert sum(map(len, preview["changes"].values())) == 3
        saved = await call(client, "save_copy", document=document, preview=preview["preview"], output=output.name)
        assert saved["output_snapshot"] == preview["output_snapshot"] == digest(output.read_bytes())

    assert source.read_bytes() == original
    assert output.read_bytes() == native.read_bytes()
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {part for part in before if before[part] != after[part]} == {
        "word/document.xml", "word/_rels/document.xml.rels",
    }
    check_inserted_docx(output)
    validate(source, output)
