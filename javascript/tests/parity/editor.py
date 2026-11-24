"""Generate and independently verify shared inputs/outputs of the packed editor tests."""

import copy
import io
import json
import posixpath
import struct
import sys
import zipfile
import zlib
from pathlib import Path

from docx import Document
from docx.enum.section import WD_HEADER_FOOTER
from docx.enum.style import WD_STYLE_TYPE
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Twips
from lxml import etree

MAIN = "word/document.xml"
MAIN_RELS = "word/_rels/document.xml.rels"
TYPES = "[Content_Types].xml"
HEADER = "word/header1.xml"
HEADER_RELS = "word/_rels/header1.xml.rels"
HEADER_LINK = "https://example.invalid/header"
BODY_LINK = "https://example.invalid/inserted"
NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "m": "http://schemas.openxmlformats.org/officeDocument/2006/math",
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
}


def xp(node, query):
    return etree.XPath(query, namespaces=NS)(node)


def elem(tag, **attrs):
    node = OxmlElement("w:" + tag)
    for key, value in attrs.items():
        node.set(qn("w:" + key), value)
    return node


def parts(file):
    with zipfile.ZipFile(file) as archive:
        assert archive.testzip() is None
        return {name: archive.read(name) for name in archive.namelist()}


def save_parts(file, entries):
    with zipfile.ZipFile(file, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in entries.items():
            archive.writestr(name, data)


def png(rgb):
    """Distinct, valid one-pixel PNGs without an image processing dependency."""
    def chunk(kind, payload):
        return (struct.pack(">I", len(payload)) + kind + payload
                + struct.pack(">I", zlib.crc32(kind + payload)))

    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"\0" + bytes(rgb)))
            + chunk(b"IEND", b""))


def generate(directory):
    catalog = Document()
    catalog.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    for name in ["Memo Replace", "Memo Drop", "Memo Active"]:
        catalog.styles.add_style(name, WD_STYLE_TYPE.PARAGRAPH).font.italic = False
    catalog.sections[0].header.paragraphs[0].style = "Memo Active"
    catalog.save(directory / "styles-source.docx")
    doc = Document()
    # python-docx's bundled template omits this required Transitional attribute.
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    first = doc.add_paragraph()
    first.paragraph_format.keep_with_next = True
    first._p.get_or_add_pPr().append(elem("jc", val="start"))
    first.add_run("Source bold").bold = True
    field = elem("fldSimple", instr=" PAGE ")
    field.append(first.add_run("page cache")._r)
    first._p.append(field)
    doc.add_paragraph("unset alignment").paragraph_format.keep_together = True
    original_image = png((10, 20, 30))
    doc.add_picture(io.BytesIO(original_image), width=Twips(500))

    header = doc.sections[0].header
    p = header.paragraphs[0]
    p.add_run("Shared header ").italic = True
    link = elem("hyperlink")
    link.set(qn("r:id"), p.part.relate_to(HEADER_LINK, RELATIONSHIP_TYPE.HYPERLINK, is_external=True))
    link.append(p.add_run("kept link")._r)
    p._p.append(link)
    doc.add_section()
    doc.add_paragraph("next section").alignment = WD_ALIGN_PARAGRAPH.RIGHT
    doc.add_section()
    doc.add_section()
    sections = doc.sections
    sections[3].header.is_linked_to_previous = False
    sections[3].header.paragraphs[0].text = "Distinct default"
    alias = sections[0]._sectPr.headerReference_lst[0].rId
    sections[3]._sectPr.add_headerReference(WD_HEADER_FOOTER.FIRST_PAGE, alias)
    sections[3]._sectPr.add_headerReference(WD_HEADER_FOOTER.EVEN_PAGE, alias)
    sections[3].different_first_page_header_footer = True
    doc.settings.odd_and_even_pages_header_footer = True
    for i, section in enumerate(sections):
        section.page_width = Twips(10000 if i == 0 else 14000)
        section.left_margin = section.right_margin = Twips(1000 if i == 0 else 2000)

    doc.save(directory / "base.docx")
    (directory / "image.png").write_bytes(png((100, 20, 30)))
    ranged = Document(directory / "base.docx")
    p = ranged.paragraphs[1]
    p.clear()
    p.add_run("a😀b").bold = True
    p.add_run("e\u0301").underline = True
    p.add_run("cd").italic = True
    ranged.save(directory / "range-source.docx")
    base = parts(directory / "base.docx")
    assert base["word/media/image1.png"] == original_image
    assert HEADER in base and HEADER_RELS in base

    edge = base.copy()
    edge["word/styles.xml"] = (
        b'<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        b'<w:style w:type="future" w:styleId="X"/><w:style/></w:styles>')
    save_parts(directory / "styles-edge.docx", edge)
    missing = base.copy()
    rels = etree.fromstring(missing[MAIN_RELS])
    for rel in list(rels):
        if rel.get("Type").endswith("/styles"):
            rels.remove(rel)
    missing[MAIN_RELS] = etree.tostring(rels)
    save_parts(directory / "styles-missing.docx", missing)
    invalid = base.copy()
    invalid["word/styles.xml"] = b'<w:invalid xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>'
    save_parts(directory / "styles-invalid.docx", invalid)

    def body_variant(name, mutate):
        main = etree.fromstring(base[MAIN])
        mutate(xp(main, "./w:body/w:p"))
        save_parts(directory / f"{name}.docx", {**base, MAIN: etree.tostring(main)})

    def crossing(p):
        p[0].insert(1, elem("bookmarkStart", id="7", name="crossing"))
        p[1].append(elem("bookmarkEnd", id="7"))

    body_variant("unsafe", crossing)
    body_variant("ambiguous", lambda p: xp(p[0], "./w:pPr")[0].append(elem("jc", val="left")))

    drawing = etree.fromstring(base[HEADER])
    xp(drawing, "./w:p/w:r")[0].append(elem("drawing"))
    save_parts(directory / "drawing.docx", {**base, HEADER: etree.tostring(drawing)})

    relocated = base.copy()
    new = "content/headers/source.xml"
    relocated[new] = relocated.pop(HEADER)
    relocated[rel_name(new)] = relocated.pop(HEADER_RELS)
    rels = etree.fromstring(relocated[MAIN_RELS])
    header_rel, = [rel for rel in rels if rel.get("Target") == "header1.xml"]
    header_rel.set("Target", "../" + new)
    duplicate = copy.deepcopy(header_rel)
    duplicate.set("Id", "rIdSamePhysicalHeader")
    rels.append(duplicate)
    relocated[MAIN_RELS] = etree.tostring(rels)
    main = etree.fromstring(base[MAIN])
    third = xp(main, "./w:body/w:p/w:pPr/w:sectPr")[2]
    alternate_ref = elem("headerReference", type="default")
    alternate_ref.set(qn("r:id"), duplicate.get("Id"))
    third.insert(0, alternate_ref)
    last = xp(main, "./w:body/w:sectPr")[0]
    first_page, = xp(last, "./w:headerReference[@w:type='first']")
    last.remove(first_page)  # Isolate the even-page consumer in this variant.
    relocated[MAIN] = etree.tostring(main)
    relocated[TYPES] = relocated[TYPES].replace(b"/word/header1.xml", b"/content/headers/source.xml")
    save_parts(directory / "relocated.docx", relocated)


def rel_name(part):
    folder, name = posixpath.split(part.lstrip("/"))
    return posixpath.join(folder, "_rels", name + ".rels")


def relations(package, owner):
    return {r.get("Id"): dict(r.attrib) for r in etree.fromstring(package[rel_name(owner)])}


def untouched(before, after, allowed=()):
    for name, payload in before.items():
        if name not in allowed:
            assert after.get(name) == payload, f"changed or missing untouched part {name}"


def tree(node, excluded=None):
    """Compare expanded names, attributes, children and text, not namespace aliases."""
    return (node.tag, dict(node.attrib), node.text,
            [(tree(child, excluded), child.tail) for child in node if child.tag != excluded])


def inspect_body(before, after):
    original = xp(etree.fromstring(before[MAIN]), "./w:body/*")
    body = xp(etree.fromstring(after[MAIN]), "./w:body/*")
    assert tree(original[0], qn("w:jc")) == tree(body[0], qn("w:jc"))
    assert tree(original[1], qn("w:jc")) == tree(body[2], qn("w:jc"))
    assert tree(original[2]) == tree(body[3])  # existing body drawing and media reference
    assert xp(body[0], "w:pPr/w:jc/@w:val") == ["center"]
    assert xp(body[2], "w:pPr/w:jc/@w:val") == ["right"]
    rich, auto = body[1], body[5]
    assert xp(rich, "w:tblGrid/w:gridCol/@w:w") == ["1200", "2400", "3600"]
    assert xp(rich, "w:tr/w:tc[1]/w:tcPr/w:gridSpan/@w:val") == ["2"]
    assert xp(rich, "w:tr/w:tc/w:tbl/w:tblGrid/w:gridCol/@w:w") == ["1800", "1800"]
    assert xp(rich, ".//w:r[w:rPr/w:b]/w:t/text()") == ["bold"]
    assert xp(rich, ".//m:f//m:t/text()") == ["1", "2"]
    assert xp(body[4], "w:pPr/w:sectPr")
    assert xp(auto, "w:tblGrid/w:gridCol/@w:w") == ["5000", "5000"]
    link, = xp(rich, ".//w:hyperlink/@r:id")
    rels = relations(after, MAIN)
    assert rels[link]["Target"] == BODY_LINK and rels[link]["TargetMode"] == "External"
    assert rels[link]["Type"].endswith("/hyperlink")
    for key, original in relations(before, MAIN).items():
        assert rels[key] == original


def inspect_header(before, after, change, image):
    part = change["outputPart"].lstrip("/")
    source = change["sourcePart"].lstrip("/")
    header = etree.fromstring(after[part])
    old = etree.fromstring(before[source])
    assert tree(old[0]) == tree(header[0])
    image_id, = xp(header, ".//a:blip/@r:embed")
    rels = relations(after, part)
    picture = rels[image_id]
    assert picture["Type"].endswith("/image")
    target = picture["Target"]
    media_part = (target.lstrip("/") if target.startswith("/")
                  else posixpath.normpath(posixpath.join(posixpath.dirname(part), target)))
    assert after[media_part] == image
    for key, link in relations(before, source).items():
        assert rels[key] == link
    link_id, = xp(header, ".//w:hyperlink/@r:id")
    assert rels[link_id]["Target"] == HEADER_LINK and rels[link_id]["TargetMode"] == "External"
    return int(xp(header, ".//wp:docPr/@id")[0])


def verify(directory):
    from rocx import declarative as d
    from rocx.editor import ImportedDocument, StyleEdit
    source_path = directory / "styles-source.docx"
    style_path = directory / "styleEdits.docx"
    styles_source = ImportedDocument.open(source_path.read_bytes())
    edit = styles_source.preview([], style_edits=[
        StyleEdit.add(d.ParagraphStyleDefinition(name="Memo Added", base_style="Memo Active")),
        StyleEdit.replace(styles_source.style_anchor("MemoReplace"),
                          d.ParagraphStyleDefinition(name="Memo Replace", bold=True)),
        StyleEdit.remove(styles_source.style_anchor("MemoDrop")),
        StyleEdit.patch(styles_source.style_anchor("MemoActive"), italic=True),
    ])
    assert edit.to_docx() == style_path.read_bytes()
    before, after = parts(source_path), parts(style_path)
    assert before.keys() == after.keys()
    untouched(before, after, ("word/styles.xml",))
    readback = Document(style_path)
    assert "Memo Drop" not in readback.styles
    assert readback.styles["Memo Added"].base_style.name == "Memo Active"
    assert readback.styles["Memo Replace"].font.bold and readback.styles["Memo Replace"].font.italic is None
    assert readback.sections[0].header.paragraphs[0].style.font.italic
    original_ids = [s.style_id for s in Document(source_path).styles if s.name != "Memo Drop"]
    assert [s.style_id for s in readback.styles] == original_ids + ["MemoAdded"]
    source = parts(directory / "base.docx")
    image = (directory / "image.png").read_bytes()
    reports = json.loads((directory / "reports.json").read_text(encoding="utf8"))
    assert parts(directory / "noop.docx") == source
    removal = parts(directory / "removal.docx")
    untouched(source, removal, (MAIN,))
    assert Document(directory / "removal.docx").paragraphs[0].text == ""
    root = etree.fromstring(removal[MAIN])
    assert root.find(".//" + qn("w:fldSimple")) is not None
    from rocx.editor import ImportedDocument, TextRangeEdit
    range_source = parts(directory / "range-source.docx")
    result = parts(directory / "range.docx")
    assert result.keys() == range_source.keys()
    untouched(range_source, result, (MAIN,))
    # Preserve the opaque simple field in the neighboring paragraph.
    assert tree(xp(etree.fromstring(range_source[MAIN]), "//w:fldSimple")[0]) == tree(xp(etree.fromstring(result[MAIN]), "//w:fldSimple")[0])
    runs = Document(directory / "range.docx").paragraphs[1].runs
    assert [r.text for r in runs] == ["a 🐈 ", "", "d"]
    assert runs[0].bold and runs[1].underline and runs[2].italic
    ranged = ImportedDocument.open((directory / "range-source.docx").read_bytes())
    selection = TextRangeEdit(ranged.body[1].anchor, 1, 6, " 🐈 ")
    assert ranged.preview([], text_ranges=[selection]).to_docx() == (directory / "range.docx").read_bytes()
    imported = ImportedDocument.open((directory / "base.docx").read_bytes())
    anchor = imported.body[0].inline_structure[0].anchor
    python_preview = imported.preview([], run_removals=[anchor])
    assert python_preview.run_removals[0].before == "Source bold"
    assert python_preview.to_docx() == (directory / "removal.docx").read_bytes()

    import asyncio
    from rocx import declarative as d
    from rocx.editor import InlineEdit

    async def inline_batch():
        target = lambda i: imported.body[i].inline_structure[0].anchor
        link = await d.Hyperlink("https://example.invalid/inline", [d.TextRun("editable link")]).resolve()
        return imported.preview([], inline_edits=[InlineEdit.insert_after(target(4), await d.Break().resolve()),
            InlineEdit.set_bold(target(0), False), InlineEdit.replace(target(1), link)])

    inline = asyncio.run(inline_batch())
    assert inline.inline_changes == [(0, 0), (1, 0), (4, 0)]
    assert inline.to_docx() == (directory / "inline.docx").read_bytes()
    untouched(source, parts(directory / "inline.docx"), (MAIN, MAIN_RELS))
    edited = Document(directory / "inline.docx").paragraphs
    assert edited[0].runs[0].bold is False
    assert edited[1].hyperlinks[0].url == "https://example.invalid/inline"
    assert edited[1].text == "editable link" and edited[4].text == "next section\n"

    body = parts(directory / "body.docx")
    untouched(source, body, (MAIN, MAIN_RELS))
    inspect_body(source, body)
    readback = Document(directory / "body.docx")
    assert readback.paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert readback.tables[0].cell(0, 0).paragraphs[0].runs[0].bold
    assert readback.tables[0].cell(0, 0).tables[0].cell(0, 0).text == "nested A"
    assert len(readback.inline_shapes) == 1

    cell_output = parts(directory / "cell.docx")
    untouched(body, cell_output, (MAIN,))
    from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
    cell_readback = Document(directory / "cell.docx")
    assert cell_readback.tables[1].cell(0, 0).vertical_alignment == WD_CELL_VERTICAL_ALIGNMENT.CENTER
    assert cell_readback.tables[1].cell(0, 0).text == "next A"

    structure = directory / "tableStructure.docx"
    untouched(body, parts(structure), (MAIN,))
    row = Document(structure).tables[1].rows[0]
    assert [cell.text for cell in row.cells] == ["next A", "inserted paragraph\n"]
    assert [c.text for c in row.cells[1].tables[0].rows[0].cells] == ["next A", "next B"]
    assert row.cells[1]._tc[-1].tag == qn("w:p")
    assert row.grid_cols_after == 1

    mixed_body = directory / "mixedBody.docx"
    blocks = parts(directory / "blocks.docx")
    untouched(source, blocks, (MAIN,))
    assert blocks.keys() == source.keys()
    before = Document(directory / "base.docx")
    after = Document(directory / "blocks.docx")
    assert after.paragraphs[0].paragraph_format.keep_with_next is False
    assert after.paragraphs[1].text == after.paragraphs[4].text == "inserted paragraph"
    assert [c.text for c in after.tables[0].rows[0].cells] == ["next A", "next B"]
    assert xp(etree.fromstring(blocks[MAIN]), ".//w:tblGrid/w:gridCol/@w:w") == ["5000", "5000"]
    assert len(before.paragraphs) == len(after.paragraphs)
    old, new = etree.fromstring(source[MAIN]), etree.fromstring(blocks[MAIN])
    assert [tree(n) for n in xp(old, ".//w:sectPr")] == [tree(n) for n in xp(new, ".//w:sectPr")]
    assert [tree(n) for n in xp(old, ".//w:fldSimple | .//w:drawing")] == [
        tree(n) for n in xp(new, ".//w:fldSimple | .//w:drawing")]
    assert [s.header.is_linked_to_previous for s in before.sections] == [
        s.header.is_linked_to_previous for s in after.sections]
    untouched(body, parts(mixed_body), (MAIN,))
    combined = Document(mixed_body)
    assert [p.text for p in combined.paragraphs[:3]] == [readback.paragraphs[0].text, "inserted paragraph", ""]
    cell = combined.tables[1].cell(0, 0)
    assert (cell.text, cell.vertical_alignment) == ("next A", WD_CELL_VERTICAL_ALIGNMENT.CENTER)

    shared = parts(directory / "shared.docx")
    untouched(source, shared, (HEADER, HEADER_RELS, TYPES))
    inspect_header(source, shared, reports["shared"], image)

    local = parts(directory / "local.docx")
    untouched(source, local, (MAIN, MAIN_RELS, TYPES))
    inspect_header(source, local, reports["local"], image)
    local_readback = Document(directory / "local.docx")
    assert local_readback.sections[1].header.paragraphs[0].text == "Shared header kept link"
    assert len(local_readback.sections[1].header.paragraphs) == 2
    assert len(local_readback.sections[2].header.paragraphs) == 1

    first_shared = Document(directory / "firstShared.docx")
    first_local = Document(directory / "firstLocal.docx")
    assert len(first_shared.sections[3].first_page_header._element.xpath(".//w:drawing")) == 1
    assert len(first_local.sections[3].first_page_header._element.xpath(".//w:drawing")) == 1
    assert len(first_local.sections[0].header._element.xpath(".//w:drawing")) == 0
    assert parts(directory / "firstLocal.docx")[HEADER] == source[HEADER]

    relocated_source = parts(directory / "relocated.docx")
    relocated = parts(directory / "relocated-out.docx")
    untouched(relocated_source, relocated, (MAIN, MAIN_RELS, TYPES))
    inspect_header(relocated_source, relocated, reports["relocated"], image)

    mixed = parts(directory / "mixed.docx")
    untouched(source, mixed, (MAIN, MAIN_RELS, TYPES))
    inspect_body(source, mixed)
    first, second = [inspect_header(source, mixed, edit, image) for edit in reports["mixed"]]
    assert first < second
    mixed_shared = parts(directory / "mixedShared.docx")
    untouched(source, mixed_shared, (MAIN, MAIN_RELS, HEADER, HEADER_RELS, TYPES))
    inspect_body(source, mixed_shared)
    inspect_header(source, mixed_shared, reports["shared"], image)
    print("Independent python-docx/XML/ZIP: edit placement, relationship ownership and exact untouched payloads")


if __name__ == "__main__":
    mode, path = sys.argv[1:]
    {"generate": generate, "verify": verify}[mode](Path(path))
