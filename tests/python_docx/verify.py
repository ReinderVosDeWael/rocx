"""Read Rust-generated files using python-docx, without a CMI source checkout.

Invoked by tests/python_docx.rs. unittest assertions remain active under python -O.
"""

import argparse
from pathlib import Path
import sys
import unittest

try:
    import docx
except ImportError as error:
    raise SystemExit(
        "Run `uv sync --locked --group test` from the repository root."
    ) from error

from docx import Document
from docx.enum.section import WD_ORIENT, WD_SECTION_START
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
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
    parser.add_argument("case", choices=["empty", "text", "rich", "images", "layout", "template"])
    parser.add_argument("document", type=Path)
    parser.add_argument("--source", type=Path)
    arguments = parser.parse_args()
    suite = unittest.TestSuite([Readback(arguments.case, arguments.document, arguments.source)])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
