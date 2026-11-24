"""Direct OOXML patches, missing-part allocation, and provable style migrations."""
import io

import pytest
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml.ns import qn
from lxml import etree
from rocx import declarative as d
from rocx.editor import ImportedDocument, StyleEdit as S

from test_imported_style_edits import fixture, package, parts, save


def test_composite_region_patch_retains_opaque_children_attributes_and_other_tabs():
    doc = fixture()
    t = doc.styles.add_style("Format Matrix", WD_STYLE_TYPE.TABLE)
    region = etree.SubElement(t.element, qn("w:tblStylePr"), {qn("w:type"): "firstRow"})
    group = etree.SubElement(region, qn("w:pPr"))
    tabs = etree.SubElement(group, qn("w:tabs"))
    def stop(pos):
        return etree.SubElement(tabs, qn("w:tab"), {qn("w:pos"):pos, qn("w:val"):"center"})
    stop("-120").set("{urn:test}label", "preserved")
    stop("720")
    spacing = etree.SubElement(group, qn("w:spacing"), {qn("w:after"):"032", "{urn:test}slot":"preserved"})
    spacing.append(etree.Comment("inside spacing"))
    source = ImportedDocument.open(save(doc))
    a = source.style_anchor("FormatMatrix")
    def patch(k, v):
        return S.field(a, k, v, region="firstRow")
    out = source.preview([], style_edits=[
        patch("spacing.before", "240"), patch("spacing.line", "320"), patch("spacing.lineRule", "atLeast"),
        patch("tab.-120.leader", "dot"), patch("tab.720.val", None), patch("tab.360.val", "decimal"),
        patch("shd.fill", "123456"), patch("shd.themeFill", "accent4"), patch("ind.left", "-30"),
        patch("rFonts.ascii", "Arial"), patch("lang.eastAsia", "ja-JP"),
        patch("u.val", "dashLong"), patch("color.themeTint", "AA"), patch("szCs.val", "22"),
    ]).to_docx()
    read = Document(io.BytesIO(out)).styles[t.name].element
    n, = read.xpath(".//w:spacing")
    assert (n.get(qn("w:before")),n.get(qn("w:after")),n.get("{urn:test}slot"),n[0].text) == (
        "240","032","preserved","inside spacing")
    assert [(n.get(qn("w:pos")),n.get(qn("w:leader")),n.get("{urn:test}label")) for n in read.xpath(".//w:tab")] == [
        ("-120","dot","preserved"),("360",None,None)]
    assert read.xpath(".//w:shd/@w:val") == ["clear"] and read.xpath(".//w:color/@w:val") == ["auto"]
    opened = ImportedDocument.open(out)
    b = opened.style_anchor("FormatMatrix")
    cleared = opened.preview([],style_edits=[S.field(b,"spacing.after",None,region="firstRow")]).to_docx()
    n, = Document(io.BytesIO(cleared)).styles[t.name].element.xpath(".//w:spacing")
    assert n.get(qn("w:after")) is None and n.get("{urn:test}slot") == "preserved" and n[0].text=="inside spacing"
    for ops in ([patch("tab.-120.val",None)], [patch("spacing.before","1"),patch("spacing.before",None)],
                [patch("shd.fill","GG0000")],[patch("tab.NaN.val","left")]):
        with pytest.raises(ValueError):
            source.preview([],style_edits=ops)


def test_create_new_catalog_beside_relocated_main_without_adopting_orphan():
    before = parts(save(fixture()))
    before["reports/main.xml"] = before.pop("word/document.xml")
    root = etree.fromstring(before.pop("word/_rels/document.xml.rels"))
    for r in list(root):
        if r.get("Type").endswith("/styles"):
            root.remove(r)
        elif r.get("TargetMode") != "External":
            r.set("Target", "../word/"+r.get("Target") if not r.get("Target").startswith("../") else r.get("Target"))
    rels = "reports/_rels/main.xml.rels"
    before[rels] = etree.tostring(root)
    before["_rels/.rels"] = before["_rels/.rels"].replace(b'word/document.xml',b'reports/main.xml')
    before["[Content_Types].xml"] = before["[Content_Types].xml"].replace(b'/word/document.xml',b'/reports/main.xml')
    before["reports/STYLES.XML"] = b"<unrelated/>"
    source = ImportedDocument.open(package(before))
    assert source.styles is None
    with pytest.raises(KeyError, match="unknown style"):
        source.preview([],style_edits=[S.add(d.ParagraphStyleDefinition(name="No",base_style="Missing"))])
    output = source.preview([],style_edits=[S.add(d.ParagraphStyleDefinition(name="Child",base_style="Root")),
                                            S.add(d.ParagraphStyleDefinition(name="Root"))]).to_docx()
    catalog = ImportedDocument.open(output).styles
    assert catalog.part_name == "reports/styles1.xml"
    assert Document(io.BytesIO(output)).styles["Child"].base_style.name == "Root"
    after = parts(output)
    assert {k for k in after.keys()|before.keys() if after.get(k)!=before.get(k)} == {rels,"[Content_Types].xml",catalog.part_name}
    assert source.styles is None


@pytest.mark.parametrize("guard", ["cycle","type","field","alias","protected","unknown","duplicate"])
def test_retarget_fails_atomically_for_unprovable_migrations(guard):
    doc = fixture()
    old = doc.styles["Memo Style"]
    new = doc.styles.add_style("Destination",WD_STYLE_TYPE.CHARACTER if guard=="type" else WD_STYLE_TYPE.PARAGRAPH)
    doc.paragraphs[0].style = old
    if guard=="cycle":
        new.base_style = old
    elif guard=="field":
        etree.SubElement(doc.paragraphs[0]._p,qn("w:fldSimple"),{qn("w:instr"):' STYLEREF "Memo Style" '})
    elif guard=="alias":
        etree.SubElement(old.element,qn("w:aliases"),{qn("w:val"):"Stray"})
    elif guard=="protected":
        sdt=etree.SubElement(doc.element.body,qn("w:sdt"))
        content=etree.SubElement(sdt,qn("w:sdtContent"))
        content.append(doc.paragraphs[0]._p)
    elif guard=="unknown":
        wrapper=etree.SubElement(doc.element.body,qn("w:futureWrapper"))
        wrapper.append(doc.paragraphs[0]._p)
    elif guard=="duplicate":
        etree.SubElement(doc.paragraphs[0]._p.get_or_add_pPr(),qn("w:pStyle"),{qn("w:val"):old.style_id})
    data=save(doc)
    src=ImportedDocument.open(data)
    with pytest.raises(ValueError):
        src.preview([],style_edits=[S.retarget(src.style_anchor(old.style_id),src.style_anchor(new.style_id))])
    assert parts(src.preview([]).to_docx()) == parts(data)
