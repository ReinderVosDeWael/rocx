"""Reference-safe edits against a python-docx package with relocated numbering."""
from io import BytesIO

import docx
import pytest
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from lxml import etree

from rocx.declarative import NumberingLevel
from rocx.editor import BlockEdit, ImportedDocument, ResourceEdit as R
from test_resource_fields import pack, unpack

NUMBERS = "resources/lists.xml"


def fixture():
    doc = docx.Document()
    doc.add_paragraph("number this")
    doc.add_paragraph("unrelated")
    buf = BytesIO()
    doc.save(buf)
    parts = unpack(buf.getvalue())
    parts[NUMBERS] = parts.pop("word/numbering.xml")
    parts["word/_rels/document.xml.rels"] = parts["word/_rels/document.xml.rels"].replace(
        b'Target="numbering.xml"', b'Target="../resources/lists.xml"')
    parts["[Content_Types].xml"] = parts["[Content_Types].xml"].replace(b"/word/numbering.xml", b"/resources/lists.xml")
    return parts


def additions(doc):
    return [R.instance_start(90, 1, 6), R.paragraph_numbering(doc.body[0].anchor, (90, 1)),
            R.instance_level(90, 1, NumberingLevel(level=1, start=3, format="lowerRoman")),
            R.add_instance(90, 80), R.add_abstract(80, [NumberingLevel(), NumberingLevel(level=1, text="%1.%2")])]


def test_relocated_numbering_lifecycle_and_stale_anchors():
    parts = fixture()
    doc = ImportedDocument.open(pack(parts))
    old = doc.body[0].anchor
    assert doc.paragraph_numbering(old) is None
    edits = additions(doc)
    for conflict in ([R.remove_abstract(80)], [R.add_instance(90, 80)], [R.instance_start(90, 2, 5)],
                     [R.paragraph_numbering(old, None)], [R.instance_level(90, 1, NumberingLevel())]):
        with pytest.raises(ValueError):
            doc.preview([], resources=[*edits, *conflict])
    with pytest.raises(ValueError):
        doc.preview([], block_edits=[BlockEdit.remove(old)], resources=edits)
    data = doc.preview([], resources=edits).to_docx()
    after = unpack(data)
    assert {k for k in parts.keys() | after.keys() if parts.get(k) != after.get(k)} == {NUMBERS, "word/document.xml"}
    readback = docx.Document(BytesIO(data))
    assert readback.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:ilvl/@w:val") == ["1"]
    nums = readback.part.numbering_part.element
    owner, = nums.xpath("w:num[@w:numId='90']/w:lvlOverride[@w:ilvl='1']")
    assert owner.xpath("w:lvl/w:numFmt/@w:val") == ["lowerRoman"]
    assert owner.xpath("w:startOverride/@w:val") == ["6"]
    edited = ImportedDocument.open(data)
    instance, = (n for n in edited.numbering_instances if n.num_id == 90)
    assert (instance.abstract_id, instance.start_overrides, instance.replacement_levels) == (80, [(1, 6)], [1])
    target = edited.body[0].anchor
    assert edited.paragraph_numbering(target) == (90, 1)
    for op in (lambda: edited.paragraph_numbering(old), lambda: edited.preview([], resources=edits)):
        with pytest.raises(ValueError, match="snapshot"):
            op()
    for removal in (R.remove_instance(90), R.remove_abstract(80)):
        with pytest.raises(ValueError, match="references"):
            edited.preview([], resources=[removal])
    cleared = ImportedDocument.open(edited.preview([], resources=[R.instance_level(90, 1, None)]).to_docx())
    cleared_instance, = (n for n in cleared.numbering_instances if n.num_id == 90)
    assert (cleared_instance.start_overrides, cleared_instance.replacement_levels) == ([(1, 6)], [])
    final = edited.preview([], resources=[R.remove_abstract(80), R.remove_instance(90),
                                          R.paragraph_numbering(target, None)]).to_docx()
    final_doc = docx.Document(BytesIO(final))
    assert not final_doc.paragraphs[0]._p.xpath("w:pPr/w:numPr")
    assert not final_doc.part.numbering_part.element.xpath("w:abstractNum[@w:abstractNumId='80'] | w:num[@w:numId='90']")


@pytest.mark.parametrize("context", ["header", "style", "extension"])
def test_guard_cross_part_uses_even_after_clearing_body(context):
    doc = ImportedDocument.open(pack(fixture()))
    data = doc.preview([], resources=additions(doc)).to_docx()
    word = docx.Document(BytesIO(data))
    if context == "header":
        ppr = word.sections[0].header.paragraphs[0]._p.get_or_add_pPr()
    elif context == "style":
        ppr = word.styles["Normal"].element.get_or_add_pPr()
    else:
        etree.SubElement(word.paragraphs[1]._p, "{urn:opaque}reference")
    if context != "extension":
        num = OxmlElement("w:numId")
        num.set(qn("w:val"), "0090")
        ppr.get_or_add_numPr().append(num)
    buf = BytesIO()
    word.save(buf)
    edited = ImportedDocument.open(buf.getvalue())
    with pytest.raises(ValueError, match="references"):
        edited.preview([], resources=[R.paragraph_numbering(edited.body[0].anchor, None), R.remove_instance(90)])
    assert edited.paragraph_numbering(edited.body[0].anchor) == (90, 1)


def test_override_start_clear_retains_opaque_replacement_and_noop_bytes():
    parts = fixture()
    doc = ImportedDocument.open(pack(parts))
    parts = unpack(doc.preview([], resources=additions(doc)).to_docx())
    root = etree.fromstring(parts[NUMBERS])
    ns = {"w": qn("w:lvl").split("}")[0][1:]}
    owner, = root.xpath("w:num[@w:numId='90']/w:lvlOverride", namespaces=ns)
    owner.find(qn("w:startOverride")).set(qn("w:val"), "006")
    owner.find(qn("w:lvl")).append(etree.Comment("keep replacement"))
    parts[NUMBERS] = etree.tostring(root)
    doc = ImportedDocument.open(pack(parts))
    assert unpack(doc.preview([], resources=[R.instance_start(90, 1, 6)]).to_docx()) == parts
    with pytest.raises(ValueError, match="rich"):
        doc.preview([], resources=[R.instance_level(90, 1, None)])
    after = unpack(doc.preview([], resources=[R.instance_start(90, 1, None)]).to_docx())
    assert b"<!--keep replacement-->" in after[NUMBERS]
    assert not etree.fromstring(after[NUMBERS]).xpath("w:num[@w:numId='90']//w:startOverride", namespaces=ns)
