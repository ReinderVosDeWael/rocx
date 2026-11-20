"""Read Rust-generated files using python-docx, without a CMI source checkout.

Invoked by tests/python_docx.rs. unittest assertions remain active under python -O.
"""

import argparse
from pathlib import Path
import sys
import unittest
from lxml import etree

try:
    import docx
except ImportError as error:
    raise SystemExit(
        "Run `uv sync --locked --group test` from the repository root."
    ) from error

from docx import Document
from docx.enum.section import WD_ORIENT, WD_SECTION_START
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT, WD_ROW_HEIGHT_RULE
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_LINE_SPACING, WD_TAB_ALIGNMENT, WD_TAB_LEADER
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml.ns import qn
from docx.shared import Inches, Pt, Twips
from docx.text.run import Run


class Readback(unittest.TestCase):
    def __init__(self, case, document, source):
        super().__init__(f"test_{case}")
        self.path = document
        self.source = source

    def setUp(self):
        self.assertGreaterEqual(
            tuple(int(part) for part in docx.__version__.split(".")[:2]),
            (1, 2),
            "These tests require python-docx >=1.2.0; run `uv sync --locked --group test`.",
        )
        self.assertTrue(self.path.is_file(), f"Missing Rust output: {self.path}")
        self.document = Document(self.path)
        # Mutation tests require this marker before accepting an assertion failure:
        # a broken ZIP/XML file is not evidence that semantic checks work.
        print(f"Loaded DOCX: {self.path}", flush=True)

    def test_numbering_overrides(self):
        numbering = self.document.part.numbering_part.element
        num = next(n for n in numbering if n.get(qn("w:numId")) == "42")
        abstract_id = num.find(qn("w:abstractNumId")).get(qn("w:val"))
        abstract = next(n for n in numbering if n.get(qn("w:abstractNumId")) == abstract_id)
        self.assertEqual(abstract.find(qn("w:lvl")).find(qn("w:start")).get(qn("w:val")), "1")
        override = num.find(qn("w:lvlOverride"))
        self.assertEqual(override.get(qn("w:ilvl")), "0")
        self.assertEqual(override.find(qn("w:startOverride")).get(qn("w:val")), "7")
        self.assertEqual(self.document.paragraphs[0].text, "restart")

    def test_character_styles(self):
        style = self.document.styles["Child Character"]
        self.assertEqual(style.base_style.name, "Later Character")
        self.assertTrue(style.font.italic)
        self.assertEqual(style._element.find(qn("w:rPr")).find(qn("w:szCs")).get(qn("w:val")), "29")
        run = self.document.paragraphs[0].runs[0]
        self.assertEqual(run.style.name, "Child Character")
        self.assertFalse(run.bold)
        self.assertIsNone(run.italic)
        self.assertEqual(run.text, "styled")

    def test_style_references(self):
        self.assertEqual(self.document.styles["Child"].base_style.name, "Later Base")
        self.assertEqual(self.document.styles["Child"].next_paragraph_style.name, "Later Next")
        self.assertEqual(self.document.styles["Later Next"].next_paragraph_style.name, "Later Next")

    def test_table_look(self):
        table = self.document.tables[0]
        node = table._tbl.find(qn("w:tblPr")).find(qn("w:tblLook"))
        for name, expected in [("firstRow", "1"), ("lastRow", "1"), ("firstColumn", "0"), ("lastColumn", "1"), ("noHBand", "1"), ("noVBand", "0"), ("val", "0360")]:
            self.assertEqual(node.get(qn(f"w:{name}")), expected)
        self.assertEqual(table.cell(0, 0).text, "table-look request, not computed formatting")
        self.assertIsNone(table.cell(0, 0).paragraphs[0].runs[0].bold)

    def test_table_alignment(self):
        tables = self.document.tables
        self.assertEqual([None if (n := table._tbl.find(qn("w:tblPr")).find(qn("w:jc"))) is None else n.get(qn("w:val")) for table in tables], [None, "left", "center", "right", "start", "end"])
        for table in tables:
            self.assertEqual(table._tbl.find(qn("w:tblGrid")).find(qn("w:gridCol")).get(qn("w:w")), "1440")
            paragraph = table.cell(0, 0).paragraphs[0]
            self.assertEqual(paragraph.text, "table alignment, not text alignment")
            self.assertEqual(paragraph._p.find(qn("w:pPr")).find(qn("w:jc")).get(qn("w:val")), "right")

    def test_numbering_alignment(self):
        root = self.document.part.numbering_part.element
        num = next(n for n in root if n.get(qn("w:numId")) == "80")
        identity = num.find(qn("w:abstractNumId")).get(qn("w:val"))
        abstract = next(n for n in root if n.get(qn("w:abstractNumId")) == identity)
        replacements = [n.find(qn("w:lvl")) for n in num.findall(qn("w:lvlOverride"))]
        requests = [None, "left", "center", "right", "start", "end"]
        for nodes, expected in [(abstract.findall(qn("w:lvl")), requests), (replacements, list(reversed(requests)))]:
            self.assertEqual([None if (n := level.find(qn("w:lvlJc"))) is None else n.get(qn("w:val")) for level in nodes], expected)
        self.assertEqual(self.document.paragraphs[0].text, "label alignment, not paragraph alignment")
        self.assertEqual(self.document.paragraphs[0]._p.find(qn("w:pPr")).find(qn("w:jc")).get(qn("w:val")), "right")

    def test_numbering_restart(self):
        root = self.document.part.numbering_part.element
        num = next(n for n in root if n.get(qn("w:numId")) == "80")
        abstract_id = num.find(qn("w:abstractNumId")).get(qn("w:val"))
        abstract = next(n for n in root if n.get(qn("w:abstractNumId")) == abstract_id)
        replacements = [n.find(qn("w:lvl")) for n in num.findall(qn("w:lvlOverride"))]
        requests = [None, "0", "1", "2", "4294967295"]
        for nodes, expected in [(abstract.findall(qn("w:lvl")), requests), (replacements, list(reversed(requests)))]:
            self.assertEqual([None if (n := level.find(qn("w:lvlRestart"))) is None else n.get(qn("w:val")) for level in nodes], expected)
            self.assertEqual([level.find(qn("w:start")).get(qn("w:val")) for level in nodes], ["3"] * 5)
        self.assertEqual(num.findall(qn("w:lvlOverride"))[2].find(qn("w:startOverride")).get(qn("w:val")), "7")
        self.assertEqual(self.document.paragraphs[0].text, "restart request, not a calculated counter")

    def test_numbering_suffix(self):
        root = self.document.part.numbering_part.element
        num = next(n for n in root if n.get(qn("w:numId")) == "80")
        abstract_id = num.find(qn("w:abstractNumId")).get(qn("w:val"))
        abstract = next(n for n in root if n.get(qn("w:abstractNumId")) == abstract_id)
        replacements = [n.find(qn("w:lvl")) for n in num.findall(qn("w:lvlOverride"))]
        tokens = [None, "tab", "space", "nothing"]
        for nodes, expected in [(abstract.findall(qn("w:lvl")), tokens), (replacements, list(reversed(tokens)))]:
            self.assertEqual([None if (n := level.find(qn("w:suff"))) is None else n.get(qn("w:val")) for level in nodes], expected)
            self.assertEqual([level.find(qn("w:numFmt")).get(qn("w:val")) for level in nodes], ["upperRoman"] * 4)
        self.assertEqual(self.document.paragraphs[0].text, "suffix request, not inserted text")

    def test_numbering_replacements(self):
        root = self.document.part.numbering_part.element
        num = next(n for n in root if n.get(qn("w:numId")) == "42")
        overrides = num.findall(qn("w:lvlOverride"))
        self.assertEqual([n.get(qn("w:ilvl")) for n in overrides], ["0", "1", "2"])
        self.assertEqual([n.tag for n in overrides[0]], [qn("w:startOverride"), qn("w:lvl")])
        level = overrides[0].find(qn("w:lvl"))
        self.assertEqual(level.get(qn("w:ilvl")), "0")
        self.assertEqual(level.find(qn("w:start")).get(qn("w:val")), "4")
        self.assertEqual(level.find(qn("w:numFmt")).get(qn("w:val")), "upperRoman")
        self.assertEqual(level.find(qn("w:lvlText")).get(qn("w:val")), "Replacement-%1")
        self.assertEqual(overrides[0].find(qn("w:startOverride")).get(qn("w:val")), "7")
        self.assertEqual(overrides[1].find(qn("w:lvl")).find(qn("w:start")).get(qn("w:val")), "0")
        abstract_id = num.find(qn("w:abstractNumId")).get(qn("w:val"))
        abstract = next(n for n in root if n.get(qn("w:abstractNumId")) == abstract_id)
        self.assertEqual([n.find(qn("w:numFmt")).get(qn("w:val")) for n in abstract.findall(qn("w:lvl"))], ["decimal"] * 3)
        self.assertEqual(self.document.paragraphs[0].text, "replacement request, not a calculated label")

    def test_section_gutter(self):
        self.assertEqual([s._sectPr.find(qn("w:rtlGutter")).get(qn("w:val")) for s in self.document.sections], ["1", "1", "0"])
        self.assertEqual([s._sectPr.find(qn("w:bidi")).get(qn("w:val")) for s in self.document.sections], ["0", "0", "0"])
        self.assertEqual([p.text for p in self.document.paragraphs if p.text], ["on", "inherited", "off"])

    def test_linked_styles(self):
        for name, target in [("Pair Paragraph", "PairCharacter"), ("Pair Character", "PairParagraph")]:
            self.assertEqual(self.document.styles[name]._element.find(qn("w:link")).get(qn("w:val")), target)
        self.assertEqual(self.document.paragraphs[0].style.name, "Pair Paragraph")
        self.assertEqual(self.document.paragraphs[0].runs[0].style.name, "Pair Character")
        self.assertFalse(self.document.paragraphs[0].runs[0].italic)
        self.assertIsNone(self.document.styles["Pair Character"].font.bold)

    def test_document_defaults(self):
        defaults = self.document.styles.element.find(qn("w:docDefaults"))
        run = defaults.find(qn("w:rPrDefault")).find(qn("w:rPr"))
        self.assertEqual(run.find(qn("w:sz")).get(qn("w:val")), "27")
        self.assertEqual(run.find(qn("w:rFonts")).get(qn("w:ascii")), "Literal")
        self.assertIsNone(run.find(qn("w:rFonts")).get(qn("w:asciiTheme")))
        self.assertEqual(run.find(qn("w:b")).get(qn("w:val")), "1")
        self.assertEqual(run.find(qn("w:lang")).get(qn("w:bidi")), "ar-SA")
        paragraph = defaults.find(qn("w:pPrDefault")).find(qn("w:pPr"))
        self.assertEqual(paragraph.find(qn("w:spacing")).get(qn("w:before")), "0")
        self.assertEqual(paragraph.find(qn("w:spacing")).get(qn("w:line")), "280")
        self.assertEqual(paragraph.find(qn("w:ind")).get(qn("w:left")), "240")
        self.assertEqual(paragraph.find(qn("w:ind")).get(qn("w:hanging")), "120")
        self.assertFalse(self.document.paragraphs[0].runs[0].bold)
        self.assertIsNone(self.document.paragraphs[0].paragraph_format.left_indent)

    def test_conditional_script_formatting(self):
        style = self.document.styles["Scripts"]._element
        region = next(n for n in style.findall(qn("w:tblStylePr")) if n.get(qn("w:type")) == "firstRow")
        properties = region.find(qn("w:rPr"))
        for element, expected in [("bCs", "0"), ("iCs", "1"), ("szCs", "27"), ("rtl", "0"), ("cs", "1")]:
            self.assertEqual(properties.find(qn(f"w:{element}")).get(qn("w:val")), expected)
        self.assertEqual(dict(properties.find(qn("w:lang")).attrib), {qn("w:val"): "en-US", qn("w:eastAsia"): "ja-JP", qn("w:bidi"): "ar-SA"})
        self.assertEqual(dict(properties.find(qn("w:rFonts")).attrib), {qn("w:eastAsia"): "East", qn("w:cs"): "Complex"})

    def test_conditional_paragraph_formatting(self):
        style = self.document.styles["Conditional"]._element
        region = next(n for n in style.findall(qn("w:tblStylePr")) if n.get(qn("w:type")) == "firstRow")
        properties = region.find(qn("w:pPr"))
        self.assertEqual(properties.find(qn("w:spacing")).get(qn("w:line")), "290")
        self.assertEqual(properties.find(qn("w:spacing")).get(qn("w:lineRule")), "exact")
        tab = properties.find(qn("w:tabs")).find(qn("w:tab"))
        self.assertEqual(tab.get(qn("w:pos")), "1440")
        self.assertEqual(tab.get(qn("w:val")), "right")
        self.assertEqual(tab.get(qn("w:leader")), "dot")
        self.assertEqual(properties.find(qn("w:bidi")).get(qn("w:val")), "0")

    def test_section_direction(self):
        self.assertEqual([section._sectPr.find(qn("w:bidi")).get(qn("w:val")) for section in self.document.sections], ["1", "1", "0"])
        self.assertEqual(self.document.paragraphs[0].text, "first العربية last")
        self.assertFalse(self.document.paragraphs[0].runs[0].font.rtl)
        self.assertEqual(self.document.paragraphs[0]._p.find(qn("w:pPr")).find(qn("w:bidi")).get(qn("w:val")), "0")

    def test_script_formatting(self):
        self.assertEqual(self.document.paragraphs[0]._p.find(qn("w:pPr")).find(qn("w:bidi")).get(qn("w:val")), "1")
        self.assertEqual(self.document.styles["Script"]._element.find(qn("w:pPr")).find(qn("w:bidi")).get(qn("w:val")), "0")
        run = self.document.paragraphs[0].runs[0]
        self.assertEqual(run.text, "العربية 日本語")
        self.assertFalse(run.bold)
        self.assertTrue(run.font.cs_bold)
        self.assertTrue(run.italic)
        self.assertFalse(run.font.cs_italic)
        self.assertTrue(run.font.rtl)
        self.assertFalse(run.font.complex_script)
        self.assertEqual(run.font.size.pt, 11)
        self.assertEqual(run.font.name, "Latin")
        props = run._r.find(qn("w:rPr"))
        self.assertEqual(props.find(qn("w:rFonts")).get(qn("w:eastAsia")), "East Asian")
        self.assertEqual(props.find(qn("w:rFonts")).get(qn("w:cs")), "Arabic")
        self.assertEqual(props.find(qn("w:szCs")).get(qn("w:val")), "29")
        self.assertEqual(dict(props.find(qn("w:lang")).attrib), {qn("w:val"): "en-GB", qn("w:eastAsia"): "ja-JP", qn("w:bidi"): "ar-SA"})
        style = self.document.styles["Script"]
        self.assertFalse(style.font.cs_bold)
        self.assertEqual(style._element.find(qn("w:rPr")).find(qn("w:lang")).get(qn("w:bidi")), "he-IL")

    def test_cell_layout(self):
        cell = self.document.tables[0].cell(0, 0)
        self.assertEqual(cell.text, "cell")
        props = cell._tc.find(qn("w:tcPr"))
        margins = props.find(qn("w:tcMar"))
        self.assertEqual([(etree.QName(node).localname, node.get(qn("w:w")), node.get(qn("w:type"))) for node in margins],
                         [("top", "63", "dxa"), ("start", "0", "dxa"), ("bottom", "120", "dxa"), ("end", "180", "dxa")])
        self.assertEqual(props.find(qn("w:textDirection")).get(qn("w:val")), "tbRl")

    def test_row_properties(self):
        rows = self.document.tables[0].rows
        self.assertEqual(rows[0].height.pt, 14.25)
        self.assertEqual(rows[0].height_rule, WD_ROW_HEIGHT_RULE.EXACTLY)
        self.assertEqual(rows[1].height.pt, 18)
        self.assertEqual(rows[1].height_rule, WD_ROW_HEIGHT_RULE.AT_LEAST)
        self.assertIsNone(rows[2].height)
        for row, flag in zip(rows[:2], ["1", "0"]):
            for name in ["tblHeader", "cantSplit"]:
                self.assertEqual(row._tr.find(qn("w:trPr")).find(qn(f"w:{name}")).get(qn("w:val")), flag)
        self.assertIsNone(rows[2]._tr.find(qn("w:trPr")))

    def test_tab_stops(self):
        paragraphs = self.document.paragraphs
        self.assertEqual(paragraphs[0].text, "label\tvalue")
        stops = list(paragraphs[0].paragraph_format.tab_stops)
        self.assertEqual([stop.position.pt for stop in stops], [-12, 72, 108, 144, 180])
        self.assertEqual([stop.alignment for stop in stops], [WD_TAB_ALIGNMENT.LEFT, WD_TAB_ALIGNMENT.CENTER,
                         WD_TAB_ALIGNMENT.RIGHT, WD_TAB_ALIGNMENT.DECIMAL, WD_TAB_ALIGNMENT.BAR])
        self.assertEqual(stops[2].leader, WD_TAB_LEADER.DOTS)
        self.assertEqual(len(paragraphs[1].paragraph_format.tab_stops), 0)
        self.assertEqual(paragraphs[2].paragraph_format.tab_stops[0].alignment, WD_TAB_ALIGNMENT.CLEAR)
        self.assertEqual(self.document.styles["Tabbed"].paragraph_format.tab_stops[0].position.pt, 72)

    def test_line_heights(self):
        formats = [p.paragraph_format for p in self.document.paragraphs]
        self.assertEqual(formats[0].line_spacing, 1.5)
        self.assertEqual(formats[0].line_spacing_rule, WD_LINE_SPACING.ONE_POINT_FIVE)
        self.assertEqual(formats[1].line_spacing.pt, 14.25)
        self.assertEqual(formats[1].line_spacing_rule, WD_LINE_SPACING.EXACTLY)
        self.assertEqual(formats[2].line_spacing.pt, 18)
        self.assertEqual(formats[2].line_spacing_rule, WD_LINE_SPACING.AT_LEAST)
        self.assertIsNone(formats[3].line_spacing)
        style = self.document.styles["Measured"].paragraph_format
        self.assertEqual(style.line_spacing.pt, 20)
        self.assertEqual(style.line_spacing_rule, WD_LINE_SPACING.AT_LEAST)

    def test_fields(self):
        w = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
        paragraph = self.document.paragraphs[0]
        root = paragraph._p
        simple = list(root.iter(f"{w}fldSimple"))
        self.assertEqual([n.get(f"{w}instr") for n in simple],
                         [" PAGE ", " REF target ", ' TOC \\o "1-3" \\h \\z \\u '])
        self.assertEqual([[Run(run, paragraph).text for run in node.findall(f"{w}r")] for node in simple],
                         [["7"], ["Target"], ["Contents pending"]])
        self.assertTrue(Run(simple[0].find(f"{w}r"), paragraph).bold)
        self.assertEqual([n.text for n in root.iter(f"{w}instrText")],
                         ['  IF 1 = 1 "yes" "no"  ', " NUMPAGES "])
        marks = list(root.iter(f"{w}fldChar"))
        self.assertEqual([n.get(f"{w}fldCharType") for n in marks],
                         ["begin", "separate", "begin", "separate", "end", "end"])
        self.assertEqual(marks[0].get(f"{w}dirty"), "1")
        self.assertEqual(marks[0].get(f"{w}fldLock"), "0")
        self.assertEqual("".join(n.text or "" for n in root.iter(f"{w}t")),
                         "Target7cached 12TargetContents pending")

    def assert_comment_spans(self, expected):
        spans = {}
        for paragraph in self.document.paragraphs:
            active = {}
            for child in paragraph._p:
                if child.tag == qn("w:commentRangeStart"):
                    identifier = child.get(qn("w:id"))
                    self.assertNotIn(identifier, spans, "comment.duplicate_id")
                    active[identifier] = ""
                elif child.tag == qn("w:r"):
                    for identifier in active:
                        active[identifier] += Run(child, paragraph).text
                elif child.tag == qn("w:commentRangeEnd"):
                    identifier = child.get(qn("w:id"))
                    self.assertIn(identifier, active, "comment.unmatched_end")
                    spans[identifier] = active.pop(identifier)
            self.assertFalse(active, "comment.unmatched_start")
        self.assertEqual(spans, expected, "comment.anchored_text")

    def test_empty(self):
        self.assertEqual(len(self.document.paragraphs), 0)
        self.assertEqual(len(self.document.tables), 0)
        self.assertEqual(len(self.document.inline_shapes), 0)
        self.assertEqual(len(self.document.sections), 1)
        self.assertIsNotNone(self.document.core_properties.created)

    def test_notes(self):
        w="{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
        r="{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
        a="{http://schemas.openxmlformats.org/drawingml/2006/main}"
        target="https://example.invalid/λ?q=a%20b&x=2#frag"
        image=(Path(__file__).resolve().parents[1]/"fixtures/images/sample.png").read_bytes()
        self.assertEqual([p.text for p in self.document.paragraphs],["body λ","reviewed"])
        self.assertEqual([int(n.get(qn("w:id"))) for n in self.document._element.iter(qn("w:footnoteReference"))],[0,2])
        self.assertEqual([int(n.get(qn("w:id"))) for n in self.document._element.iter(qn("w:endnoteReference"))],[0])
        def resources(node,part,with_link):
            blip=node.find(f".//{a}blip")
            self.assertIsNotNone(blip)
            self.assertEqual(part.rels[blip.get(f"{r}embed")].target_part.blob,image)
            if with_link:
                link=node.find(f".//{w}hyperlink")
                self.assertIsNotNone(link)
                relation=part.rels[link.get(f"{r}id")]
                self.assertTrue(relation.is_external)
                self.assertEqual(relation.target_ref,target)
        for kind,typ,expected,text in [("footnote",RELATIONSHIP_TYPE.FOOTNOTES,{0,2},["Foot λ","note link","note table","nested table"]),
                                       ("endnote",RELATIONSHIP_TYPE.ENDNOTES,{0},["End Ω"])]:
            part=self.document.part.part_related_by(typ)
            root=etree.fromstring(part.blob)
            notes={int(n.get(f"{w}id")):n for n in root if n.tag==f"{w}{kind}"}
            self.assertEqual({i for i,n in notes.items() if n.get(f"{w}type","normal")=="normal"},expected)
            self.assertEqual({n.get(f"{w}type") for n in notes.values() if n.get(f"{w}type")},{"separator","continuationSeparator"})
            selected=self.document.settings._element.findall(f"{w}{kind}Pr/{w}{kind}")
            self.assertEqual(len(selected),2)
            self.assertTrue(all(notes[int(n.get(f"{w}id"))].get(f"{w}type") in {"separator","continuationSeparator"} for n in selected))
            node=notes[0]
            self.assertEqual([n.text for n in node.iter(f"{w}t")],text)
            self.assertIsNotNone(node.find(f".//{w}{kind}Ref"))
            if kind=="footnote":
                self.assertEqual(len(node.findall(f".//{w}tbl")),2)
            resources(node,part,kind=="footnote")
        comment=list(self.document.comments)[0]
        self.assertEqual(comment.author,"Ada")
        self.assertEqual(comment._element.get(f"{w}date"),"2026-11-18T01:02:03Z")
        self.assertEqual(comment._element.get(f"{w}initials"),"AL")
        self.assertEqual(comment.paragraphs[0].text,"Rich λcomment link")
        self.assertTrue(comment.paragraphs[0].runs[0].bold)
        self.assertEqual(comment.tables[0].cell(0,0).text,"comment table")
        self.assertEqual(comment.tables[0].cell(0,1).tables[0].cell(0,0).text,"nested table")
        resources(comment._element,comment.part,True)

    def test_ranges(self):
        self.assertEqual([p.text for p in self.document.paragraphs], ["jump alpha beta", "gamma delta epsilon", "legacy"])
        link = self.document.paragraphs[0].hyperlinks[0]
        self.assertEqual(link.fragment, "beta")
        self.assertIsNone(link._hyperlink.get(qn("r:id")))
        self.assertEqual([(c.text, c.author) for c in self.document.comments], [("range note", "Ada"), ("legacy note", "")])
        active, spans, names = {}, {}, {}
        for node in self.document._element.iter():
            if node.tag in (qn("w:bookmarkStart"), qn("w:commentRangeStart")):
                kind = "bookmark" if node.tag == qn("w:bookmarkStart") else "comment"
                key = kind, node.get(qn("w:id"))
                self.assertNotIn(key, active)
                active[key] = ""
                names[key] = node.get(qn("w:name"))
            elif node.tag == qn("w:t"):
                for key in active:
                    active[key] += node.text or ""
            elif node.tag in (qn("w:bookmarkEnd"), qn("w:commentRangeEnd")):
                kind = "bookmark" if node.tag == qn("w:bookmarkEnd") else "comment"
                key = kind, node.get(qn("w:id"))
                self.assertIn(key, active)
                spans[key] = active.pop(key)
        self.assertFalse(active)
        self.assertEqual({names[k]: v for k,v in spans.items() if k[0] == "bookmark"}, {"alpha":"alpha betagamma ","beta":"gamma delta epsilon"})
        self.assertEqual(sorted(v for k,v in spans.items() if k[0] == "comment"), ["betagamma delta ","legacy"])

    def test_hyperlinks(self):
        paragraphs = [self.document.paragraphs[0],
                      self.document.tables[0].cell(0, 0).paragraphs[0],
                      self.document.sections[0].header.paragraphs[-1],
                      self.document.sections[0].footer.paragraphs[-1]]
        for paragraph in paragraphs:
            self.assertEqual(paragraph.text, "Read more λ")
            self.assertEqual(len(paragraph.hyperlinks), 1)
            link = paragraph.hyperlinks[0]
            self.assertEqual(link.url, "https://example.invalid/λ?q=a%20b&lang=日本#résumé")
            self.assertEqual([run.text for run in link.runs], ["Read ", "more λ"])
            self.assertTrue(link.runs[0].bold)
            self.assertTrue(link.runs[1].italic)
            identifier = link._hyperlink.get(qn("r:id"))
            relationship = paragraph.part.rels[identifier]
            self.assertTrue(relationship.is_external)
            self.assertEqual(relationship.reltype, RELATIONSHIP_TYPE.HYPERLINK)

    def test_r1(self):
        self.assertEqual(self.document.core_properties.title, "R1 λ")
        self.assertEqual(self.document.core_properties.author, "Ada")
        self.assertEqual(self.document.sections[0].left_margin, 914400)
        self.assertEqual(self.document.sections[0].header.paragraphs[-1].text, "shared R1 λ")
        table = self.document.tables[0]
        self.assertEqual(table.cell(0, 0).text, "shared R1 λ")
        self.assertEqual(table.columns[0].width, Twips(1440))
        self.assertFalse(table.autofit)
        shading = table.cell(0, 0)._tc.tcPr.find(qn("w:shd"))
        self.assertIsNotNone(shading)
        self.assertEqual(shading.get(qn("w:fill")), "0C2238")

    def test_text(self):
        self.assertEqual(
            [paragraph.text for paragraph in self.document.paragraphs],
            [
                'Unicode: café Ελληνικά 日本語 🌍 <tag> & "quotes"',
                "  leading  middle  trailing  \tend\nnext\n\nCRLF",
                "",
                "",
                "First async lazy\t\n",
            ],
            "document.paragraphs.text",
        )
        self.assertEqual(len(self.document.sections), 1)
        self.assertEqual(len(self.document.paragraphs[2].runs), 0)
        self.assertEqual(len(self.document.paragraphs[3].runs), 0)
        paragraph = self.document.paragraphs[4]
        self.assertEqual(
            [run.text for run in paragraph.runs],
            ["First ", "", "async", " lazy", "\t", "\n"],
            "paragraph.runs.text",
        )
        self.assertIs(paragraph.runs[0].bold, True)

    def test_dependencies(self):
        self.assertEqual([p.text for p in self.document.paragraphs],
                         ["The result is 42 λ", "42 λ"], "dependencies.paragraphs.text")
        self.assertEqual([(c.text, c.author) for c in self.document.comments],
                         [("Source: 42 λ", "Ada")], "dependencies.comment.content")
        self.assert_comment_spans({"0": "The result is 42 λ"})

    def test_rich(self):
        document = self.document
        self.assertEqual(
            [p.text for p in document.paragraphs],
            ["Feature parity report", "Formatted supsub\tafter tab\nafter line",
             "", "", "Landscape section"],
            "document.paragraphs.text",
        )
        self.assertEqual(document.paragraphs[0].style.name, "Heading 1")
        paragraph = document.paragraphs[1]
        self.assertEqual(paragraph.style.name, "ReportBody")
        fmt = paragraph.paragraph_format
        self.assertEqual(fmt.alignment, WD_ALIGN_PARAGRAPH.JUSTIFY)
        for name, expected in {
            "space_before": Pt(12), "space_after": Pt(8), "line_spacing": 1.5,
            "left_indent": Pt(18), "right_indent": Pt(9),
            "first_line_indent": Pt(-6), "keep_together": True,
            "keep_with_next": False, "page_break_before": False,
            "widow_control": False,
        }.items():
            with self.subTest(paragraph_property=name):
                self.assertEqual(getattr(fmt, name), expected)
        run = next(r for r in paragraph.runs if r.text == "Formatted ")
        self.assertIs(run.bold, True, "run.bold")
        self.assertIs(run.italic, False)
        self.assertIs(run.underline, True)
        self.assertEqual(run.font.name, "Arial")
        self.assertEqual(run.font.size, Pt(14))
        self.assertEqual(str(run.font.color.rgb), "010203")
        self.assertIs(run.font.strike, True)
        self.assertIs(run.font.all_caps, True)
        self.assertIs(next(r for r in paragraph.runs if r.text == "sup").font.superscript, True)
        self.assertIs(next(r for r in paragraph.runs if r.text == "sub").font.subscript, True)

        style = document.styles["ReportBody"]
        self.assertEqual(style.base_style.name, "Normal")
        self.assertEqual(style.next_paragraph_style.name, "Normal")
        self.assertEqual(style.font.name, "Arial")
        self.assertEqual(style.font.size, Pt(11), "style.font.size")
        self.assertIs(style.font.bold, False)
        self.assertIs(style.font.italic, True)
        self.assertEqual(str(style.font.color.rgb), "0C2238")
        self.assertEqual(style.paragraph_format.left_indent, Inches(.25))
        self.assertEqual(style.paragraph_format.right_indent, Inches(.5))
        self.assertEqual(style.paragraph_format.first_line_indent, Inches(-.125))

        self.assertEqual(len(document.tables), 1)
        table = document.tables[0]
        self.assertEqual(table.style.name, "ReportTable")
        self.assertIs(table.autofit, False)
        self.assertEqual([column.width for column in table.columns],
                         [Twips(1000), Twips(2000), Twips(3000)], "table.column_widths")
        self.assertEqual(len(table.rows), 3)
        self.assertEqual([cell.text for cell in table.rows[2].cells], ["short", "row", ""])
        for row in table.rows[:2]:
            self.assertEqual([c.text for c in row.cells[:2]], ["merged", "merged"])
        self.assertEqual(table.cell(0, 0).vertical_alignment, WD_CELL_VERTICAL_ALIGNMENT.CENTER)
        properties = table.cell(0, 0)._tc.tcPr
        self.assertEqual(properties.find(qn("w:shd")).get(qn("w:fill")),
                         "FFEEDD", "table.cell.background")
        border = properties.find(qn("w:tcBorders")).find(qn("w:top"))
        self.assertEqual([border.get(qn(f"w:{name}")) for name in ["val", "sz", "color"]],
                         ["dashed", "4", "102030"], "table.cell.border")
        self.assertEqual(table.style.font.name, "Calibri")
        self.assertEqual(table.style.font.size, Pt(10))
        first_row = table.style.element.xpath('w:tblStylePr[@w:type="firstRow"]')[0]
        self.assertEqual(first_row.find(qn("w:tcPr")).find(qn("w:shd")).get(qn("w:fill")),
                         "002060", "table.style.first_row.background")
        self.assertEqual(first_row.find(qn("w:rPr")).find(qn("w:color")).get(qn("w:val")),
                         "FFFFFF", "table.style.first_row.color")
        nested_cell = table.cell(0, 2)
        self.assertEqual(nested_cell.tables[0].cell(0, 0).text, "nested")
        self.assertEqual([p.text for p in nested_cell.paragraphs], ["", "", "after nested"])
        self.assertEqual(table.cell(1, 2).text, "tail")

        self.assertEqual(len(document.sections), 2)
        self.assertEqual(document.sections[0].orientation, WD_ORIENT.PORTRAIT)
        self.assertEqual(document.sections[1].orientation, WD_ORIENT.LANDSCAPE)
        for section, dimensions in zip(document.sections, [(8.5, 11), (11, 8.5)]):
            self.assertEqual((section.page_width, section.page_height),
                             tuple(Inches(n) for n in dimensions), "section.dimensions")
            self.assertEqual(section.top_margin, Inches(1))
            self.assertEqual(section.bottom_margin, Inches(1))
            self.assertEqual(section.left_margin, Inches(1.25))
            self.assertEqual(section.right_margin, Inches(1.25))
            self.assertEqual(section.header.paragraphs[-1].text, "Default header", "section.header.text")
            self.assertEqual(section.first_page_header.paragraphs[-1].text, "First header")
            self.assertEqual(section.even_page_header.paragraphs[-1].text, "Even header")
            self.assertEqual(section.footer.paragraphs[-1].text, "Footer")
        self.assertEqual(len(document.inline_shapes), 1)
        self.assertEqual((document.inline_shapes[0].width, document.inline_shapes[0].height),
                         (Pt(72), Pt(36)), "image.dimensions")
        self.assertEqual([(c.author, c.text) for c in document.comments],
                         [("Default Author", "Run note"), ("Paragraph Author", "Paragraph note")],
                         "comments.content")
        self.assert_comment_spans({"0": "Formatted ", "1": "Formatted supsub\tafter tab\nafter line"})
        for name, expected in {
            "author": "Rust Author", "title": "Parity report", "subject": "Testing",
            "comments": "Core comments", "keywords": "rust, docx", "category": "Reports", "version": "1.0",
        }.items():
            with self.subTest(metadata=name):
                self.assertEqual(getattr(document.core_properties, name), expected)

    def test_images(self):
        self.assertEqual(len(self.document.inline_shapes), 6)
        expected = [(36, 18), (12, 6), (24, 12), (12, 6), (48, 24), (72, 36)]
        for shape, (width, height) in zip(self.document.inline_shapes, expected):
            self.assertEqual((shape.width, shape.height), (Pt(width), Pt(height)))
        # Accessibility fields currently have no public python-docx accessor.
        properties = self.document.inline_shapes[0]._inline.docPr
        self.assertEqual(properties.get("title"), "Title <&>")
        self.assertEqual(properties.get("descr"), "Red sample")
        self.assertEqual(properties.get("name"), "Sample PNG")
        image_parts = [rel.target_part for rel in self.document.part.rels.values()
                       if rel.reltype == RELATIONSHIP_TYPE.IMAGE]
        self.assertEqual(len(image_parts), 5, "Repeated PNG must share an image part")
        fixtures = Path(__file__).resolve().parents[1] / "fixtures" / "images"
        for index, (shape, extension) in enumerate(zip(
            self.document.inline_shapes, ["png", "jpg", "gif", "bmp", "tiff", "png"]
        )):
            identifier = shape._inline.graphic.graphicData.pic.blipFill.blip.get(qn("r:embed"))
            image_part = self.document.part.related_parts[identifier]
            self.assertEqual(image_part.blob, (fixtures / f"sample.{extension}").read_bytes(),
                             f"image[{index}].content")
        self.assertEqual({part.blob for part in image_parts},
                         {(fixtures / f"sample.{extension}").read_bytes()
                          for extension in ["png", "jpg", "gif", "bmp", "tiff"]})
        self.assertEqual({part.content_type for part in image_parts},
                         {"image/png", "image/jpeg", "image/gif", "image/bmp", "image/tiff"})

    def test_layout(self):
        section = self.document.sections[0]
        self.assertEqual(section.orientation, WD_ORIENT.LANDSCAPE)
        self.assertEqual((section.page_width, section.page_height), (Inches(11), Inches(8.5)))
        self.assertEqual(section.left_margin, Inches(.5))
        self.assertEqual(section.right_margin, Inches(.75))
        self.assertEqual(section.start_type, WD_SECTION_START.ODD_PAGE)
        self.assertTrue(section.different_first_page_header_footer)
        # These properties have no high-level python-docx accessor; use its XML model.
        columns = section._sectPr.find(qn("w:cols"))
        self.assertEqual([columns.get(qn(f"w:{name}")) for name in ["num", "space", "sep"]],
                         ["2", "360", "1"])
        numbering = section._sectPr.find(qn("w:pgNumType"))
        self.assertEqual(numbering.get(qn("w:start")), "7")
        self.assertEqual(numbering.get(qn("w:fmt")), "lowerRoman")
        self.assertEqual(section._sectPr.find(qn("w:vAlign")).get(qn("w:val")), "center")
        paragraph = self.document.paragraphs[0]
        self.assertEqual(paragraph.text, "Numbered item")
        self.assertEqual(paragraph._p.pPr.numPr.numId.val, 42)
        self.assertEqual(paragraph._p.pPr.numPr.ilvl.val, 0)
        numbering = self.document.part.numbering_part.element
        number = numbering.xpath('w:num[@w:numId="42"]')[0]
        identifier = number.find(qn("w:abstractNumId")).get(qn("w:val"))
        level = numbering.xpath(f'w:abstractNum[@w:abstractNumId="{identifier}"]/w:lvl')[0]
        for name, expected in [("start", "3"), ("numFmt", "decimal"), ("lvlText", "%1.")]:
            self.assertEqual(level.find(qn(f"w:{name}")).get(qn("w:val")), expected)

    def test_template(self):
        document = self.document
        self.assertEqual([p.text for p in document.paragraphs],
                         ["Hello Alice", "Inserted {{NAME}}", "", "Additional comment", "", "Tail"],
                         "template.paragraphs.text")
        self.assertEqual(document.paragraphs[0].style.name, "TemplateBody")
        self.assertIs(document.paragraphs[0].runs[0].bold, True, "template.run.bold")
        self.assertTrue(any(r.italic is True for r in document.paragraphs[0].runs))
        self.assertEqual(document.styles["TemplateBody"].font.name, "Courier New")
        self.assertEqual(document.styles["TemplateBody"].font.size, Pt(11))
        self.assertEqual(document.tables[0].cell(0, 0).text, "Alice", "template.table.text")
        self.assertEqual([p.text for p in document.sections[0].header.paragraphs],
                         ["", "Header Alice", "Header extension"], "template.header.text")
        self.assertEqual(document.sections[0].footer.paragraphs[-1].text, "Footer Alice")
        self.assertEqual([(c.author, c.text) for c in document.comments],
                         [("Original Author", "Original note"), ("New Author", "New note")])
        self.assert_comment_spans({"0": "Hello Alice", "1": "Additional comment"})
        self.assertEqual([(s.width, s.height) for s in document.inline_shapes],
                         [(Pt(12), Pt(6)), (Pt(48), Pt(24))])
        self.assertEqual(document.core_properties.title, "Updated title")
        self.assertEqual(document.core_properties.subject, "Retained subject")
        self.assertEqual(document.core_properties.author, "Template Author")
        self.assertIsNotNone(self.source, "Template test requires the original source file")
        source = Document(self.source)
        self.assertEqual(source.paragraphs[0].text, "Hello {{NAME}}")
        self.assertEqual(source.tables[0].cell(0, 0).text, "{{NAME}}")
        self.assertEqual(source.core_properties.title, "Template title")
        self.assertEqual(len(source.comments), 1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=["empty", "text", "dependencies", "rich", "images", "layout", "template", "r1", "hyperlinks", "ranges", "notes", "fields", "line_heights", "tab_stops", "row_properties", "cell_layout", "script_formatting", "style_references", "character_styles", "numbering_overrides", "section_direction", "conditional_paragraph_formatting", "conditional_script_formatting", "document_defaults", "linked_styles", "section_gutter", "numbering_replacements", "table_look", "numbering_suffix", "numbering_restart", "numbering_alignment", "table_alignment"])
    parser.add_argument("document", type=Path)
    parser.add_argument("--source", type=Path)
    arguments = parser.parse_args()
    suite = unittest.TestSuite([Readback(arguments.case, arguments.document, arguments.source)])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
