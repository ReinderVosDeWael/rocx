"""Independent Python visual catalog, byte-matched to Rust and JavaScript.

Run: uv run --no-sync python examples/all_features.py [output.docx]
See ALL_FEATURES.md for coverage, viewer caveats and parity verification.
The catalog includes the native counterpart of the named JavaScript report,
also used for legacy API parity.
"""
import asyncio
import pathlib
import sys

import rocx
from rocx import declarative as d
from rocx.editor import (
    HeaderImageEdit, ImportedDocument, ImportedHyperlink, ImportedRun, ParagraphAlignmentEdit,
    TableInsertion, RunTextEdit, CellAlignmentEdit, ParagraphInsertion,
)

from javascript_workflow import blocks as javascript_blocks

STAMP = "2026-11-19T00:00:00Z"
IMAGES = pathlib.Path(__file__).resolve().parents[1] / "tests/fixtures/images"
EDIT_LABEL = "Imported editor: this paragraph changes from left to center in the edited catalog."
INLINE_LABEL = "Imported editor inline structure: "
INLINE_LINK_RUNS = ("jump to ", "CatalogBookmark")
HEADER_INLINE_LABEL = "Default-header inline structure: "
HEADER_BATCH_LABEL = "Combined editor batch: this paragraph is centered and followed by a table in both image previews."
HEADER_LINK_RUNS = ("shared ", "link")
REGIONS = ["whole_table", "first_row", "last_row", "first_column", "last_column",
           "banding_1_row", "banding_2_row", "banding_1_column", "banding_2_column",
           "top_left_cell", "top_right_cell", "bottom_left_cell", "bottom_right_cell"]


def p(text: str) -> d.Paragraph:
    """Create a plainly labeled paragraph."""
    return d.Paragraph(text=text)


def heading(text: str) -> d.Paragraph:
    """Start a catalog topic on a new page."""
    return d.Paragraph(text=text, heading=1, page_break_before=True)


def run(text: str, **formatting) -> d.Paragraph:
    """Show one run-formatting request."""
    return d.Paragraph(children=[d.TextRun(text, **formatting)])


def cell(text: str) -> d.TableCell:
    """Create a labeled table cell."""
    return d.TableCell([p(text)])


def image(ext: str, size: dict | None = None) -> d.ImageRun:
    """Embed identical fixture bytes with explicit accessibility metadata."""
    filename = f"sample.{ext}" if ext != "zero" else "sample_zero_dpi.png"
    return d.ImageRun((IMAGES / filename).read_bytes(), transformation=size,
        alt_text={"title": f"Image {ext}", "description": f"Accessible {ext} sample", "name": f"Sample {ext}"})


def formatting_blocks() -> list:
    """Paragraph, run and style galleries, in the same order as Rust."""
    blocks = [heading("01 Paragraphs and headings")]
    blocks.append(d.Paragraph(text=EDIT_LABEL, alignment="left"))
    for level in range(10):
        blocks.append(d.Paragraph(text=f"This is a Heading {level}" + (" (heading=0: no heading style)" if level == 0 else ""), heading=level))
    for alignment, name in zip([0, 1, 2, 3, 4, 5, 7, 8, 9],
            ["left", "center", "right", "justify", "distribute", "justifyMedium", "justifyHigh", "justifyLow", "thaiJustify"], strict=True):
        blocks.append(d.Paragraph(text=f"Paragraph alignment={name}: The quick brown fox jumps over the lazy dog. Ελληνικά العربية 日本語.", alignment=alignment))
    for height, name in [(d.LineHeight.multiple(1.5), "multiple 1.5"), (d.LineHeight.exact(20), "exact 20 pt"), (d.LineHeight.at_least(18), "at_least 18 pt")]:
        blocks.append(d.Paragraph(children=[d.TextRun(f"Line height={name}; before=6 pt; after=8 pt"), d.Break(),
            d.TextRun("Second line makes the line-height request visible.")], line_height=height, spacing_before=6, spacing_after=8))
    blocks.append(d.Paragraph(text="Legacy line_spacing=1.25; left=18 pt, right=12 pt, hanging=6 pt; keep lines/next and widow control enabled.",
        line_spacing=1.25, left_indent=18, right_indent=12, first_line_indent=-6, keep_together=True, keep_with_next=True, widow_control=True))
    blocks.append(d.Paragraph(text="First-line indent=18 pt; keep lines/next, page-break-before and widow control explicitly false.",
        first_line_indent=18, keep_together=False, keep_with_next=False, page_break_before=False, widow_control=False, bidi=False))
    for name in ["left", "center", "right", "decimal", "bar", "clear", "start", "end"]:
        blocks.append(d.Paragraph(children=[d.TextRun(f"Tab {name}"), d.Tab(), d.TextRun("123.45")], tab_stops=[d.TabStop(216, name)]))
    for leader in ["none", "dot", "hyphen", "underscore", "heavy", "middle_dot"]:
        blocks.append(d.Paragraph(children=[d.TextRun(f"Tab leader={leader}"), d.Tab(), d.TextRun("at 288 pt")],
            tab_stops=[d.TabStop(288, "right", leader), d.TabStop(-12, "clear")]))
    blocks.append(heading("02 Run formatting, colors, scripts and whitespace"))
    blocks.append(p("Formatting declarations in Rust, Python and JavaScript: omitted, enabled and explicitly disabled bold."))
    blocks.append(d.Paragraph(children=[d.TextRun("Builder bold unspecified; "),
        d.TextRun("Builder bold enabled; ", bold=True),
        d.TextRun("Builder bold disabled, italic enabled.", bold=False, italic=True)],
        spacing_after=6, keep_with_next=False))
    for name, fmt in [("Bold=true", {"bold": True}), ("Italic=true", {"italic": True}),
            ("Strike=true", {"strike": True}), ("ALL CAPS=true", {"all_caps": True}),
            ("Small caps=true", {"small_caps": True}), ("Superscript=true", {"superscript": True}),
            ("Subscript=true", {"subscript": True}), ("Arial 16 pt, RGB 803020", {"font": "Arial", "size": 16, "color": (128, 48, 32)}),
            ("Automatic color", {"color": "auto"})]:
        blocks.append(run(name, **fmt))
    for underline in ["none", "single", "double", "dotted", "dash", "wave", "words"]:
        blocks.append(run(f"Underline={underline}: words with spaces", underline=underline))
    for color in ["dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
                  "hyperlink", "followedHyperlink", "background1", "text1", "background2", "text2"]:
        blocks.append(d.Paragraph(children=[d.TextRun(f"Theme {color} (light colors may be invisible on white): "),
            d.TextRun("colored text", color={"theme": color, "tint": 32, "shade": 224})]))
    blocks.append(run("Explicit false overrides: bold, italic, strike, caps, small caps, superscript, subscript and underline disabled.",
        bold=False, italic=False, strike=False, all_caps=False, small_caps=False, superscript=False, subscript=False, underline="none"))
    blocks.append(d.Paragraph(children=[d.TextRun("  Preserved leading and trailing spaces  "), d.TextRun(""),
        d.TextRun("Unicode: café Ελληνικά 日本語 العربية; literal tab\tand newline\nend.")]))
    blocks.append(d.Paragraph(children=[d.TextRun("Run language/script: en-US / ja-JP / ar-SA; العربية 日本語",
        language=d.RunLanguage("en-US", "ja-JP", "ar-SA"), bold_complex_script=True, italic_complex_script=False,
        size_complex_script=14, rtl=True, complex_script=True, font_east_asia="Yu Mincho", font_complex_script="Arial")], bidi=True))
    blocks.append(run("Run RTL=false, complex_script=false, bCs=false, iCs=true; independent from paragraph direction.",
        rtl=False, complex_script=False, bold_complex_script=False, italic_complex_script=True))
    blocks.append(heading("03 Styles, inheritance, links and direct overrides"))
    blocks.append(d.Paragraph(children=[d.TextRun("Paragraph style Catalog Child (forward base/next/link); "),
        d.TextRun("character style Catalog Character with bold=false override", style="Catalog Character", bold=False)], style="Catalog Child"))
    blocks.append(d.Paragraph(children=[d.TextRun("Updated template paragraph/character styles: original bold retained, italic added.",
        style="Catalog Existing Character")], style="Catalog Existing"))
    return blocks


def styles() -> list:
    """Author forward style references without copying inherited formatting."""
    result = [d.ParagraphStyleDefinition(name="Catalog Child", base_style="Catalog Base", next_paragraph_style="Normal",
        linked_style="Catalog Character", line_height=d.LineHeight.at_least(16), tab_stops=[d.TabStop(72)], bidi=False),
        d.CharacterStyleDefinition(name="Catalog Character", base_style="Catalog Character Base", linked_style="Catalog Child",
            bold=True, italic=True, color=(48, 64, 144)),
        d.ParagraphStyleDefinition(name="Catalog Base", base_style="Normal", font="Arial", font_size=12, bold=True,
            italic=False, underline=False, color=(24, 64, 96), alignment="left", spacing_before=6, spacing_after=6,
            left_indent=0.25, right_indent=0.125, first_line_indent=-0.125, keep_together=True, keep_with_next=False,
            page_break_before=False, widow_control=True, language=d.RunLanguage(latin="en-US"),
            bold_complex_script=False, italic_complex_script=False, size_complex_script=12, rtl=False,
            complex_script=False, font_east_asia="Yu Mincho", font_complex_script="Arial"),
        d.CharacterStyleDefinition(name="Catalog Character Base", font="Calibri", size=12),
        d.ParagraphStyleDefinition(name="Catalog Existing", italic=True),
        d.CharacterStyleDefinition(name="Catalog Existing Character", italic=True)]
    for name in REGIONS:
        fmt = d.TableSectionFormat(bold=True, italic=False, underline=False, font="Calibri", font_size=10, color=(32, 48, 96),
            background=(224, 238, 255), alignment="center", spacing_before=2, spacing_after=2,
            line_height=d.LineHeight.at_least(12), tab_stops=[d.TabStop(36)], bidi=False,
            language=d.RunLanguage("en-US", "ja-JP", "ar-SA"), bold_complex_script=False, italic_complex_script=False,
            size_complex_script=10, rtl=False, complex_script=False, font_east_asia="Yu Mincho", font_complex_script="Arial")
        result.append(d.TableStyleDefinition(name=f"Region {name}", base_style="Table Grid", **{name: fmt}))
    return result


def table_blocks() -> list:
    """Show layouts, merges, row requests, every cell direction and region."""
    blocks = [heading("04 Tables: layout, rows, cells, merges and nested content"),
        p("Fixed table: 3 columns of 1440 twips; all six table-border sides; top/bottom/start/end cell borders; padding top=3, start=0, bottom=3, end=6 pt. Header row repeats; cannot split; minimum height=28 pt.")]
    top = d.TableCell([p("Horizontal span=2; vertical merge restart; shaded; vertically centered")], grid_span=2, vmerge="restart",
        background_color=(255, 238, 204), vertical_alignment="center", margins=d.CellMargins(3, 0, 3, 6),
        borders=[d.CellBorder(side, sz=8, color=(128, 64, 32), val="dashed") for side in ["top", "bottom", "start", "end"]])
    blocks.append(d.Table([
        d.TableRow([top, cell("Top aligned cell")], height=d.RowHeight.at_least(28), repeat_header=True, cant_split=True),
        d.TableRow([d.TableCell([p("")], grid_span=2, vmerge="continue"), d.TableCell([p("Bottom aligned cell")], vertical_alignment="bottom")],
            height=d.RowHeight.exact(48), repeat_header=False, cant_split=False),
        d.TableRow([cell("Short row: padded grid"), cell("Second cell")])], column_widths=[1440, 1440, 1440], layout="fixed",
        borders=[d.TableBorder(side, sz=8, color=(32, 64, 96), val="single") for side in ["top", "left", "bottom", "right", "insideH", "insideV"]]))
    blocks.append(p("Autofit table with nested table and trailing paragraph (nested tables need trailing paragraphs)."))
    blocks.append(d.Table([d.TableRow([d.TableCell([d.Table([d.TableRow([cell("Nested fixed-width table")])], column_widths=[1440], layout="fixed"),
        p("Paragraph after nested table")]), cell("Autofit neighbor")])], layout="autofit", column_widths=[2000, 2000]))
    for direction in ["lrTb", "tbRl", "btLr", "lrTbV", "tbRlV", "tbLrV"]:
        blocks.append(p(f"Cell text_direction={direction}; independent from paragraph bidi."))
        blocks.append(d.Table([d.TableRow([d.TableCell([p(f"Text flow {direction}")], text_direction=direction, vertical_alignment="top")],
            height=d.RowHeight.at_least(72))]))
    for alignment in ["left", "center", "right", "start", "end"]:
        blocks.append(p(f"Inline table alignment={alignment}; fixed width=2160 twips. No floating positioning or paragraph alignment is requested."))
        blocks.append(d.Table([d.TableRow([cell(f"Table alignment: {alignment}")])], alignment=alignment, column_widths=[2160]))
    blocks.append(p("Repeated-header pagination sample: 36 rows follow. The consumer decides actual page breaks."))
    blocks.append(d.Table([d.TableRow([cell("Repeated header: index" if i == 0 else f"Row {i}"), cell("Minimum 20 pt; cannot split")],
        height=d.RowHeight.at_least(20), repeat_header=i == 0, cant_split=True) for i in range(36)]))
    blocks.append(p("Table look: all six activation flags=false (mask 0600). Whole-table formatting remains unconditional; no region formatting is copied to direct cells."))
    blocks.append(d.Table([d.TableRow([cell("All conditional regions disabled; base whole_table still applies")])], style="Region whole_table",
        look=d.TableLook(first_row=False, last_row=False, first_column=False, last_column=False, horizontal_banding=False, vertical_banding=False)))
    blocks.append(heading("05 Conditional table styles: all 13 regions"))
    blocks.append(p("Each table references a separate region style. All six table-look activation flags are explicitly enabled (mask 01E0). Region fill/font/paragraph/script requests stay in styles.xml, not direct cell formatting; the viewer applies conditional precedence. Base whole_table is unconditional."))
    for name in REGIONS:
        blocks.append(p(f"Conditional region={name}: blue shading, bold 10 pt, centered, minimum line=12 pt, tab=36 pt, bidi/RTL=false; language/script/font slots authored."))
        blocks.append(d.Table([d.TableRow([cell(f"{name} r{r}c{c}") for c in range(1, 4)]) for r in range(1, 4)], style=f"Region {name}",
            look=d.TableLook(first_row=True, last_row=True, first_column=True, last_column=True, horizontal_banding=True, vertical_banding=True)))
    return blocks


def link(label: str) -> d.Hyperlink:
    """Use explicit display formatting; hyperlink targets are never fetched."""
    return d.Hyperlink("https://example.com/inspection?q=docx#sample", [d.TextRun(label, color={"theme": "hyperlink"}, underline="single")],
        tooltip="External link; never fetched by the library", history=True)


def story_blocks() -> list:
    """Lists, rich stories, range pairs, field caches and image formats."""
    blocks = [heading("06 Lists, hyperlinks, bookmarks, comments, notes and cached fields")]
    blocks.append(p("Concrete numbering-level replacements (numId=60): level 0 uses lowerLetter, text %1), replacement start=3 plus separate startOverride=7; level 1 uses upperRoman, text (%2), start=4. Abstract formats/starts are unchanged. The viewer computes the resulting labels."))
    blocks.append(p("Legal numbering requests: abstract level 2 and replacement level 1 enable decimal display of referenced levels; replacement level 0 explicitly disables it. Other levels omit the request. numFmt tokens remain unchanged; the viewer renders labels."))
    blocks.append(p("Numbering suffix requests: abstract level 2 explicitly uses tab; replacement level 0 uses space; replacement level 1 uses nothing. Other levels omit suff (consumer default: tab). Separators are not inserted into paragraph text by the library."))
    blocks.append(p("Level restart requests: abstract level 2 and replacement level 0 use 0 (never restart); replacement level 1 uses trigger 1 (one-based first level). Other levels omit lvlRestart (restart after the preceding level). This is independent of concrete startOverride; the viewer calculates counters."))
    blocks.append(p("Numbering-label alignment: abstract levels 0..4 request left, center, right, start, end; replacement levels 0/1 request end/start. Remaining levels omit lvlJc (direction-dependent consumer default). Paragraph text alignment is independent; the viewer positions generated labels."))
    blocks.append(p("Numbering label formatting: abstract level 0 requests bold; replacement level 0 requests bold off and italic on."))
    blocks.append(p("Numbering level paragraph properties: abstract level 0 requests 24 pt left and 12 pt hanging; replacement level 0 requests 36 pt left."))
    blocks.append(p("Numbering level style links: abstract level 0 links to Heading 1; replacement level 0 links to Heading 2."))
    for level in range(9):
        blocks.append(d.Paragraph(text=f"Numbered list: concrete numId=60 level={level}; abstract start=1; concrete level 0 restart=7, level 8 restart=0. Labels are computed by your viewer.", numbering=d.NumberingReference(60, level)))
    blocks.append(d.Paragraph(text="Bullet list: numId=61, literal bullet label.", numbering=d.NumberingReference(61, 0)))
    blocks.append(d.Paragraph(text="Shared abstract numbering: numId=62 reuses the bullet levels of numId=61 with its own start override.", numbering=d.NumberingReference(62)))
    blocks.append(p("Numbering style: Catalog Bullets links concrete numId=61 to abstract definition 100."))
    blocks.append(d.Paragraph(text="Inherited numbering style: numId=63 gets its levels from Catalog Bullets.", numbering=d.NumberingReference(63)))
    blocks.append(d.Paragraph(children=[link("External hyperlink: example.com (tooltip and history=true)")]))
    blocks.append(d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("CatalogBookmark"), [d.TextRun("Internal hyperlink: jump to CatalogBookmark")], history=False)]))
    blocks.append(d.Paragraph(children=[
        d.TextRun(INLINE_LABEL),
        d.Hyperlink(d.BookmarkTarget("CatalogBookmark"), [d.TextRun(text) for text in INLINE_LINK_RUNS]),
    ]))
    blocks.append(d.Paragraph(children=[d.RangeStart.bookmark("CatalogBookmark"), d.TextRun("Bookmark CatalogBookmark starts here. "),
        d.RangeStart.bookmark("OverlappingBookmark"), d.RangeStart.comment("cross-paragraph-review", "Cross-paragraph plain comment", "Range reviewer"),
        d.TextRun("Overlapping bookmark and comment begin.")]))
    blocks.append(d.Paragraph(children=[d.TextRun("Cross-paragraph bookmark ends; overlapping bookmark continues. "),
        d.RangeEnd.bookmark("CatalogBookmark"), d.RangeEnd.bookmark("OverlappingBookmark"), d.RangeEnd("cross-paragraph-review")]))
    comment = d.Comment([p("Structured comment: paragraph, table, image and hyperlink."), d.Table([d.TableRow([cell("Comment table")])]),
        d.Paragraph(children=[image("png", {"width": 18}), link("Comment external link")])], author="Structured reviewer", date=STAMP, initials="SR")
    blocks.append(d.Paragraph(children=[d.RangeStart.structured_comment("structured-review", comment),
        d.TextRun("Structured comment anchor: inspect the comment pane."), d.RangeEnd("structured-review")]))
    blocks.append(d.Paragraph(children=[d.TextRun("Run comment convenience anchor", comment_text="Run convenience comment", comment_author="Run reviewer")],
        comment_text="Paragraph convenience comment", comment_author="Paragraph reviewer"))
    blocks.append(d.Paragraph(children=[d.TextRun("Footnote reference"), d.NoteReference.footnote(0), d.TextRun("; endnote reference"), d.NoteReference.endnote(0)]))
    blocks.append(p("Note numbering: lower-Roman footnotes start at 3 each page; uppercase-letter endnotes start at 7 each section. Footnotes are requested beneath text and endnotes at section end."))
    blocks.append(p("Tracked text revisions: inserted text and deleted text retain review metadata. Tracking future edits is explicitly off."))
    blocks.append(d.Paragraph(children=[
        d.TextRun("Current: "),
        d.TextRun("inserted wording", revision=d.TextRevision("insertion", "Catalog editor", "2026-11-20T00:00:00Z")),
        d.TextRun("old wording", revision=d.TextRevision("deletion", "Catalog editor")),
    ]))
    blocks.append(p("Inline content control: rich text, alias Review caption, tag review-caption, ID 5001, content locked."))
    blocks.append(d.Paragraph(children=[
        d.TextRun("Review: "),
        d.InlineContentControl([d.TextRun("controlled caption", bold=True)],
                               alias="Review caption", tag="review-caption", lock="contentLocked", id=5001),
    ]))
    blocks.append(p("Block content control: a controlled paragraph and table share an alias, tag and ID 5002."))
    blocks.append(d.BlockContentControl([
        p("Controlled block paragraph"), d.Table([d.TableRow([cell("Controlled table cell")])]),
    ], alias="Review block", tag="review-block", lock="sdtLocked", id=5002))
    blocks.append(p("Plain-text control: one run, multiline enabled."))
    blocks.append(d.Paragraph(children=[
        d.InlineContentControl([d.TextRun("first line\nsecond line")],
                               tag="plain-review", text_type="plain", multi_line=True),
    ]))
    blocks.append(p("Drop-down control: Review status offers Pending and Accepted, with Accepted selected."))
    blocks.append(d.Paragraph(children=[
        d.InlineContentControl([d.TextRun("Accepted")], alias="Review status", tag="review-status",
            text_type="dropdown", items=[
                d.ContentControlListItem("Pending", "pending"),
                d.ContentControlListItem("Accepted", "accepted"),
            ], selected="accepted"),
    ]))
    blocks.append(p("Tracked text move: CatalogMove pairs a source and a destination."))
    for caption, kind in [("Move source: ", "move_source"), ("Move destination: ", "move_destination")]:
        blocks.append(d.Paragraph(children=[
            d.TextRun(caption),
            d.TextRun("relocated wording", revision=d.TextRevision(
                kind, "Catalog editor", "2026-11-20T00:00:00Z", "CatalogMove")),
        ]))
    blocks.append(p("Table content controls: one row and its cell have separate tags."))
    blocks.append(d.Table([
        d.TableRow([cell("Uncontrolled table cell")]),
        d.TableRow([d.TableCell([p("Controlled row and cell")],
            content_control=d.ContentControlMetadata(tag="review-cell", id=5004))],
            content_control=d.ContentControlMetadata(tag="review-row", id=5003)),
    ]))
    for instruction, label in [(d.FieldInstruction.page(), "PAGE"), (d.FieldInstruction.num_pages(), "NUMPAGES"),
            (d.FieldInstruction.reference("CatalogBookmark"), "REF CatalogBookmark"), (d.FieldInstruction.table_of_contents(), "TOC"),
            (d.FieldInstruction.raw(" DATE "), "raw DATE")]:
        for form in ["simple", "complex"]:
            blocks.append(d.Paragraph(children=[d.TextRun(f"Field {form} {label}: "), d.Field(instruction,
                [d.TextRun(f"[{label} cached placeholder]")], form=form, dirty=True, locked=False)]))
    blocks.append(d.Paragraph(children=[d.TextRun("Nested cached field (outer complex locked=true): "),
        d.Field(d.FieldInstruction.raw(" IF 1 = 1 "), [d.Field(d.FieldInstruction.page(), [d.TextRun("[nested PAGE cache]")])],
            form="complex", dirty=False, locked=True)]))
    blocks.append(heading("07 Inline images and explicit breaks"))
    for ext, label, size in [("png", "PNG width-only 48 pt (aspect retained)", {"width": 48}),
            ("jpg", "JPEG height-only 36 pt (aspect retained)", {"height": 36}), ("gif", "GIF both dimensions 48 x 24 pt", {"width": 48, "height": 24}),
            ("bmp", "BMP native density-derived size", None), ("tiff", "TIFF native density-derived size", None), ("zero", "PNG zero-DPI fallback size", None)]:
        blocks.append(p(f"Image: {label}; accessibility title/description/name authored."))
        blocks.append(d.Paragraph(children=[image(ext, size)]))
    blocks.append(d.Paragraph(children=[d.Hyperlink("https://example.com/image", [image("png", {"width": 24}), d.Tab(),
        d.TextRun("Linked image, tab and line break"), d.Break(), d.TextRun("Same PNG bytes are deduplicated in the package.")])]))
    blocks.append(p("Image crop and rotation: 10% left, 5% top, 20% right and 15% bottom; 30 degrees clockwise, horizontal flip on and vertical flip off."))
    blocks.append(d.Paragraph(children=[image("png", {"width": 60, "height": 40,
        "rotation": 30, "flip_horizontal": True, "flip_vertical": False, "crop": d.ImageCrop(10, 5, 20, 15)})]))
    blocks.append(p("Floating picture: column-relative horizontal offset 250 pt, page-relative bottom alignment, square wrapping on the largest side; gaps top 0, bottom 2, left 3, right 4 pt. Anchor locked; in-cell layout and overlap off."))
    blocks.append(d.Paragraph(children=[image("png", {"width": 42, "anchor": d.ImageAnchor(
        horizontal_reference="column", x=250, vertical_alignment="bottom", wrap="square", relative_height=4, wrap_side="largest",
        distance_top=0, distance_bottom=2, distance_left=3, distance_right=4,
        locked=True, layout_in_cell=False, allow_overlap=False,
    )})]))
    blocks.append(p("Floating picture: tight text wrapping around an authored triangular contour."))
    blocks.append(d.Paragraph(children=[image("png", {"width": 42, "anchor": d.ImageAnchor(
        x=360, y=160, wrap="tight", wrap_polygon=[(0, 0), (21600, 21600), (0, 21600)],
    )})]))
    blocks.append(p("Inline DrawingML shapes: a blue rectangle with text and a green ellipse with a dark outline."))
    blocks.append(p("Shape text style: blue rectangle, 14.25 pt white Arial text, bold explicitly false, italic true, centered horizontally and bottom anchored. Blank middle line preserved; ellipse has no text body."))
    blocks.append(d.Paragraph(children=[
        d.ShapeRun(96, 64, fill=(32, 80, 176), text="Blue\n\nText", text_format=d.ShapeTextFormat(
            font_size=14.25, bold=False, color=(255, 255, 255), alignment="center", vertical_alignment="bottom", italic=True, font="Arial")),
        d.ShapeRun(44, 32, geometry="ellipse", fill=(32, 144, 80), outline=d.ShapeOutline(1.5, (16, 48, 32))),
    ]))
    blocks.append(p("Standalone inline text box: 144 by 48 points, plain multiline DrawingML text, Arial 12 pt, bold explicitly false; no geometric fill or outline, no rich WordprocessingML story."))
    blocks.append(d.Paragraph(children=[d.ShapeRun(144, 48, geometry="text_box", text="Standalone\nText box",
        text_format=d.ShapeTextFormat(font="Arial", font_size=12, bold=False))]))
    blocks.append(d.Paragraph(children=[
        d.TextRun("Floating text box: 220 pt from the margin, 12 pt below its paragraph, square wrapping with a 6 pt gap on the left."),
        d.ShapeRun(108, 24, geometry="text_box", text="Floating note", anchor=d.ImageAnchor(
            horizontal_reference="margin", vertical_reference="paragraph",
            x=220, y=12, wrap="square", distance_left=6,
        )),
    ]))
    blocks.append(p("Inline column chart: North 8, South 13.5, West 5; one series without an embedded workbook."))
    blocks.append(d.Paragraph(children=[
        d.ColumnChartRun(288, 180, ["North", "South", "West"], [8, 13.5, 5]),
    ]))
    blocks.append(d.Paragraph(children=[
        d.TextRun("Inline equation: stacked fraction "), d.InlineFraction("1", "2"), d.TextRun("."),
    ]))
    blocks.append(p("Image from a named fixture (filename preserved), followed by async byte data with deferred size/alt text."))
    async def image_bytes() -> bytes:
        return (IMAGES / "sample.png").read_bytes()
    blocks.append(d.Paragraph(children=[d.ImageRun(IMAGES / "sample.png", type="png", transformation={"width": 24}),
        d.ImageRun(image_bytes, transformation=lambda: {"height": 24}, alt_text=lambda: {"description": "Deferred image accessibility"})]))
    for kind, name in [("line", "line"), ("textWrapping", "textWrapping"), ("page", "page"), ("column", "column")]:
        blocks.append(d.Paragraph(children=[d.TextRun(f"Before explicit {name} break"), d.Break(kind), d.TextRun(f"After explicit {name} break")]))
    return blocks


async def generate(output: pathlib.Path) -> bytes:
    """Save the catalog and its template; return deterministic final bytes."""
    seed = d.Paragraph(children=[d.TextRun("Template {{re"), d.TextRun("place}}: cross-run replacement.")])
    tagged_seed = d.Paragraph(children=[
        d.TextRun("Template plain-text control replacement: "),
        d.InlineContentControl([d.TextRun("old controlled value", bold=True)],
                               tag="template-control", lock="sdtLocked", text_type="plain"),
    ])
    revision_seed = d.Paragraph(children=[
        d.TextRun("Template revision action: "),
        d.TextRun("accepted template wording", revision=d.TextRevision("insertion", "Template editor")),
        d.TextRun("removed template wording", revision=d.TextRevision("deletion", "Template editor")),
    ])
    move_seed = d.Paragraph(children=[
        d.TextRun("Template accepted move: "),
        d.TextRun("moved template wording", revision=d.TextRevision("move_source", "Template editor", STAMP, "TemplateMove")),
        d.TextRun("destination: "),
        d.TextRun("moved template wording", revision=d.TextRevision("move_destination", "Template editor", STAMP, "TemplateMove")),
    ])
    template_path = output.with_suffix(".template.docx")
    await d.Document([d.Section([seed, p("Template retained paragraph: insertion appears before this."), tagged_seed, revision_seed, move_seed])], timestamp=STAMP,
        custom_properties={"Project": "Atlas", "Review status": "Pending", "Review complete": False},
        styles=[d.ParagraphStyleDefinition(name="Catalog Existing", bold=True),
            d.CharacterStyleDefinition(name="Catalog Existing Character", bold=True)]).save(template_path)
    blocks = [p("ALL FEATURES — visual inspection catalog"),
        p("Each sample names its requested feature. Field caches are placeholders; update fields in your viewer. Font fallback, pagination and conditional table-style activation depend on the viewer. Inspect comments, footnotes, endnotes and document properties too."),
        p("Document defaults: Calibri 11 pt, after spacing 4 pt, 1.1 line height. See template replacement at the start and the retained template paragraph after topic 08."),
        p("Template safety: this sample uses an unsigned template. Templates with an OPC signature origin are rejected before any output is written."),
        p("Custom document properties: Project (Atlas) is retained; Review status changes to Draft & review."),
        p("Boolean custom properties: Review complete changes from false to true; Published is explicitly false."),
        p("Template compatibility: unsupported mandatory namespaces in edited WordprocessingML are rejected."),
        p("Extended document properties: Company is Atlas & Co.; Manager is Taylor.")]
    blocks.extend(formatting_blocks())
    blocks.extend(table_blocks())
    blocks.extend(story_blocks())
    blocks.extend(javascript_blocks())
    blocks.append(heading("08 Authoring: ready, lazy, async, references, conditions and snapshot reuse"))
    blocks.append(p("Ready value: this paragraph was supplied immediately."))
    blocks.append(d.Paragraph(text=lambda: "Lazy value: this paragraph was produced by a factory."))
    async def async_text() -> str:
        return "Async value: this paragraph was produced by an awaitable."
    blocks.append(d.Paragraph(text=async_text))
    async def shared_text() -> str:
        return "Shared reference: computed once and used twice."
    shared = d.Paragraph(text=shared_text)
    blocks.extend([d.Paragraph(text=shared.deferred("text")), d.Paragraph(text=shared.deferred("text"))])
    blocks.append(d.Paragraph(text=d.Deferred(lambda: "Explicit Deferred: shared computation.")))
    blocks.append(d.Paragraph(text="Condition true: this paragraph is visible.", condition=True))
    def excluded() -> str:
        raise AssertionError("excluded producer must not execute")
    blocks.append(d.Paragraph(text=excluded, condition=False))
    blocks.append(p("Condition false: the excluded paragraph is absent and its producer did not execute."))
    reusable = await p("Resolved-child reuse: immutable paragraph used twice.").resolve()
    blocks.extend([reusable, reusable])
    inline = await d.TextRun("Resolved-inline reuse: immutable run used twice.").resolve()
    blocks.append(d.Paragraph(children=[inline, inline]))
    blocks.append(p("Header/footer distances: 720 and 360 twips from the page edges."))
    headers = {name: d.Header([d.Paragraph(children=[d.TextRun(f"Header {name}: "), link("owner-local external hyperlink"), image("png", {"width": 12})])])
               for name in ["default", "first", "even"]}
    footers = {name: d.Footer([d.Paragraph(children=[d.TextRun(f"Footer {name}: PAGE "), d.Field(d.FieldInstruction.page(), [d.TextRun("1")]),
        d.TextRun(" / NUMPAGES "), d.Field(d.FieldInstruction.num_pages(), [d.TextRun("1")])])]) for name in ["default", "first", "even"]}
    blocks.append(p("Page borders: navy single top/bottom and gray dashed left/right; first page, offset from page, behind text."))
    horizontal = d.PageBorder("single", 12, 6, (32, 48, 96))
    vertical = d.PageBorder("dashed", 8, 4, (96, 96, 96))
    blocks.append(p("Document page settings: mirrored margins enabled, top gutter explicitly off."))
    first = d.Section(blocks, properties=d.SectionProperties(page_size={"width": 7772400, "height": 10058400},
        page_orientation="portrait", page_margins={"top": 914400, "bottom": 914400, "left": 914400, "right": 914400, "header": 457200, "footer": 228600},
        page_borders=d.PageBorders(horizontal, vertical, horizontal, vertical, offset_from="page", display="firstPage", z_order="back"),
        title_page=True, bidi=False, rtl_gutter=False), headers=headers, footers=footers)
    sections = [first]
    for i, kind in enumerate(["nextPage", "nextColumn", "continuous", "evenPage", "oddPage"]):
        valign = ["top", "center", "bottom", "both", "top"][i]
        flag = str(i % 2 == 1).lower()
        restart = ["newPage", "newSection", "continuous", None, None][i]
        line_numbering_label = (f"Line numbering: every 5 lines, start=0, distance=240 twips, restart={restart}."
                                if restart else
                                "Line numbering: clear previous section settings." if i == 4 else
                                "Line numbering: preserve previous section settings.")
        grid_mode = ["default", "lines", "linesAndChars", "snapToChars", None][i]
        grid_label = (f"Document grid: mode={grid_mode}, line pitch=360 twips, character pitch delta=-4096."
                      if grid_mode else "Document grid: preserve previous section settings.")
        sections.append(d.Section([
            p(f"09 Section: break={kind}; landscape; 2 columns with separator; page numbering lowerRoman starts at {i+1}; vertical alignment={valign}; section bidi={flag}; rtl_gutter={flag}."),
            p(line_numbering_label),
            d.Paragraph(text="Paragraph layout: contextual spacing enabled, mirrored indents explicitly off.",
                        contextual_spacing=True, mirror_indents=False),
            p(grid_label),
            d.Paragraph(children=[d.TextRun("Grid snapping: paragraph enabled, run explicitly off.", snap_to_grid=False)],
                        snap_to_grid=True),
            d.Paragraph(text="Paragraph line numbers: suppressed.", suppress_line_numbers=True),
            d.Paragraph(text="Paragraph line numbers: suppression explicitly off.", suppress_line_numbers=False),
            p("Section layout request: physical margins remain unchanged. Gutter placement does not create a gutter amount. This is the first column."),
            d.Paragraph(children=[d.Break("column"), d.TextRun("This follows a column break in the section sample.")])],
            properties=d.SectionProperties(type=kind, page_orientation="landscape", columns={"count": 2, "space": 360, "separator": True},
                page_numbering={"start": i+1, "format": "lowerRoman"},
                document_grid=d.DocumentGrid(grid_mode, 360, -4096) if grid_mode else None,
                line_numbering=(d.LineNumbering(5, 0, 240, restart) if restart else
                                d.LineNumbering(clear=True) if i == 4 else None), vertical_align=valign, title_page=False, bidi=i % 2 == 1, rtl_gutter=i % 2 == 1)))
    notes = [d.Note.footnote(0, [p("Footnote body: rich paragraph, table, image and external hyperlink."),
        d.Table([d.TableRow([cell("Footnote table")])]), d.Paragraph(children=[image("png", {"width": 18}), link("Footnote external link")])]),
        d.Note.endnote(0, [p("Endnote body: this is an endnote, distinct from footnote id=0.")])]
    bullet_levels = [d.NumberingLevel(0, 1, "bullet", "•")]
    numbering = [d.NumberingDefinition(60, [d.NumberingLevel(level, 1, "lowerRoman" if level == 1 else "decimal", f"%{level+1}.", is_legal=True if level == 2 else None, suffix="tab" if level == 2 else None, restart=0 if level == 2 else None, alignment=["left", "center", "right", "start", "end"][level] if level < 5 else None, label_format=d.RunDefaults(bold=True) if level == 0 else None, paragraph_format=d.ParagraphDefaults(left_indent=24, first_line_indent=-12) if level == 0 else None, paragraph_style="Heading 1" if level == 0 else None) for level in range(9)],
        start_overrides=[d.NumberingStartOverride(0, 7), d.NumberingStartOverride(8, 0)],
        level_overrides=[d.NumberingLevel(1, 4, "upperRoman", "(%2)", is_legal=True, suffix="nothing", restart=1, alignment="start"), d.NumberingLevel(0, 3, "lowerLetter", "%1)", is_legal=False, suffix="space", restart=0, alignment="end", label_format=d.RunDefaults(bold=False, italic=True), paragraph_format=d.ParagraphDefaults(left_indent=36), paragraph_style="Heading 2")]),
        d.NumberingDefinition(61, bullet_levels, abstract_id=100),
        d.NumberingDefinition(62, bullet_levels, start_overrides=[d.NumberingStartOverride(0, 4)], abstract_id=100),
        d.NumberingDefinition(63, abstract_id=101, inherit_style="Catalog Bullets")]
    document = d.Document(sections, timestamp=STAMP, creator="DOCX inspection catalog", title="All features visual inspection",
        subject="Rust/Python/JavaScript byte parity", description="Labeled functionality samples; not a rendering guarantee", keywords="docx,visual,parity",
        category="Examples", comments="Core metadata comments", version="1.0", comment_author="Catalog reviewer",
        custom_properties={"Review status": "Draft & review", "Review complete": True, "Published": False},
        company="Atlas & Co.", manager="Taylor",
        defaults=d.DocumentDefaults(run=d.RunDefaults(font="Calibri", size=11),
            paragraph=d.ParagraphDefaults(spacing_after=4, line_height=d.LineHeight.multiple(1.1)),
            mirror_margins=True, gutter_at_top=False, track_revisions=False,
            footnote_position="beneathText", endnote_position="sectEnd",
            footnote_numbering=d.NoteNumbering(3, "eachPage", "lowerRoman"),
            endnote_numbering=d.NoteNumbering(7, "eachSect", "upperLetter")),
        styles=[*styles(), d.NumberingStyleDefinition("Catalog Bullets", 61)], numbering=numbering, notes=notes)
    snapshot = await document.resolve()
    options = rocx.RenderOptions(timestamp=STAMP, profile="transitional",
        read_limits=rocx.ReadLimits(max_parts=10000, max_part_bytes=67108864, max_expanded_bytes=268435456))
    template = d.DocumentTemplate(template_path, replacements={"{{replace}}": "REPLACED"}, paragraph_index=1,
        scope="supported_stories", content_control_replacements={"template-control": "updated controlled value"},
        text_revision_action="accept")
    data = await snapshot.to_bytes(template, options=options)
    assert data == await snapshot.to_bytes(template, options=options)
    await snapshot.save(output, template, options=options)
    imported = ImportedDocument.open(data)
    assert any(s.name == "Catalog Bullets" and s.kind == "numbering" and s.id
               for s in imported.styles.styles)
    revisions = [node for block in imported.body for node in getattr(block, "inline_structure", [])
                 if getattr(node, "author", None) == "Catalog editor"]
    for kind, text in [("insertion", "inserted wording"), ("deletion", "old wording")]:
        revision, = (r for r in revisions if r.kind == kind and r.children[0].items[0].value == text)
        assert imported.inline_at(revision.children[0].anchor).items[0].value == text
    controls = [block for block in imported.body if getattr(block, "tag", None) == "review-block"]
    assert len(controls) == 1
    assert (controls[0].alias, controls[0].lock) == ("Review block", "sdtLocked")
    entries = imported.entry_stories
    assert {e.kind for e in entries if e.id == "0"} >= {"footnote", "endnote"}
    review, = (e for e in entries if e.author == "Structured reviewer")
    assert review.kind == "comment" and review.initials == "SR"
    review_run = review.blocks[0].inline_structure[0]
    assert imported.inline_at(review_run.anchor).items[0].value.startswith("Structured comment:")
    inline_block, = (block for block in imported.body
                     if getattr(block, "text", None) == INLINE_LABEL + "".join(INLINE_LINK_RUNS))
    label, inline_link = inline_block.inline_structure
    replacement = "Imported editor: replaced this run by its anchor. "
    inserted_paragraph = await d.Paragraph(text="Imported editor: inserted paragraph.").resolve()
    preview = imported.preview([], run_text=[RunTextEdit(label.anchor, replacement)],
                               paragraph_insertions=[ParagraphInsertion(inline_block.anchor, inserted_paragraph)])
    assert preview.paragraph_insertions == [
        next(i for i, block in enumerate(imported.body)
             if getattr(block, "text", None) == INLINE_LABEL + "".join(INLINE_LINK_RUNS))
    ]
    assert preview.run_text_changes[0].before == INLINE_LABEL
    removal = imported.preview([], run_removals=[label.anchor])
    assert removal.run_removals[0].before == INLINE_LABEL
    output.with_suffix(".run-text.docx").write_bytes(preview.to_docx())
    assert isinstance(label, ImportedRun) and isinstance(inline_link, ImportedHyperlink)
    assert [(item.kind, item.value) for item in label.items] == [("text", INLINE_LABEL)]
    assert inline_link.bookmark == "CatalogBookmark" and inline_link.relationship_id is None
    assert imported.inline_at(inline_link.anchor).anchor == inline_link.anchor
    for child, text in zip(inline_link.children, INLINE_LINK_RUNS, strict=True):
        assert isinstance(child, ImportedRun)
        assert [(item.kind, item.value) for item in child.items] == [("text", text)]
        assert imported.inline_at(child.anchor).anchor == child.anchor
    targets = [block for block in imported.body if getattr(block, "text", None) == EDIT_LABEL]
    assert len(targets) == 1
    assert [(item.kind, item.value) for item in targets[0].inline_items] == [
        ("text", EDIT_LABEL),
    ]
    table = d.Table([d.TableRow([
        d.TableCell([run("Imported editor table: bold label", bold=True)]),
        d.TableCell([d.Paragraph(children=[d.TextRun("Fixed-width "), d.TextRun("second column")])]),
    ])], column_widths=[2160, 2160], layout="fixed")
    resolved_table = await table.resolve()
    insertion = TableInsertion(targets[0].anchor, resolved_table)
    preview = imported.preview([ParagraphAlignmentEdit(targets[0].anchor, "center")],
                               table_insertions=[insertion])
    edited = preview.to_docx()
    # Typed physical-table readback from the edited original package.
    reopened = ImportedDocument.open(edited)
    table_index = preview.insertions[0].after_body_index + 1
    inserted = reopened.body[table_index]
    first, second = inserted.children[0].children
    first_block = first.blocks[0]
    assert [(item.kind, item.value) for item in first_block.inline_items] == [
        ("text", "Imported editor table: bold label"),
    ]
    first_run, = first_block.inline_structure
    prefix, suffix = second.blocks[0].inline_structure
    assert all(isinstance(node, ImportedRun) for node in (first_run, prefix, suffix))
    assert [[item.value for item in node.items] for node in (prefix, suffix)] == [
        ["Fixed-width "], ["second column"],
    ]
    assert first_run.anchor != prefix.anchor
    assert reopened.inline_at(suffix.anchor).anchor == suffix.anchor
    reopened_again = ImportedDocument.open(edited)
    other_cell = reopened_again.body[table_index].children[0].children[1]
    assert other_cell.blocks[0].inline_structure[1].anchor != suffix.anchor
    cell_preview = reopened.preview([], cell_alignments=[CellAlignmentEdit(inserted.anchor, 0, 0, "center")])
    assert len(cell_preview.cell_alignment_changes) == 1
    output.with_suffix(".edited.docx").write_bytes(cell_preview.to_docx())

    # Inspect inherited header/footer slots before editing the default header.
    header_paragraph = d.Paragraph(children=[
        d.TextRun(HEADER_INLINE_LABEL),
        d.Hyperlink("https://example.invalid/catalog-header",
                    [d.TextRun(text) for text in HEADER_LINK_RUNS]),
    ])
    variants = ("default", "first", "even")
    headers = {"default": d.Header([header_paragraph])}
    headers.update({variant: d.Header([p(f"Story inspection: {variant} header")])
                    for variant in variants[1:]})
    footers = {variant: d.Footer([p(f"Story inspection: {variant} footer")])
               for variant in variants}
    source = d.Document([
        d.Section([p(HEADER_BATCH_LABEL)],
                  headers=headers, footers=footers),
        d.Section([p("Header image scope: second section shares the same header.")]),
    ], timestamp=STAMP)
    source_bytes = await (await source.resolve()).to_bytes()
    output.with_suffix(".header-source.docx").write_bytes(source_bytes)
    header_doc = ImportedDocument.open(source_bytes)
    assert [(s.body_start, s.body_end) for s in header_doc.sections] == [(0, 2), (2, 3)]
    # Tracked replacement and paired move are independent opt-in previews.
    start, after = (header_doc.body[i].inline_structure[0].anchor for i in (0, 2))
    tracked = header_doc.preview_tracked_text(start, "replace", "Reviewer", text="Reviewed label", date=STAMP)
    assert ImportedDocument.open(tracked.to_docx()).body[0].text == ""
    moved = header_doc.preview_tracked_move(start, after, "CatalogMove", "Reviewer", STAMP)
    pair = ImportedDocument.open(moved.to_docx())
    restored = pair.preview_revision(pair.body[0].inline_structure[1].anchor, "reject",
                                     paired=pair.body[2].inline_structure[2].anchor)
    assert ImportedDocument.open(restored.to_docx()).body[0].text == HEADER_BATCH_LABEL
    first, target = header_doc.default_headers
    assert target.section_index == 1 and target.inherited and first.part_name == target.part_name
    empty, paragraph = target.blocks
    assert not empty.inline_structure
    assert [(item.kind, item.value) for item in paragraph.inline_items] == [
        ("text", text) for text in (HEADER_INLINE_LABEL, *HEADER_LINK_RUNS)
    ]
    label, header_link = paragraph.inline_structure
    assert isinstance(label, ImportedRun) and isinstance(header_link, ImportedHyperlink)
    assert header_link.relationship_id is not None
    assert header_link.anchor == first.blocks[1].inline_structure[1].anchor
    assert header_doc.inline_at(label.anchor).anchor == label.anchor
    assert header_doc.inline_at(header_link.anchor).anchor == header_link.anchor
    assert all(isinstance(run, ImportedRun) for run in header_link.children)
    assert [[item.value for item in header_doc.inline_at(run.anchor).items]
            for run in header_link.children] == [[text] for text in HEADER_LINK_RUNS]
    stories = header_doc.stories
    assert [(s.section_index, s.kind, s.variant, s.inherited) for s in stories] == [
        (index, kind, variant, bool(index))
        for index in range(2) for kind in ("header", "footer") for variant in variants
    ]
    for explicit, inherited in zip(stories[:6], stories[6:], strict=True):
        assert explicit.part_name == inherited.part_name
        first_run = explicit.blocks[1].inline_structure[0]
        second_run = inherited.blocks[1].inline_structure[0]
        assert first_run.anchor == second_run.anchor == header_doc.inline_at(second_run.anchor).anchor
    assert stories[0].blocks[1].inline_structure[0].anchor == label.anchor
    resolved_image = await image("png").resolve()
    first_page = next(s for s in stories if (s.section_index, s.kind, s.variant) == (1, "header", "first"))
    for mode, suffix in (("shared", ".header-first-shared.docx"),
                         ("section_local", ".header-first-local.docx")):
        first_preview = header_doc.preview_header_image(HeaderImageEdit(first_page.anchor, resolved_image, mode=mode))
        assert first_preview.change.affected_sections == ([0, 1] if mode == "shared" else [1])
        output.with_suffix(suffix).write_bytes(first_preview.to_docx())
    body_target = header_doc.body[0]
    assert body_target.text == HEADER_BATCH_LABEL
    for mode, suffix in [("shared", ".header-shared.docx"),
                         ("section_local", ".header-local.docx")]:
        edit = HeaderImageEdit(target.anchor, resolved_image, mode=mode)
        preview = header_doc.preview(
            [ParagraphAlignmentEdit(body_target.anchor, "center")],
            table_insertions=[TableInsertion(body_target.anchor, resolved_table)],
            header_images=[edit],
        )
        assert preview.header_changes[0].affected_sections == ([0, 1] if mode == "shared" else [1])
        output.with_suffix(suffix).write_bytes(preview.to_docx())
    return data


if __name__ == "__main__":
    asyncio.run(generate(pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "examples/all_features-python.docx")))
