"""Independent DOCX fixtures for style identity, preservation and usage guards."""
import io
from zipfile import ZipFile

import pytest
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from lxml import etree

from rocx import declarative as d
from rocx.editor import ImportedDocument, ParagraphAlignmentEdit, StyleEdit


def parts(data):
    with ZipFile(io.BytesIO(data)) as z:
        return {n: z.read(n) for n in z.namelist()}


def package(entries):
    buf = io.BytesIO()
    with ZipFile(buf, "w") as z:
        for name, data in entries.items():
            z.writestr(name, data)
    return buf.getvalue()


def fixture():
    doc = Document()
    doc.add_paragraph("Body")
    doc.styles.add_style("Memo Style", WD_STYLE_TYPE.PARAGRAPH).font.bold = True
    return doc


def save(doc):
    f = io.BytesIO()
    doc.save(f)
    return f.getvalue()


def test_relocated_style_patch_preserves_opaque_nodes_and_noop_payload():
    doc = fixture()
    style = doc.styles["Memo Style"].element
    flag = style.find(".//" + qn("w:b"))
    flag.set("{urn:memo}flag", "kept")
    etree.SubElement(flag, "{urn:memo}extension").text = "opaque"
    style.append(etree.Comment("after properties"))
    before = parts(save(doc))
    target = "resources/selected-styles.xml"
    before[target] = before.pop("word/styles.xml")
    rels = "word/_rels/document.xml.rels"
    before[rels] = before[rels].replace(b'Target="styles.xml"', b'Target="../resources/selected-styles.xml"')
    before["[Content_Types].xml"] = before["[Content_Types].xml"].replace(b"/word/styles.xml", b"/resources/selected-styles.xml")
    imported = ImportedDocument.open(package(before))
    anchor = imported.style_anchor("MemoStyle")
    assert parts(imported.preview([], style_edits=[StyleEdit.patch(anchor, bold=True)]).to_docx()) == before
    with pytest.raises(ValueError, match="unsupported markup"):
        imported.preview([], style_edits=[StyleEdit.remove(anchor)])
    output = imported.preview([], style_edits=[StyleEdit.patch(anchor, bold=False, italic=True)]).to_docx()
    after = parts(output)
    assert {p for p in before.keys() | after.keys() if before.get(p) != after.get(p)} == {target}
    readback = Document(io.BytesIO(output)).styles["Memo Style"]
    assert readback.font.bold is False and readback.font.italic is True
    flag = readback.element.find(".//" + qn("w:b"))
    assert flag.get("{urn:memo}flag") == "kept" and flag[0].text == "opaque"
    assert b"<!--after properties-->" in after[target]
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(output).preview([], style_edits=[StyleEdit.patch(anchor, bold=True)])


@pytest.mark.parametrize("reference", ["body", "header", "base", "numbering", "field", "unknown"])
def test_cross_part_usage_rejects_replace_and_remove_atomically(reference):
    doc = fixture()
    if reference == "body":
        doc.paragraphs[0].style = "Memo Style"
    elif reference == "header":
        doc.sections[0].header.paragraphs[0].style = "Memo Style"
    elif reference == "base":
        doc.styles.add_style("Dependent", WD_STYLE_TYPE.PARAGRAPH).base_style = doc.styles["Memo Style"]
    elif reference == "numbering":
        node = OxmlElement("w:pStyle")
        node.set(qn("w:val"), "MemoStyle")
        doc.part.numbering_part.element.xpath(".//w:lvl")[0].append(node)
    elif reference == "field":
        node = OxmlElement("w:instrText")
        node.text = ' STYLEREF "Memo Style" '
        doc.paragraphs[0].add_run()._r.append(node)
    else:
        etree.SubElement(doc.paragraphs[0]._p, "{urn:opaque}user")
    source = save(doc)
    imported = ImportedDocument.open(source)
    target = imported.style_anchor("MemoStyle")
    alignment = ParagraphAlignmentEdit(imported.body[0].anchor, "center")
    for edit in (StyleEdit.remove(target),
                 StyleEdit.replace(target, d.ParagraphStyleDefinition(name="Memo Style"))):
        with pytest.raises(ValueError, match="references"):
            imported.preview([alignment], style_edits=[StyleEdit.add(
                d.CharacterStyleDefinition(name="Staged")), edit])
        assert parts(imported.preview([]).to_docx()) == parts(source)
    assert imported.preview([alignment]).changes[0].after == "center"


def test_duplicate_type_and_inheritance_guards():
    doc = fixture()
    source = save(doc)
    imported = ImportedDocument.open(source)
    add = lambda name, base=None: StyleEdit.add(d.ParagraphStyleDefinition(name=name, base_style=base))
    for edits, error in [
        ([add("Memo Style")], "duplicate"),
        ([add("MemoStyle")], "duplicate"),
        ([add("Forward A", "Forward B"), add("Forward B", "Forward A")], "cycle"),
        ([add("Wrong Base", "DefaultParagraphFont")], "paragraph style"),
        ([StyleEdit.replace(imported.style_anchor("MemoStyle"), d.CharacterStyleDefinition(name="Memo Style"))], "type"),
        ([StyleEdit.patch(imported.style_anchor("TableNormal"), italic=False)], "paragraph or character"),
    ]:
        with pytest.raises(ValueError, match=error):
            imported.preview([], style_edits=edits)
    output = imported.preview([], style_edits=[add("Forward A", "Forward B"), add("Forward B")]).to_docx()
    assert Document(io.BytesIO(output)).styles["Forward A"].base_style.name == "Forward B"
    duplicate = doc.styles.add_style("Second Memo", WD_STYLE_TYPE.CHARACTER)
    duplicate.element.set(qn("w:styleId"), "MemoStyle")
    ambiguous = ImportedDocument.open(save(doc))
    with pytest.raises(ValueError, match="duplicate style ID"):
        ambiguous.style_anchor("MemoStyle")
    with pytest.raises(ValueError, match="duplicate style ID"):
        ambiguous.preview([], style_edits=[add("New Style")])
