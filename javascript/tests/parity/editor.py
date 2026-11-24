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


def generate_controls(directory):
    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    def wrap(node, *extra):
        parent = node.getparent()
        pos = parent.index(node)
        control, props, cached = elem("sdt"), elem("sdtPr"), elem("sdtContent")
        # Repeated tags intentionally identify several different controls.
        props.extend([elem("tag", val="repeat"), *extra, elem("richText")])
        cached.append(node)
        control.extend([props, cached])
        parent.insert(pos, control)
        return props

    wrap(doc.add_paragraph("block")._p, elem("lock", val="sdtLocked"))
    run = doc.add_paragraph().add_run("inline")
    run.bold = True
    props = wrap(run._r, elem("lock", val="contentLocked"))
    # Use an extension in a declared ignorable namespace; preserve it verbatim
    # as an XML subtree even when inserting a new alias before the existing tag.
    etree.SubElement(props, "{http://schemas.microsoft.com/office/word/2010/wordml}probe").text = "opaque"
    for tag in ("row", "cell"):
        t = doc.add_table(rows=1, cols=1)
        t.cell(0, 0).text = tag
        wrap(t.rows[0]._tr if tag == "row" else t.cell(0, 0)._tc)
    wrap(doc.add_paragraph("bound")._p,
         elem("dataBinding", xpath="/root", storeItemID="{00000000-0000-0000-0000-000000000001}"))
    unsafe = doc.add_paragraph("first")
    unsafe.add_run("second")
    wrap(unsafe._p)
    doc.add_paragraph("untouched")
    doc.save(directory / "control-source.docx")

    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    for context in ("block", "inline"):
        doc.add_paragraph().add_run(context).bold = True
    for context in ("row", "cell"):
        doc.add_table(rows=1, cols=1).cell(0, 0).text = context
    props = wrap(doc.add_paragraph().add_run("First")._r)
    props.remove(props[-1])
    choices = elem("dropDownList", lastValue="1")
    choices.extend([elem("listItem", displayText=label, value=value)
                    for label, value in [("First", "1"), ("Second", "2")]])
    props.append(choices)
    doc.save(directory / "control-choice-source.docx")

    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.add_paragraph("ordinary")
    p = doc.add_paragraph("range ")
    p.add_run("start").bold = True
    doc.add_table(rows=1, cols=1).cell(0, 0).text = "retained table"
    doc.add_paragraph("range end")
    p = doc.add_paragraph("one")
    p.add_run("two").italic = True
    p.add_run("outside")
    wrap(doc.add_paragraph("block payload")._p, elem("lock", val="sdtLocked"))
    wrap(doc.add_paragraph().add_run("inline payload")._r)
    wrap(doc.add_paragraph("locked")._p, elem("lock", val="contentLocked"))
    for name in ("header","footer"):
        p = getattr(doc.sections[0], name).paragraphs[0]
        wrap(p.add_run(name)._r)
    comment = doc.add_comment(doc.add_paragraph().add_run("comment range"), "comment", author="Editor")
    wrap(comment.paragraphs[0]._p)
    from docx.opc.part import Part
    from docx.opc.packuri import PackURI
    for tag in ("footnote","endnote"):
        root = elem(tag + "s")
        entry = elem(tag, id="1")
        payload = copy.deepcopy(doc.element.xpath("//w:sdt[w:sdtPr/w:lock/@w:val='contentLocked']")[0])
        entry.append(payload)
        root.append(entry)
        part = Part(PackURI("/word/" + tag + "s.xml"),
                    "application/vnd.openxmlformats-officedocument.wordprocessingml."+tag+"s+xml",
                    etree.tostring(root), doc.part.package)
        doc.part.relate_to(part, NS["r"]+"/"+tag+"s")
        doc.add_paragraph().add_run()._r.append(elem(tag+"Reference",id="1"))
    p = doc.add_paragraph("mixed-field left")
    field = elem("fldSimple", instr=" PAGE ")
    field.append(p.add_run("cached page")._r)
    p._p.append(field)
    p.add_run("mixed-field right")
    doc.add_section()
    doc.save(directory / "rich-control-source.docx")


def verify_controls(directory):
    from rocx.editor import ControlEdit, ControlProperties, ImportedDocument
    path = directory / "control-source.docx"
    data = path.read_bytes()
    doc = ImportedDocument.open(data)
    c = doc.controls
    assert [v.context for v in c] == ["block", "inline", "row", "cell", "block", "block"]
    assert all(v.tag == "repeat" and v.id is None for v in c) and c[4].bound
    edits = [ControlEdit(c[0].anchor, text=" block "), ControlEdit(c[1].anchor, alias="caption"),
             ControlEdit(c[2].anchor, text="new row"), ControlEdit(c[3].anchor, tag="cell", text="new cell")]
    for target, batch, reason in [
        (doc, [edits[0], ControlEdit(c[1].anchor, text="no")], "locked"),
        (doc, [ControlEdit(c[0].anchor, tag="no")], "locked"),
        (doc, [edits[0], ControlEdit(c[4].anchor, alias="no")], "XML-bound"),
        (doc, [edits[0], ControlEdit(c[5].anchor, text="no")], "structure"),
        (doc, [edits[0], edits[0]], "duplicate"),
        (ImportedDocument.open(data), edits, "snapshot"),
    ]:
        try:
            target.preview([], controls=batch)
        except ValueError as exc:
            assert reason in str(exc), exc
        else:
            raise AssertionError("expected " + reason)
    output = directory / "controlEdits.docx"
    assert doc.preview([], controls=edits).to_docx() == output.read_bytes()
    src, out = parts(path), parts(output)
    untouched(src, out, (MAIN,))
    readback = Document(output)
    assert readback.paragraphs[-1].text == "untouched"
    assert readback.element.xpath("//w:sdtContent//w:t/text()") == [
        " block ", "inline", "new row", "new cell", "bound", "first", "second"]
    assert readback.element.xpath("//w:sdtPr/w:alias/@w:val") == ["caption"]
    assert readback.element.xpath("//w:sdtPr/w:tag/@w:val").count("repeat") == 5
    assert readback.element.xpath("//w:sdtPr/*[local-name()='probe']/text()") == ["opaque"]
    assert readback.element.xpath("//w:sdtContent/w:r/w:rPr/w:b")
    assert readback.element.xpath("//w:t[.=' block ']/@xml:space") == ["preserve"]

    data = (directory / "control-choice-source.docx").read_bytes()
    source = ImportedDocument.open(data)
    choice, = source.controls
    assert (choice.kind, choice.selected, choice.choices) == (
        "dropDownList", "1", [("First", "1"), ("Second", "2")])
    def refused(action, reason):
        try:
            action()
        except ValueError as exc:
            assert reason in str(exc), exc
        else:
            raise AssertionError("expected " + reason)
    refused(lambda: source.preview_control_choice(choice.anchor, "absent"), "unknown")
    refused(lambda: source.preview_control_choice(choice.anchor, "x", choices=[("A","x"),("B","x")]), "unique")
    refused(lambda: doc.preview_control_choice(choice.anchor, "2"), "snapshot")
    refused(lambda: doc.preview_unwrap_control(c[0].anchor), "locked")
    refused(lambda: doc.preview_unwrap_control(c[4].anchor), "bound")
    refused(lambda: doc.preview_unwrap_control(c[1].anchor), "opaque")
    target = source.body[0].inline_structure[0].anchor
    marked = ImportedDocument.open(source.set_paired_range(90, "Guard", (target,target)))
    assert not any(s.text == "block" for s in marked.control_sites)
    refused(lambda: marked.preview_tracked_bold(marked.body[0].inline_structure[1].anchor, False, "Reviewer"), "protected")
    plain = ImportedDocument.open(marked.set_paired_range(90, "Guard"))
    proposed = ImportedDocument.open(plain.preview_tracked_bold(plain.body[0].inline_structure[0].anchor, False, "Reviewer").to_docx())
    revised, other = [p.inline_structure[0] for p in proposed.body[:2]]
    assert not any(s.text == "block" for s in proposed.control_sites)
    refused(lambda: proposed.set_paired_range(int(revised.property_revision.id), "Other", (other.anchor,other.anchor)), "collides")
    accepted = ImportedDocument.open(proposed.preview_revision(revised.anchor, "accept").to_docx())
    site, = (s for s in accepted.control_sites if s.context == s.text == "block")
    workflow = accepted.preview_wrap_control(site, "reviewed").to_docx()
    assert workflow == (directory / "guardedWorkflow.docx").read_bytes()
    untouched(parts(directory / "control-choice-source.docx"), parts(io.BytesIO(workflow)), (MAIN,))
    xml = Document(io.BytesIO(workflow)).element
    assert not xml.xpath("//w:bookmarkStart | //w:bookmarkEnd | //w:rPrChange")
    assert xml.xpath("//w:sdt[w:sdtPr/w:tag/@w:val='reviewed']/w:sdtContent/w:p/w:r[w:t='block']/w:rPr/w:b/@w:val") == ["0"]
    selected = ImportedDocument.open(source.preview_control_choice(choice.anchor, "2").to_docx())
    assert selected.controls[0].selected == "2" and selected.controls[0].choices == choice.choices
    data = selected.preview_control_choice(selected.controls[0].anchor, "R", choices=[(" Revised ", "R")]).to_docx()
    for context in ("block", "inline", "row", "cell"):
        target = ImportedDocument.open(data)
        site, = (s for s in target.control_sites if s.context == s.text == context)
        refused(lambda: source.preview_wrap_control(site, "bad"), "snapshot")
        data = target.preview_wrap_control(site, context).to_docx()
    target = ImportedDocument.open(data)
    by_tag = {c.tag: c.anchor for c in target.controls}
    definition = ControlProperties(tag="inline", text_type="plain", multi_line=False, id=42)
    refused(lambda: target.preview_replace_control(by_tag["row"], definition), "rich text")
    data = target.preview_replace_control(by_tag["inline"], definition).to_docx()
    target = ImportedDocument.open(data)
    by_tag = {c.tag: c for c in target.controls}
    assert (by_tag["inline"].kind, by_tag["inline"].multi_line, by_tag["inline"].id) == ("text", "0", "42")
    refused(lambda: target.preview_replace_control(by_tag["row"].anchor, ControlProperties(id=42)), "duplicate")
    assert data == (directory / "controlWrapped.docx").read_bytes()
    xml = Document(io.BytesIO(data)).element
    assert xml.xpath("//w:sdtPr/w:dropDownList/@w:lastValue") == ["R"]
    assert xml.xpath("//w:sdtContent/w:r/w:t[.=' Revised ']/@xml:space") == ["preserve"]
    assert xml.xpath("//w:sdtContent/w:p/w:r/w:rPr/w:b") and xml.xpath("//w:sdtContent/w:r/w:rPr/w:b")
    assert xml.xpath("//w:tbl/w:sdt/w:sdtContent/w:tr") and xml.xpath("//w:tr/w:sdt/w:sdtContent/w:tc")
    for context in ("block", "inline", "row", "cell"):
        target = ImportedDocument.open(data)
        control, = (c for c in target.controls if c.tag == c.context == context)
        data = target.preview_unwrap_control(control.anchor).to_docx()
    assert data == (directory / "controlUnwrapped.docx").read_bytes()
    reopened = Document(io.BytesIO(data))
    assert [p.text for p in reopened.paragraphs[:2]] == ["block", "inline"]
    assert [t.cell(0, 0).text for t in reopened.tables] == ["row", "cell"]
    assert reopened.paragraphs[0].runs[0].bold and reopened.paragraphs[1].runs[0].bold
    for name in ("controlWrapped", "controlUnwrapped"):
        untouched(parts(directory / "control-choice-source.docx"), parts(directory / (name + ".docx")), (MAIN,))

    import asyncio
    from rocx import declarative as d
    from rocx.editor import (FieldEdit, InlineEdit, PairedRangeEdit, WrapperEdit as W,
                             HeaderImageEdit, ParagraphAlignmentEdit, ParagraphInsertion)
    rich_path = directory / "rich-control-source.docx"
    rich = ImportedDocument.open(rich_path.read_bytes())
    assert sorted(c.owner_part for c in rich.story_controls) == [
        "word/comments.xml","word/endnotes.xml","word/footer1.xml","word/footnotes.xml","word/header1.xml"]
    assert sorted(c.entry_id for c in rich.story_controls if c.entry_index is not None) == ["0","1","1"]
    refused(lambda: rich.preview_unwrap_control(rich.story_controls[0].anchor), "read-only")
    site = lambda ctx, label: next(s for s in rich.control_sites if s.context == ctx and s.text == label)
    def wrap(ctx, first, last, tag):
        return W.wrap(site(ctx, first), tag, last=site(ctx,last))
    async def rich_batch():
        a,b,adjacent = [await d.Paragraph(text=t).resolve() for t in ("A","B","adjacent")]
        image = await d.ImageRun((directory / "image.png").read_bytes()).resolve()
        runs = [await d.TextRun("picture:").resolve(), image]
        c = rich.controls
        wrappers = [wrap("block","range start","range end","group"),wrap("inline","one","two","runs"),
                    W.replace_blocks(c[0].anchor, [a,b]), W.replace_inlines(c[1].anchor,runs)]
        p = next(p for p in rich.body if getattr(p, "text", "").startswith("mixed-field left"))
        left, _, right = p.inline_structure
        field = FieldEdit.patch(p.anchor, 1, "dirty", True)
        mixed_range = PairedRangeEdit(91, "MixedControl", (left.anchor, right.anchor))
        crossed = PairedRangeEdit(92, "Crossed",
                                  (rich.body[1].inline_structure[0].anchor, rich.body[3].inline_structure[0].anchor))
        refused(lambda: rich.preview([], wrappers=[wrappers[0],W.replace_blocks(c[2].anchor,[a])]),"locked")
        refused(lambda: rich.preview([ParagraphAlignmentEdit(rich.body[1].anchor,"center")],wrappers=wrappers),"conflicting")
        refused(lambda: rich.preview([], wrappers=wrappers, paired_ranges=[crossed]), "conflicting")
        return rich.preview([],wrappers=wrappers,paragraph_insertions=[ParagraphInsertion(rich.body[0].anchor,adjacent)],
            fields=[field], paired_ranges=[mixed_range],
            inline_edits=[InlineEdit.insert_after(rich.body[0].inline_structure[0].anchor,image),
                          InlineEdit.patch_format(left.anchor,size=13.5)],
            header_images=[HeaderImageEdit(rich.default_headers[0].anchor,image,mode="shared")]).to_docx()
    mixed = asyncio.run(rich_batch())
    assert mixed == (directory / "richControls.docx").read_bytes()
    xml = Document(io.BytesIO(mixed)).element
    grouped = xml.xpath("//w:sdt[w:sdtPr/w:tag/@w:val='group']/w:sdtContent")[0]
    assert [c.tag for c in grouped] == [qn("w:p"),qn("w:tbl"),qn("w:p")]
    assert xp(grouped,".//w:r[w:rPr/w:b]/w:t/text()") == ["start"]
    assert xml.xpath("//w:sdt[w:sdtPr/w:tag/@w:val='runs']/w:sdtContent/w:r[w:rPr/w:i]/w:t/text()") == ["two"]
    assert xml.xpath("//w:sdt[w:sdtPr/w:lock/@w:val='sdtLocked']/w:sdtContent/w:p/w:r/w:t/text()") == ["A","B"]
    readback = Document(io.BytesIO(mixed))
    assert [p.text for p in readback.paragraphs[:2]] == ["ordinary","adjacent"]
    assert len(readback.inline_shapes)==1  # python-docx omits drawings inside inline controls.
    assert len(xml.xpath(".//w:sdtContent//wp:docPr")) == 1
    header=readback.sections[0].header._element
    ids = header.xpath(".//wp:docPr/@id") + xml.xpath(".//wp:docPr/@id")
    assert len(ids) == len(set(ids)) == 3
    assert xml.xpath("//w:fldSimple/@w:dirty") == ["1"]
    assert xml.xpath("//w:bookmarkStart[@w:name='MixedControl']/following-sibling::w:r[1]/w:rPr/w:sz/@w:val") == ["27"]
    untouched(parts(rich_path),parts(io.BytesIO(mixed)),(MAIN,MAIN_RELS,HEADER,HEADER_RELS,TYPES))
    promoted=ImportedDocument.open(mixed)
    protected = next(p for p in promoted.body if getattr(p, "text", "").startswith("mixed-field left"))
    refused(lambda: promoted.preview_tracked_bold(protected.inline_structure[1].anchor, False, "Reviewer"), "protected")
    data=promoted.preview([],wrappers=[W.unwrap(c.anchor) for c in promoted.controls if c.tag in ("group","runs")]).to_docx()
    assert data == (directory / "richUnwrapped.docx").read_bytes()
    restored=Document(io.BytesIO(data))
    assert [p.text for p in restored.paragraphs[:5]] == ["ordinary","adjacent","range start","range end","onetwooutside"]
    assert restored.tables[0].cell(0,0).text=="retained table"
    untouched(parts(io.BytesIO(mixed)),parts(io.BytesIO(data)),(MAIN,))


def numbering_fixture():
    from runpy import run_path
    return run_path(str(Path(__file__).resolve().parents[3] / "tests/fixtures/generate_numbering_catalog.py"))


def generate_physical_resources(path):
    word = Document()
    word.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    word.add_paragraph("Body remains unchanged")
    footer = word.sections[0].footer
    footer.paragraphs[0].text = "Shared numbered footer"
    footer.add_table(1, 1, Twips(2400)).cell(0, 0).text = "Styled footer cell"
    word.add_section()
    section = word.sections[0]._sectPr
    section.find(qn("w:pgMar")).set(qn("w14:probe"), "margin extension")
    borders = etree.Element(qn("w:pgBorders"))
    borders.set(qn("w14:probe"), "border group extension")
    right = etree.SubElement(borders, qn("w:right"))
    right.set(qn("w:val"), "single")
    right.set(qn("w:sz"), "8")
    right.set(qn("w:color"), "667788")
    right.set(qn("w14:probe"), "opaque right edge")
    section.insert_element_before(borders, "w:cols", "w:docGrid")
    word.save(path)


def generate(directory):
    fixture = numbering_fixture()
    for name, fault in [("numbering-linked", None), ("numbering-dangling", "dangling"),
                        ("numbering-opaque", "opaque")]:
        (directory / f"{name}.docx").write_bytes(fixture["source"](fault))
    (directory / "linked-mixed-source.docx").write_bytes(fixture["source"](mixed=True))
    generate_physical_resources(directory / "physical-resources-source.docx")
    shared = Document()
    shared.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    comment = shared.add_comment(shared.add_paragraph("reference").runs[0], text="annotation", author="Verifier")
    comment.add_table(1, 1, Twips(1440)).cell(0,0).text = "comment table"
    outer = shared.add_table(1, 1)
    outer.cell(0,0).add_table(1, 1).cell(0,0).text = "nested body"
    footer = shared.sections[0].footer.add_table(1, 1, Twips(2880))
    footer.cell(0,0).add_table(1, 1).cell(0,0).text = "shared footer"
    shared.add_section()
    shared.save(directory / "physical-table-source.docx")
    merged = Document()
    merged.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    table = merged.add_table(rows=2, cols=3)
    table.autofit = False
    table.cell(0, 0).merge(table.cell(1, 1)).text = "origin"
    merged.add_table(rows=1, cols=1).cell(0, 0).text = "remove me"
    merged.add_paragraph("after both tables")
    merged.save(directory / "table-layout.docx")
    layout = Document()
    layout.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    layout.add_table(rows=1, cols=1).cell(0, 0).text = "table-first section"
    p = layout.add_paragraph("direct layout")
    p.paragraph_format.space_before = Twips(123)
    p.paragraph_format.first_line_indent = Twips(72)
    p.alignment = WD_ALIGN_PARAGRAPH.RIGHT
    p._p.pPr.spacing.set(qn("w14:probe"), "retained")
    layout.sections[0].header.paragraphs[0].text = "retained header"
    layout.add_section()
    # Empty implicit final section; neither section starts with a paragraph.
    layout._element.body.remove(layout.sections[-1]._sectPr)
    layout.save(directory / "layout-source.docx")
    generate_controls(directory)
    paired = Document()
    paired.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    first = paired.add_paragraph()
    field = elem("fldSimple", instr=" PAGE ")
    field.append(first.add_run("old cache")._r)
    first._p.append(field)
    paired.add_comment(paired.add_paragraph().add_run("annotated"), text="original comment", author="Reviewer")
    p = paired.add_paragraph()
    link = elem("hyperlink")
    link.set(qn("r:id"), p.part.relate_to(BODY_LINK, RELATIONSHIP_TYPE.HYPERLINK, is_external=True))
    link.append(p.add_run("link")._r)
    p._p.append(link)
    paired.add_paragraph("last")
    paired.save(directory / "paired-source.docx")
    owners = Document()
    owners.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    cell = owners.add_table(rows=1, cols=1).cell(0, 0)
    for label, p in [("cell",cell.paragraphs[0]), ("header",owners.sections[0].header.paragraphs[0]),
                     ("footer",owners.sections[0].footer.paragraphs[0])]:
        link = elem("hyperlink")
        link.set(qn("r:id"),p.part.relate_to("https://example.invalid/"+label, RELATIONSHIP_TYPE.HYPERLINK,is_external=True))
        run = p.add_run(label+"\tword")
        run.add_picture(io.BytesIO(png((11,22,33))),width=Twips(100))
        run._r.append(elem("noBreakHyphen"))
        link.append(run._r)
        p._p.append(link)
    owners.add_section()
    owners.save(directory / "format-owners-source.docx")
    # A complex field starts in one paragraph and ends in another. Its result
    # has multiple leaves in a formatted run; unsupported symbol neighbors
    # and XML comments lie immediately outside the editable span.
    p = paired.add_paragraph()
    p.add_run()._r.append(elem("sym", font="Wingdings", char="F0FC"))
    p._p.append(etree.Comment("before-field"))
    p.add_run()._r.append(elem("fldChar", fldCharType="begin", fldLock="1"))
    code = elem("instrText")
    code.text = " PAGE "
    p.add_run()._r.append(code)
    p.add_run()._r.append(elem("fldChar", fldCharType="separate"))
    run = p.add_run("old")
    run.italic = True
    run.add_tab()
    run.add_text("cache")
    q = paired.add_paragraph()
    q.paragraph_format.keep_together = True
    q.add_run("more old cache")
    q.add_run()._r.append(elem("fldChar", fldCharType="end"))
    q._p.append(etree.Comment("after-field"))
    q.add_run()._r.append(elem("sym", font="Wingdings", char="F0FD"))
    paired.save(directory / "field-batch-source.docx")
    catalog = Document()
    catalog.add_paragraph("Numbering target")
    catalog.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    for name in ["Memo Replace", "Memo Drop", "Memo Active"]:
        catalog.styles.add_style(name, WD_STYLE_TYPE.PARAGRAPH).font.italic = False
    catalog.sections[0].header.paragraphs[0].style = "Memo Active"
    catalog.add_paragraph("Style migration", style="Memo Active")
    catalog.styles.add_style("Memo Follower", WD_STYLE_TYPE.PARAGRAPH).base_style = catalog.styles["Memo Active"]
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
    p.runs[-1]._r.rPr.find(qn("w:u")).set(qn("w:color"), "135790")
    p.runs[-1]._r.rPr.append(elem("lang", bidi="ar"))
    p.add_run("cd").italic = True
    ranged.add_paragraph(style="Caption")
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


def verify_ordinary(directory):
    from rocx import declarative as d
    from rocx.editor import BlockEdit, ImportedDocument, ResourceEdit, StyleEdit, TextRangeEdit
    source = directory / "base.docx"
    output = directory / "combined.docx"
    doc = ImportedDocument.open(source.read_bytes())
    assert doc.preview(
        [],
        text_ranges=[TextRangeEdit(doc.body[1].anchor, 0, 1, "X")],
        block_edits=[BlockEdit.keep_with_next(doc.body[4].anchor, True),
                     BlockEdit.paragraph_flag(doc.body[4].anchor, "page_break_before", False),
                     BlockEdit.section_flag(doc.body[4].anchor, "title_page", True),
                     BlockEdit.section_type(doc.body[0].anchor, "continuous")],
        style_edits=[StyleEdit.add(d.NumberingStyleDefinition("Batch List", 702))],
        resources=[ResourceEdit.document_default("tabStops", [(720, "right", "dot")]),
                   ResourceEdit.core_title("Combined editor"), ResourceEdit.metadata("subject", "Batch resources"),
                   ResourceEdit.document_default("bold", False), ResourceEdit.add_instance(702,701),
                   ResourceEdit.add_abstract(701, [d.NumberingLevel()])],
    ).to_docx() == output.read_bytes()
    original, result = parts(source), parts(output)
    assert original.keys() == result.keys()
    untouched(original, result, (MAIN, "docProps/core.xml", "word/styles.xml", "word/numbering.xml"))
    tab, = xp(etree.fromstring(result["word/styles.xml"]), "w:docDefaults/w:pPrDefault/w:pPr/w:tabs/w:tab")
    assert [tab.get(qn("w:" + attr)) for attr in ("val", "pos", "leader")] == ["right", "720", "dot"]
    for query in (".//w:fldSimple", ".//w:drawing"):
        assert [tree(n) for n in xp(etree.fromstring(original[MAIN]), query)] == [
            tree(n) for n in xp(etree.fromstring(result[MAIN]), query)]
    readback = Document(output)
    assert readback.core_properties.title == "Combined editor"
    assert readback.core_properties.subject == "Batch resources"
    assert readback.styles["Batch List"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["702"]
    assert readback.part.numbering_part.element.xpath("w:abstractNum[@w:abstractNumId='701']/w:styleLink/@w:val") == ["BatchList"]
    p = readback.paragraphs
    assert p[1].text == "Xnset alignment"
    assert p[4].paragraph_format.keep_with_next is True and p[4].text == "next section"
    assert p[4].paragraph_format.page_break_before is False
    before_sections = xp(etree.fromstring(original[MAIN]), ".//w:sectPr")
    after_sections = xp(etree.fromstring(result[MAIN]), ".//w:sectPr")
    assert len(before_sections) == len(after_sections)
    for i, (old, new) in enumerate(zip(before_sections, after_sections)):
        if i < 2:
            name, expected = [("type", "continuous"), ("titlePg", "1")][i]
            assert new.find(qn("w:" + name)).get(qn("w:val")) == expected
            for node in (old, new):
                prop = node.find(qn("w:" + name))
                if prop is not None:
                    node.remove(prop)
        assert tree(old) == tree(new)


def verify_field_batch(directory):
    import asyncio
    from rocx import declarative as d
    from rocx.editor import FieldEdit, ImportedDocument, InlineEdit, PairedRangeEdit
    source = directory / "field-batch-source.docx"
    doc = ImportedDocument.open(source.read_bytes())
    async def payload():
        return [await d.Hyperlink("https://example.invalid/cache", [d.TextRun("remote")]).resolve(),
                await d.Hyperlink(d.BookmarkTarget("BatchMark"), [d.TextRun("jump")]).resolve()]
    at = lambda p, i: doc.body[p].inline_structure[i].anchor
    fields = [
        FieldEdit.cache(doc.body[4].anchor, 1, asyncio.run(payload())),
        FieldEdit.patch(doc.body[4].anchor, 1, "dirty", False),
        FieldEdit.patch(doc.body[0].anchor, 0, "dirty", True),
        FieldEdit.patch(doc.body[0].anchor, 0, "instruction", " NUMPAGES "),
    ]
    ranges = [PairedRangeEdit(7, "BatchMark", (at(1, 1), at(2, 0))),
              PairedRangeEdit(0, endpoints=(at(2, 0), at(3, 0)))]
    links = [InlineEdit.hyperlink_tooltip(at(2, 0), "Batch tooltip"), InlineEdit.hyperlink_history(at(2, 0), False)]
    batch = doc.preview([], fields=fields, paired_ranges=ranges, inline_edits=links)
    data = batch.to_docx()
    assert doc.preview([], fields=fields[::-1], paired_ranges=ranges[::-1], inline_edits=links[::-1]).to_docx() == data
    assert data == (directory / "fieldBatch.docx").read_bytes()
    before, after = parts(source), parts(io.BytesIO(data))
    untouched(before, after, (MAIN, MAIN_RELS))
    rels = lambda parts: {n.get("Id"): dict(n.attrib) for n in etree.fromstring(parts[MAIN_RELS])}
    old, new = rels(before), rels(after)
    assert old.items() <= new.items()
    added, = new.keys() - old.keys()
    assert new[added]["Target"] == "https://example.invalid/cache" and new[added]["TargetMode"] == "External"
    word = Document(io.BytesIO(data))
    p, q = word.paragraphs[4:6]
    assert [link.text for link in p.hyperlinks] == ["remote", "jump"]
    assert p._p.xpath("w:hyperlink/@r:id") == [added]
    assert p._p.xpath("w:hyperlink/@w:anchor") == ["BatchMark"]
    assert p._p.xpath("w:r/w:fldChar/@w:fldCharType") == ["begin", "separate"]
    assert p._p.xpath("w:r/w:fldChar/@w:dirty") == ["0"] and p._p.xpath("w:r/w:fldChar/@w:fldLock") == ["1"]
    assert q._p.xpath("w:r/w:fldChar/@w:fldCharType") == ["end"] and q.paragraph_format.keep_together
    assert word.element.xpath("//w:sym/@w:char") == ["F0FC", "F0FD"]
    assert [c.text for c in word.element.xpath("//comment()")] == ["before-field", "after-field"]
    assert word.paragraphs[2].hyperlinks[0].url == BODY_LINK
    assert word.paragraphs[2]._p.xpath("w:hyperlink/@w:tooltip") == ["Batch tooltip"]
    assert word.paragraphs[2]._p.xpath("w:hyperlink/@w:history") == ["0"]
    assert word.element.xpath("//w:fldSimple/@w:dirty") == ["1"]
    assert word.element.xpath("//w:fldSimple/@w:instr") == [" NUMPAGES "]
    assert len(word.comments) == 1


def verify_fields_ranges(directory):
    import asyncio
    from rocx import declarative as decl
    from rocx.editor import ImportedDocument, InlineEdit
    source = directory / "paired-source.docx"
    d = ImportedDocument.open(source.read_bytes())
    field = asyncio.run(decl.Field(decl.FieldInstruction.page(), [decl.TextRun("new cache")], form="complex").resolve())
    old = d.body[0].anchor
    d = ImportedDocument.open(d.replace_field(old, 0, field))
    try:
        d.patch_field(old, 0, "dirty", True)
    except ValueError as exc:
        assert "snapshot" in str(exc)
    else:
        raise AssertionError("accepted stale paragraph")
    d = ImportedDocument.open(d.patch_field(d.body[0].anchor, 0, "instruction", " NUMPAGES "))
    info = d.field_at(d.body[0].anchor, 0)
    assert (info.complex, info.inline_count, info.instruction, info.dirty_xml) == (True, 5, " NUMPAGES ", None)
    at = lambda p, i: d.body[p].inline_structure[i].anchor
    d = ImportedDocument.open(d.preview([], inline_edits=[InlineEdit.hyperlink_tooltip(at(2, 0), "hover")]).to_docx())
    assert d.body[2].inline_structure[0].tooltip == "hover"
    d = ImportedDocument.open(d.set_paired_range(0, endpoints=(at(2, 0), at(3, 0))))
    assert [(m.kind, m.body_index, m.inline_index, m.id) for m in d.body_range_markers] == [
        ("commentRangeStart", 2, 0, "0"), ("commentRangeEnd", 3, 1, "0"), ("commentReference", 3, 2, "0")]
    removed = Document(io.BytesIO(d.set_paired_range(0)))
    assert not removed.element.xpath("//w:commentReference | //w:commentRangeStart | //w:commentRangeEnd")
    assert len(removed.comments) == 1
    data = d.set_paired_range(7, "Crossing", (at(1, 0), at(2, 1)))
    assert data == (directory / "fieldsRanges.docx").read_bytes()
    untouched(parts(source), parts(io.BytesIO(data)), (MAIN,))
    out = Document(io.BytesIO(data))
    assert out.element.xpath("//w:instrText/text()") == [" NUMPAGES "]
    assert out.element.xpath("//w:fldChar/@w:fldCharType") == ["begin", "separate", "end"]
    assert out.paragraphs[0].text == "new cache"
    assert out.paragraphs[2].hyperlinks[0].url == BODY_LINK
    assert out.element.xpath("//w:hyperlink/@w:tooltip") == ["hover"]
    assert out.paragraphs[1]._p.xpath("w:bookmarkStart/@w:name") == ["Crossing"]
    assert out.paragraphs[2]._p.xpath("w:commentRangeStart/@w:id") == ["0"]
    assert out.paragraphs[2]._p.xpath("w:bookmarkEnd/@w:id") == ["7"]
    assert out.paragraphs[3]._p.xpath("w:commentRangeEnd/@w:id | w:r/w:commentReference/@w:id") == ["0", "0"]


def verify_rich(directory):
    verify_field_batch(directory)
    verify_fields_ranges(directory)
    from rocx.editor import ImportedDocument, InlineEdit
    raw = (directory / "format-owners-source.docx").read_bytes()
    source = parts(io.BytesIO(raw))
    for label in ("cell","header","footer"):
        d = ImportedDocument.open(raw)
        if label == "cell":
            owner, paragraph = None, d.body[0].children[0].children[0].blocks[0]
        else:
            story = next(s for s in d.stories if s.kind == label and s.section_index == 1 and s.variant == "default")
            owner, paragraph = story.part_name, story.blocks[0]
        target = paragraph.inline_structure[0].children[0].anchor
        raw = d.edit_run_format([InlineEdit.patch_format(target,size=13.5)],owner_part=owner)
    assert raw == (directory / "formatOwners.docx").read_bytes()
    output = parts(io.BytesIO(raw))
    owners = (MAIN, HEADER, "word/footer1.xml")
    untouched(source,output,owners)
    for part in owners:
        old, changed = [etree.fromstring(z[part]) for z in (source,output)]
        rpr, = xp(changed,".//w:hyperlink/w:r/w:rPr")
        assert xp(rpr,"./w:sz/@w:val") == ["27"]
        rpr.getparent().remove(rpr)
        assert tree(changed) == tree(old)
    readback = Document(io.BytesIO(raw))
    for p in [readback.tables[0].cell(0,0).paragraphs[0],readback.sections[1].header.paragraphs[0],
              readback.sections[1].footer.paragraphs[0]]:
        assert p.hyperlinks[0].runs[0].font.size.pt == 13.5


def verify_linked_mixed(source, output):
    from rocx.editor import ImportedDocument, InlineEdit, ResourceEdit as R, TableStructureEdit as T

    doc = ImportedDocument.open(source.read_bytes())
    p, removed, retained = doc.body
    number = doc.cell_paragraph_anchor(retained.anchor, 0, 0, 0)
    resources = [*numbering_fixture()["edits"](doc), R.story_paragraph_numbering(number, (713, 0))]
    batch = dict(resources=resources, inline_edits=[InlineEdit.patch_format(p.inline_structure[0].anchor, bold=True)],
                 table_structure=[T.remove_table(removed.anchor)])
    data = output.read_bytes()
    assert doc.preview([], **batch).to_docx() == data
    assert doc.preview([], **(batch | {"resources": resources[::-1]})).to_docx() == data
    result = ImportedDocument.open(data)
    after_number = result.cell_paragraph_anchor(result.body[1].anchor, 0, 0, 0)
    assert result.story_paragraph_numbering(after_number) == (713, 0)
    for imported, kwargs, reason in [
        (doc, batch | {"table_structure": [T.remove_table(retained.anchor)]}, "conflicting cell paragraph"),
        (doc, batch | {"resources": [*resources, R.remove_instance(712)]}, "references"),
        (result, batch, "snapshot"),
    ]:
        try:
            imported.preview([], **kwargs)
        except ValueError as exc:
            assert reason in str(exc), exc
        else:
            raise AssertionError(reason)
    assert doc.preview([], **batch).to_docx() == data
    before, after = parts(source), parts(output)
    allowed = (MAIN, "word/numbering.xml", "word/settings.xml", "docProps/core.xml")
    untouched(before, after, allowed)
    assert all(before[name] != after[name] for name in allowed)
    word = Document(output)
    assert len(word.tables) == 1 and word.tables[0].cell(0, 0).text == "Keep and number this cell"
    assert word.paragraphs[0].runs[0].bold and word.paragraphs[0].text == "List through a numbering style"
    assert word.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["713"]
    assert word.sections[0].header.paragraphs[0].text == "Untouched header"
    old, new = [etree.fromstring(z["word/numbering.xml"]) for z in (before, after)]
    for path in ("w:abstractNum[@w:abstractNumId='711']", "w:num[@w:numId='712']", "w:num[@w:numId='713']"):
        assert tree(xp(old, path)[0]) == tree(xp(new, path)[0])
    assert xp(new, "w:abstractNum[@w:abstractNumId='710']/w:lvl/w:start/@w:val") == ["5"]


async def physical_resources(path):
    from rocx import declarative as d
    from rocx.editor import ImportedDocument, LayoutEdit as L, ResourceEdit as R, StyleEdit as S, TableStructureEdit as E

    owner = "word/footer1.xml"
    original = ImportedDocument.open(path.read_bytes())
    staged = original.preview([], style_edits=[
        S.add(d.TableStyleDefinition("Owner Table Style")),
        S.add(d.NumberingStyleDefinition("Owner Numbering Style", 773)),
    ], resources=[
        R.add_abstract(772, [d.NumberingLevel(level=0, start=1, format="decimal", text="%1.")]),
        R.add_instance(773, 772),
        R.story_paragraph_numbering(original.story_paragraph_anchor(owner, 0), (773, 0)),
    ]).to_docx()
    numbered = ImportedDocument.open(staged)
    donor = await d.Table([d.TableRow([d.TableCell([d.Paragraph("Donor text must not move")])])],
                          style="Owner Table Style").resolve()
    table, = numbered.part_tables(owner)
    tables_edited = numbered.edit_part_tables(owner, [E.patch_property(table.anchor, "style", donor)])
    pre_layout = ImportedDocument.open(tables_edited)
    section = pre_layout.sections[0].anchor
    result = pre_layout.preview_layout([
        L.section(section, "margin_left", 1440),
        L.section(section, "border_top", {"size": 12, "color": "123456"}),
    ]).to_docx()
    return staged, tables_edited, result


def verify_physical_resources(directory):
    import asyncio
    import pytest
    from rocx.editor import ImportedDocument, LayoutEdit as L, ResourceEdit as R, StyleEdit as S

    source = directory / "physical-resources-source.docx"
    output = directory / "physicalResources.docx"
    staged, tables_edited, result = asyncio.run(physical_resources(source))
    assert result == output.read_bytes()
    owner = "word/footer1.xml"
    def changed(old, new):
        a, b = parts(old), parts(new)
        assert a.keys() == b.keys()
        return {name for name in a if a[name] != b[name]}
    assert changed(source, io.BytesIO(staged)) == {owner, "word/styles.xml", "word/numbering.xml"}
    assert changed(io.BytesIO(staged), io.BytesIO(tables_edited)) == {owner}
    assert changed(io.BytesIO(tables_edited), output) == {MAIN}

    doc = ImportedDocument.open(result)
    assert doc.story_paragraph_numbering(doc.story_paragraph_anchor(owner, 0)) == (773, 0)
    table, = doc.part_tables(owner)
    assert table.properties["style"]["val"] == "OwnerTableStyle"
    with pytest.raises(ValueError, match="references"):
        doc.preview([], resources=[R.remove_instance(773)])
    with pytest.raises(ValueError, match="used|referenc"):
        doc.preview([], style_edits=[S.remove(doc.style_anchor("OwnerTableStyle"))])
    with pytest.raises(ValueError, match="opaque"):
        doc.preview_layout([L.section(doc.sections[0].anchor, "border_right", None)])
    word = Document(output)
    for section in word.sections:
        assert section.footer.tables[0].style.style_id == "OwnerTableStyle"
        assert section.footer.tables[0].cell(0, 0).text == "Styled footer cell"
    source_sect = Document(source).sections[0]._sectPr
    target_sect = word.sections[0]._sectPr
    assert target_sect.find(qn("w:pgMar")).get(qn("w14:probe")) == "margin extension"
    assert word.sections[0].left_margin.twips == 1440
    borders = target_sect.find(qn("w:pgBorders"))
    assert borders.get(qn("w14:probe")) == "border group extension"
    assert list(borders)[0].tag == qn("w:top")
    assert tree(borders.find(qn("w:right"))) == tree(source_sect.find(qn("w:pgBorders")).find(qn("w:right")))
    numbers = etree.fromstring(parts(output)["word/numbering.xml"])
    assert xp(numbers, "w:abstractNum[@w:abstractNumId='772']/w:styleLink/@w:val") == ["OwnerNumberingStyle"]


async def structural_resources(path):
    from rocx import declarative as d
    from rocx.editor import ImportedDocument, ResourceEdit as R, StyleEdit, TableStructureEdit as T

    source = ImportedDocument.open(path.read_bytes())
    first, second, paragraph = source.body
    table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(children=[
        d.Hyperlink("https://example.invalid/staged-table", [d.TextRun("staged link")])
    ])])])], style="Staged Table").resolve()
    batch = dict(table_structure=[T.replace_node(first.anchor, table), T.remove_table(second.anchor)],
                 style_edits=[StyleEdit.add(d.TableStyleDefinition("Staged Table"))])
    resources = [R.add_abstract(90, [d.NumberingLevel()]), R.add_instance(91, 90),
                 R.paragraph_numbering(paragraph.anchor, (91, 0)), R.custom_property("Staged", True)]
    try:
        source.preview([], **batch, resources=[*resources, R.remove_instance(1)])
    except ValueError as exc:
        assert "references" in str(exc), exc
    else:
        raise AssertionError("removed live numbering")
    result = source.preview([], **batch, resources=resources).to_docx()
    assert source.preview([], **batch, resources=resources).to_docx() == result
    assert source.paragraph_numbering(paragraph.anchor) is None
    reopened = ImportedDocument.open(result)
    assert len(reopened.body) == 2 and reopened.custom_property("Staged") is True
    assert reopened.paragraph_numbering(reopened.body[1].anchor) == (91, 0)
    try:
        reopened.paragraph_numbering(paragraph.anchor)
    except ValueError as exc:
        assert "snapshot" in str(exc), exc
    else:
        raise AssertionError("accepted stale anchor")
    return result


def verify(directory):
    verify_physical_resources(directory)
    from rocx.editor import LayoutEdit as L, ImportedDocument
    path = directory / "layout-source.docx"
    doc = ImportedDocument.open(path.read_bytes())
    a, b = (s.anchor for s in doc.sections)
    p = doc.body[1].anchor
    edits = [L.paragraph(p, "space_after", 180), L.paragraph(p, "alignment", None),
             L.paragraph(p, "heading", 2), L.paragraph(p, "widow_control", False),
             L.paragraph(p, "first_line_indent", -240),
             L.paragraph(p, "tab_stops", [{"position": 720, "alignment": "right", "leader": "dot"}]),
             L.section(a, "page_width", 16000), L.section(a, "margin_top", -220),
             L.section(a, "column_count", 2), L.section(a, "border_top", {"color": "223344", "size": 8}),
             L.section(b, "grid_pitch", 360), L.section(b, "title_page", False)]
    out = directory / "layout.docx"
    assert doc.preview_layout(edits).to_docx() == out.read_bytes()
    old, new = parts(path), parts(out)
    untouched(old, new, (MAIN,))
    assert old.keys() == new.keys()
    read = Document(out)
    p = read.paragraphs[0]
    assert p.style.style_id == "Heading2" and p.paragraph_format.widow_control is False
    assert p.paragraph_format.space_before.twips == 123 and p.paragraph_format.space_after.twips == 180
    assert p._p.pPr.spacing.get(qn("w14:probe")) == "retained" and p.alignment is None
    assert p.paragraph_format.first_line_indent.twips == -240
    assert p.paragraph_format.tab_stops[0].position.twips == 720
    assert read.sections[0].page_width.twips == 16000 and read.sections[0].top_margin.twips == -220
    assert read.sections[0].header.paragraphs[0].text == "retained header"
    assert read.sections[1]._sectPr.find(qn("w:docGrid")).get(qn("w:linePitch")) == "360"
    import asyncio
    from rocx import declarative as d
    from rocx.editor import BlockEdit

    for name, snapshot, edit in [
        ("sectionSplit", doc, BlockEdit.split_section(a, 1)),
        ("sectionMerge", None, None),
    ]:
        edited_path = directory / f"{name}.docx"
        if snapshot is None:
            snapshot = ImportedDocument.open((directory / "sectionSplit.docx").read_bytes())
            edit = BlockEdit.merge_section(snapshot.sections[0].anchor)
        preview = snapshot.preview([], block_edits=[edit])
        assert preview.block_changes == [1] and preview.to_docx() == edited_path.read_bytes()
        untouched(old, parts(edited_path), (MAIN,))
        readback = Document(edited_path)
        assert [p.text for p in readback.paragraphs if p.text] == ["direct layout"]
        assert readback.tables[0].cell(0, 0).text == "table-first section"
        # python-docx lists only explicit section properties; rocx also
        # reports the empty implied final section.
        assert len(readback.sections) == (2 if name == "sectionSplit" else 1)
        assert len(ImportedDocument.open(edited_path.read_bytes()).sections) == len(readback.sections) + 1
        assert all(s.header.paragraphs[0].text == "retained header" for s in readback.sections)

    story_path = directory / "storyShared.docx"
    original_header = doc.default_headers[1]
    replacement = asyncio.run(d.Paragraph("Shared story replacement").resolve())
    assert doc.edit_story_blocks(original_header.anchor, "shared", 0, 1, [replacement]) == story_path.read_bytes()
    untouched(old, parts(story_path), (original_header.part_name,))
    assert all(s.header.paragraphs[0].text == "Shared story replacement" for s in Document(story_path).sections)
    verify_rich(directory)
    verify_controls(directory)
    verify_ordinary(directory)
    from rocx import declarative as d
    from rocx.editor import ImportedDocument, ResourceEdit, StyleEdit
    linked_source = directory / "numbering-linked.docx"
    linked_output = directory / "linkedNumbering.docx"
    linked = ImportedDocument.open(linked_source.read_bytes())
    assert linked.preview([], resources=numbering_fixture()["edits"](linked)).to_docx() == linked_output.read_bytes()
    untouched(parts(linked_source), parts(linked_output), (MAIN, "word/numbering.xml", "word/settings.xml", "docProps/core.xml"))
    word = Document(linked_output)
    assert word.core_properties.comments == "Numbering fixture review"
    assert word.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["713"]
    assert word.styles["Fixture List"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["712"]
    verify_linked_mixed(directory / "linked-mixed-source.docx", directory / "linkedMixed.docx")
    source_path = directory / "styles-source.docx"
    style_path = directory / "styleEdits.docx"
    styles_source = ImportedDocument.open(source_path.read_bytes())
    numbered = directory / "numberingEdits.docx"
    assert styles_source.preview([], resources=[
        ResourceEdit.add_abstract(90, [d.NumberingLevel()]), ResourceEdit.add_instance(91, 90),
        ResourceEdit.paragraph_numbering(styles_source.body[0].anchor, (91, 0)),
    ]).to_docx() == numbered.read_bytes()
    untouched(parts(source_path), parts(numbered), (MAIN, "word/numbering.xml"))
    assert Document(numbered).paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["91"]
    owned = styles_source.preview([],style_edits=[StyleEdit.add(d.NumberingStyleDefinition(name="Owned List",numbering_id=91))],
                          resources=[ResourceEdit.add_instance(91,90),ResourceEdit.add_abstract(90,[d.NumberingLevel()]),
                                     ResourceEdit.instance_start(91,0,6)]).to_docx()
    assert owned == (directory/"styleOwnedList.docx").read_bytes()
    untouched(parts(source_path),parts(io.BytesIO(owned)),("word/styles.xml","word/numbering.xml"))
    word = Document(io.BytesIO(owned))
    assert not word.paragraphs[0]._p.xpath("w:pPr/w:numPr")
    assert word.styles["Owned List"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["91"]
    nums = word.part.numbering_part.element
    assert nums.xpath("w:abstractNum[@w:abstractNumId='90']/w:styleLink/@w:val") == ["OwnedList"]
    assert nums.xpath("w:num[@w:numId='91']/w:lvlOverride[@w:ilvl='0']/w:startOverride/@w:val") == ["6"]
    reopened = ImportedDocument.open(owned)
    try:
        reopened.preview([],resources=[ResourceEdit.remove_instance(91)])
    except ValueError as error:
        assert "references" in str(error)
    else:
        raise AssertionError("removed style-owned concrete instance")
    edit = styles_source.preview([], style_edits=[
        StyleEdit.add(d.ParagraphStyleDefinition(name="Memo Added", base_style="Memo Active")),
        StyleEdit.replace(styles_source.style_anchor("MemoReplace"),
                          d.ParagraphStyleDefinition(name="Memo Replace", bold=True)),
        StyleEdit.remove(styles_source.style_anchor("MemoDrop")),
        StyleEdit.field(styles_source.style_anchor("Heading1"), "next", "MemoAdded"),
        StyleEdit.region(styles_source.style_anchor("TableGrid"), "nwCell", d.TableSectionFormat(bold=True, background=(10,20,30))),
        StyleEdit.field(styles_source.style_anchor("LightShading"), "shd.fill", "BADACE", region="firstRow"),
        StyleEdit.field(styles_source.style_anchor("LightShading"), "spacing.after", "180", region="firstRow"),
        StyleEdit.add(d.NumberingStyleDefinition(name="Memo List", numbering_id=1)),
        StyleEdit.patch(styles_source.style_anchor("MemoActive"), italic=True),
    ])
    assert edit.to_docx() == style_path.read_bytes()
    missing = ImportedDocument.open((directory/"styles-missing.docx").read_bytes())
    created = missing.preview([], style_edits=[StyleEdit.add(d.ParagraphStyleDefinition(name="First Style"))]).to_docx()
    assert created == (directory/"styleCreate.docx").read_bytes()
    assert missing.styles is None and "First Style" in Document(io.BytesIO(created)).styles
    untouched(parts(directory/"styles-missing.docx"), parts(io.BytesIO(created)), (MAIN_RELS,TYPES))
    migrated = styles_source.preview([], style_edits=[StyleEdit.retarget(styles_source.style_anchor("MemoActive"),
                                                                          styles_source.style_anchor("MemoReplace"))]).to_docx()
    assert migrated == (directory/"styleRetarget.docx").read_bytes()
    target = Document(io.BytesIO(migrated))
    assert "Memo Active" not in target.styles and next(p for p in target.paragraphs if p.text=="Style migration").style.name == "Memo Replace"
    assert target.sections[0].header.paragraphs[0].style.name == "Memo Replace"
    assert target.styles["Memo Follower"].base_style.name == "Memo Replace"
    untouched(parts(source_path), parts(io.BytesIO(migrated)), ("word/styles.xml",MAIN,HEADER))
    before, after = parts(source_path), parts(style_path)
    assert before.keys() == after.keys()
    untouched(before, after, ("word/styles.xml","word/numbering.xml"))
    readback = Document(style_path)
    assert "Memo Drop" not in readback.styles
    assert readback.styles["Memo Added"].base_style.name == "Memo Active"
    assert readback.styles["Memo Replace"].font.bold and readback.styles["Memo Replace"].font.italic is None
    assert readback.sections[0].header.paragraphs[0].style.font.italic
    original_ids = [s.style_id for s in Document(source_path).styles if s.name != "Memo Drop"]
    assert [s.style_id for s in readback.styles] == original_ids + ["MemoAdded","MemoList"]
    assert readback.styles["Heading 1"].next_paragraph_style.name == "Memo Added"
    grid = readback.styles["Table Grid"].element
    assert grid.xpath('./w:tblStylePr[@w:type="nwCell"]/w:tcPr/w:shd/@w:fill') == ["0A141E"]
    num = etree.fromstring(after["word/numbering.xml"])
    aid = xp(num,'w:num[@w:numId="1"]/w:abstractNumId/@w:val')[0]
    backlink = xp(num,f'w:abstractNum[@w:abstractNumId="{aid}"]/w:styleLink')[0]
    assert backlink.get(qn("w:val")) == "MemoList"
    backlink.getparent().remove(backlink)
    assert tree(num) == tree(etree.fromstring(before["word/numbering.xml"]))
    source = parts(directory / "base.docx")
    image = (directory / "image.png").read_bytes()
    reports = json.loads((directory / "reports.json").read_text(encoding="utf8"))
    assert parts(directory / "noop.docx") == source
    removal = parts(directory / "removal.docx")
    untouched(source, removal, (MAIN,))
    assert Document(directory / "removal.docx").paragraphs[0].text == ""
    root = etree.fromstring(removal[MAIN])
    assert root.find(".//" + qn("w:fldSimple")) is not None
    from rocx.editor import ImportedDocument, TextInsertion, TextRangeEdit
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
    from rocx.editor import InlineEdit
    from rocx.declarative import RunLanguage
    runs = ranged.body[1].inline_structure
    formatted = ranged.preview([], inline_edits=[
        InlineEdit.set_toggle(ranged.body[0].inline_structure[0].anchor, "bold", False),
        InlineEdit.set_toggle(runs[0].anchor, "hidden", True),
        InlineEdit.set_toggle(runs[1].anchor, "strike", True),
        InlineEdit.set_toggle(runs[2].anchor, "italic", None),
        InlineEdit.patch_format(runs[1].anchor, style="Strong", font="Cambria",
            font_east_asia="Yu Mincho", font_complex_script="Amiri", size=12.5, size_complex_script=14,
            underline="double", color={"theme":"accent2","tint":150}, superscript=True, all_caps=False,
            small_caps=True, snap_to_grid=False, bold_complex_script=True, italic_complex_script=False,
            rtl=True, complex_script=True, language=RunLanguage(latin="fr-CA", east_asia="ja-JP"), clear=["language_bidi"])])
    formatted_path = directory / "runFormatting.docx"
    assert formatted.to_docx() == formatted_path.read_bytes()
    untouched(range_source, parts(formatted_path), (MAIN,))
    heading, content = Document(formatted_path).paragraphs[:2]
    a, b, c = content.runs
    assert heading.runs[0].bold is False and a.bold and a.font.hidden
    assert b.underline and b.font.strike and c.italic is None
    assert b.font.size.pt == 12.5 and b.font.name == "Cambria" and b.style.style_id == "Strong"
    assert b._r.rPr.find(qn("w:u")).get(qn("w:color")) == "135790"
    rich = ImportedDocument.open(formatted.to_docx())
    view = rich.body[1].inline_structure[1]
    assert view.has_opaque_formatting and view.format["language"] == RunLanguage(latin="fr-CA", east_asia="ja-JP")
    assert view.format["size_complex_script"] == 14 and view.format["underline"] == "double"
    clear = [name for name in view.format if name not in ("language","superscript","underline")]
    cleared = rich.preview([], inline_edits=[InlineEdit.patch_format(view.anchor,
        clear=clear + ["language_latin","language_east_asia","vertical_alignment"])])
    remaining = ImportedDocument.open(cleared.to_docx()).body[1].inline_structure[1]
    assert remaining.format == {"underline":"double"} and remaining.has_opaque_formatting
    selection = TextRangeEdit(ranged.body[1].anchor, 1, 6, " 🐈 ")
    assert ranged.preview([], text_ranges=[selection]).to_docx() == (directory / "range.docx").read_bytes()
    point_bytes = (directory / "points.docx").read_bytes()
    preview = ranged.preview([], text_insertions=[
        TextInsertion(ranged.body[1].anchor, 3, "➡"),
        TextInsertion(ranged.body[1].anchor, 7, "!"),
        TextInsertion(ranged.body[-1].anchor, 0, " Empty paragraph ")])
    assert preview.to_docx() == point_bytes
    point_parts = parts(directory / "points.docx")
    assert point_parts.keys() == range_source.keys()
    untouched(range_source, point_parts, (MAIN,))
    paragraphs = Document(directory / "points.docx").paragraphs
    assert [(r.text, r.underline, r.italic) for r in paragraphs[1].runs] == [
        ("a😀b", None, None), ("➡e\u0301", True, None), ("cd!", None, True)]
    assert paragraphs[-1].text == " Empty paragraph " and paragraphs[-1].style.name == "Caption"
    imported = ImportedDocument.open((directory / "base.docx").read_bytes())
    anchor = imported.body[0].inline_structure[0].anchor
    date = "2024-02-29T12:34:56Z"
    current = imported
    original_text = [p.text for p in Document(directory / "base.docx").paragraphs]
    for label, kind, args in [("Insert","insertBefore",{"text":" proposed "}),("Delete","delete",{})]:
        data = current.preview_tracked_paragraph(current.body[1].anchor, kind, "A & B", date=date, **args).to_docx()
        assert data == (directory / f"para{label}.docx").read_bytes()
        untouched(source, parts(io.BytesIO(data)), (MAIN,))
        reopened = ImportedDocument.open(data)
        block = reopened.body[1]
        mark, wrapper = block.mark_revision, block.inline_structure[0]
        assert mark.id != wrapper.id and mark.kind == wrapper.kind
        assert (mark.author, mark.date) == (wrapper.author, wrapper.date) == ("A & B", date)
        for action, suffix in [("accept","Accepted"),("reject","Rejected")]:
            data = reopened.preview_paragraph_revision(block.anchor, action).to_docx()
            assert data == (directory / f"para{label}{suffix}.docx").read_bytes()
            untouched(source, parts(io.BytesIO(data)), (MAIN,))
            expected = original_text.copy()
            if (kind == "insertBefore") == (action == "accept"):
                expected.insert(1," proposed ")
            assert [p.text for p in Document(io.BytesIO(data)).paragraphs] == expected
            if action == "accept":
                current = ImportedDocument.open(data)
    changed = imported.preview_tracked_bold(anchor, False, "A & B", date).to_docx()
    formatted = ImportedDocument.open(changed)
    run = formatted.body[0].inline_structure[0]
    assert run.property_revision.id and (run.property_revision.author, run.property_revision.date) == ("A & B", date)
    for name, data, bold in [("boldChange", changed, False),
                             ("boldAccepted", formatted.preview_revision(run.anchor, "accept").to_docx(), False),
                             ("boldRejected", formatted.preview_revision(run.anchor, "reject").to_docx(), True)]:
        assert (directory / f"{name}.docx").read_bytes() == data
        untouched(source, parts(io.BytesIO(data)), (MAIN,))
        assert Document(io.BytesIO(data)).paragraphs[0].runs[0].bold is bold
        assert (ImportedDocument.open(data).body[0].inline_structure[0].property_revision is not None) == (name == "boldChange")
    tracked = imported.preview_tracked_text(anchor, "replace", "A & B", text=" Revised ", date=date).to_docx()
    reopened = ImportedDocument.open(tracked)
    reviewed = reopened.preview_revision(reopened.body[0].inline_structure[1].anchor, "accept").to_docx()
    moved = imported.preview_tracked_move(anchor, imported.body[1].inline_structure[0].anchor, "Move1", "A & B", date).to_docx()
    pair = ImportedDocument.open(moved)
    accepted = pair.preview_revision(pair.body[0].inline_structure[1].anchor, "accept", paired=pair.body[1].inline_structure[2].anchor).to_docx()
    for name, data, expected in [("tracked", tracked, ["", "unset alignment"]),
                                 ("reviewed", reviewed, [" Revised ", "unset alignment"]),
                                 ("moved", moved, ["", "unset alignment"]),
                                 ("moveAccepted", accepted, ["", "unset alignmentSource bold"])]:
        assert (directory / f"{name}.docx").read_bytes() == data
        payloads = parts(io.BytesIO(data))
        untouched(source, payloads, (MAIN,))
        assert tree(xp(etree.fromstring(source[MAIN]), ".//w:fldSimple")[0]) == tree(xp(etree.fromstring(payloads[MAIN]), ".//w:fldSimple")[0])
        assert [p.text for p in Document(io.BytesIO(data)).paragraphs[:2]] == expected
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
            InlineEdit.set_toggle(target(0), "bold", None), InlineEdit.replace(target(1), link)])

    inline = asyncio.run(inline_batch())
    assert inline.inline_changes == [(0, 0), (1, 0), (4, 0)]
    assert inline.to_docx() == (directory / "inline.docx").read_bytes()
    untouched(source, parts(directory / "inline.docx"), (MAIN, MAIN_RELS))
    edited = Document(directory / "inline.docx").paragraphs
    assert edited[0].runs[0].bold is None
    assert imported.body[0].inline_structure[0].bold is True
    assert ImportedDocument.open(inline.to_docx()).body[0].inline_structure[0].bold is None
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
    physical = directory / "physicalTables.docx"
    old, new = parts(directory / "physical-table-source.docx"), parts(physical)
    untouched(old, new, (MAIN, "word/footer1.xml", "word/comments.xml"))
    assert new.keys() - old.keys() == {"word/_rels/footer1.xml.rels"}
    link, = xp(etree.fromstring(new["word/footer1.xml"]), ".//w:tbl/w:tr/w:tc/w:p/w:hyperlink/@r:id")
    assert relations(new,"word/footer1.xml")[link]["Target"] == "https://example.invalid/physical-cell"
    result = Document(physical)
    assert len(result.tables[0].rows) == 1
    inner = result.tables[0].cell(0,0).tables[0]
    assert [r.cells[0].text for r in inner.rows] == ["nested body",""]
    for section in result.sections:
        foot = section.footer.tables[0].cell(0,0).tables[0]
        assert foot.cell(0,0).text == "Footer link" and foot.autofit is False
    ct = next(iter(result.comments)).tables[0]
    assert ct.cell(0,0).text == "comment table" and ct.columns[0].width is None
    nodes = directory / "tableNodes.docx"
    untouched(parts(directory / "table-layout.docx"), parts(nodes), (MAIN,))
    edited = Document(nodes)
    assert len(edited.tables) == 1 and [c.text for c in edited.tables[0].rows[0].cells] == ["next A","cell"]
    assert edited.paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.RIGHT
    assert edited.tables[0]._tbl.xpath("w:tblPr/w:jc/@w:val") == ["center"]
    assert edited.tables[0]._tbl.xpath("w:tr/w:trPr/w:tblHeader/@w:val") == ["0"]
    assert edited.tables[0].cell(0,0)._tc.xpath("w:tcPr/w:tcMar/w:top/@w:w") == ["90"]
    staged_source = directory / "table-layout.docx"
    staged_output = directory / "structuralResources.docx"
    assert asyncio.run(structural_resources(staged_source)) == staged_output.read_bytes()
    before, after = parts(staged_source), parts(staged_output)
    assert after.keys() - before.keys() == {"docProps/custom.xml"}
    untouched(before, after, (MAIN, MAIN_RELS, "word/numbering.xml", "word/styles.xml",
                              TYPES, "_rels/.rels"))
    staged = Document(staged_output)
    assert len(staged.tables) == 1 and staged.tables[0].style.name == "Staged Table"
    hyperlink, = staged.tables[0].cell(0, 0).paragraphs[0].hyperlinks
    assert (hyperlink.text, hyperlink.url) == ("staged link", "https://example.invalid/staged-table")
    assert staged.paragraphs[0].text == "after both tables"
    assert staged.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["91"]
    untouched(body, parts(structure), (MAIN,))
    assert Document(structure).tables[1].autofit
    assert xp(etree.fromstring(parts(structure)[MAIN]), "w:body/w:tbl[2]/w:tblPr/w:tblLayout/@w:type") == ["autofit"]
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
