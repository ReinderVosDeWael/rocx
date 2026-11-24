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
from rocx.editor import ImportedDocument, ParagraphAlignmentEdit, ParagraphInsertion, ResourceEdit, StyleEdit


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


def test_conditional_region_roundtrip_and_independent_scalar_clear():
    regions = ("wholeTable", "firstRow", "lastRow", "firstCol", "lastCol",
               "band1Horz", "band2Horz", "band1Vert", "band2Vert",
               "nwCell", "neCell", "swCell", "seCell")
    doc = fixture()
    for region in regions:
        doc.styles.add_style("Area " + region, WD_STYLE_TYPE.TABLE)
    source = ImportedDocument.open(save(doc))
    def batch(doc, factory):
        return doc.preview([], style_edits=[factory(doc.style_anchor("Area" + r), r) for r in regions]).to_docx()
    fmt = d.TableSectionFormat(italic=True, bold=False, alignment="center", background=(1, 2, 3))
    first = ImportedDocument.open(batch(source, lambda a, r: StyleEdit.region(a, r, fmt)))
    second = ImportedDocument.open(batch(first, lambda a, r: StyleEdit.field(a, "i", None, region=r)))
    for r in regions:
        item = next(s for s in second.styles.styles if s.id == "Area" + r)
        wrapper, = map(etree.fromstring, item.property_xml)
        assert wrapper.get(qn("w:type")) == r
        assert wrapper.find(".//" + qn("w:i")) is None
        assert wrapper.find(".//" + qn("w:b")).get(qn("w:val")) == "0"
        assert wrapper.find(".//" + qn("w:shd")).get(qn("w:fill")) == "010203"
    cleared = Document(io.BytesIO(batch(second, lambda a, r: StyleEdit.region(a, r, None))))
    assert all(not cleared.styles["Area " + r].element.xpath("w:tblStylePr") for r in regions)


def test_numbering_style_and_reference_errors_leave_anchor_usable():
    doc = fixture()
    data = save(doc)
    imported = ImportedDocument.open(data)
    a = imported.style_anchor("MemoStyle")
    assert next(s for s in imported.styles.styles if s.id == "Heading1").linked_style == "Heading1Char"
    bad_edits = (
        StyleEdit.field(a, "next", "DefaultParagraphFont"),
        StyleEdit.field(a, "basedOn", "MemoStyle"),
        StyleEdit.add(d.NumberingStyleDefinition(name="Missing List", numbering_id=9876)),
    )
    for edit in bad_edits:
        with pytest.raises(ValueError):
            imported.preview([], style_edits=[edit])
    first = imported.preview([], style_edits=[StyleEdit.add(d.NumberingStyleDefinition(
        name="Valid List", numbering_id=1)), StyleEdit.field(a, "next", "MemoStyle")]).to_docx()
    reopened = ImportedDocument.open(first)
    assert next(s for s in reopened.styles.styles if s.id == "MemoStyle").next_style == "MemoStyle"
    with pytest.raises(ValueError, match="snapshot"):
        reopened.preview([], style_edits=[StyleEdit.field(a, "next", None)])
    with pytest.raises(ValueError, match="already owns"):
        reopened.preview([], style_edits=[StyleEdit.add(d.NumberingStyleDefinition(
            name="Another List", numbering_id=1))])
    assert parts(imported.preview([]).to_docx()) == parts(data)


def test_region_replacement_refuses_opaque_content_but_field_patch_retains_it():
    doc = fixture()
    table = doc.styles.add_style("Annotated Table", WD_STYLE_TYPE.TABLE)
    region = etree.SubElement(table.element, qn("w:tblStylePr"), {qn("w:type"): "firstRow"})
    group = etree.SubElement(region, qn("w:rPr"))
    etree.SubElement(group, qn("w:b"), {"{urn:extension}key": "keep"})
    imported = ImportedDocument.open(save(doc))
    a = imported.style_anchor("AnnotatedTable")
    for edit in (StyleEdit.region(a, "firstRow", None), StyleEdit.field(a, "b", None, region="firstRow")):
        with pytest.raises(ValueError):
            imported.preview([], style_edits=[edit])
    output = imported.preview([], style_edits=[StyleEdit.field(a, "b", "0", region="firstRow")]).to_docx()
    b, = Document(io.BytesIO(output)).styles[table.name].element.xpath(".//w:b")
    assert (b.get(qn("w:val")), b.get("{urn:extension}key")) == ("0", "keep")


def test_guarded_removal_of_unused_custom_style_with_region():
    imported = ImportedDocument.open(save(fixture()))
    added = imported.preview([], style_edits=[StyleEdit.add(d.TableStyleDefinition(
        name="Private Table", first_row=d.TableSectionFormat(italic=True, spacing_after=3)
    ))]).to_docx()
    reopened = ImportedDocument.open(added)
    result = reopened.preview([], style_edits=[StyleEdit.remove(reopened.style_anchor("PrivateTable"))]).to_docx()
    assert "Private Table" not in Document(io.BytesIO(result)).styles


def test_numbering_style_survives_content_revalidation_and_shared_resource_patch():
    data = save(fixture())
    doc = ImportedDocument.open(data)
    numbers = etree.fromstring(parts(data)["word/numbering.xml"])
    ns = {"w": qn("w:num").split("}")[0][1:]}
    aid = int(numbers.xpath('./w:num[@w:numId="1"]/w:abstractNumId/@w:val', namespaces=ns)[0])
    output = doc.preview([ParagraphAlignmentEdit(doc.body[0].anchor, "right")], style_edits=[
        StyleEdit.remove(doc.style_anchor("MemoStyle")),
        StyleEdit.add(d.NumberingStyleDefinition(name="Ordinary List", numbering_id=1)),
    ], resources=[ResourceEdit.abstract_start(aid, 0, 7)]).to_docx()
    numbers = etree.fromstring(parts(output)["word/numbering.xml"])
    abstract, = numbers.xpath(f'./w:abstractNum[@w:abstractNumId="{aid}"]', namespaces=ns)
    assert abstract.xpath("w:styleLink/@w:val", namespaces=ns) == ["OrdinaryList"]
    assert abstract.xpath('w:lvl[@w:ilvl="0"]/w:start/@w:val', namespaces=ns) == ["7"]


async def test_staged_style_catalog_is_shared_by_body_and_defaults_edits():
    source = save(fixture())
    imported = ImportedDocument.open(source)
    target = imported.body[0].anchor
    new_style = d.ParagraphStyleDefinition(name="Review Summary")
    insertion = ParagraphInsertion(target, await d.Paragraph(text="Summary", style=new_style.name).resolve())
    defaults = [ResourceEdit.default_alignment("right")]
    output = imported.preview([], paragraph_insertions=[insertion], resources=defaults,
                              style_edits=[StyleEdit.add(new_style)]).to_docx()
    readback = Document(io.BytesIO(output))
    assert readback.paragraphs[1].text == "Summary"
    assert readback.paragraphs[1].style.name == "Review Summary"
    assert ImportedDocument.open(output).default_alignment == "right"
    before, after = parts(source), parts(output)
    assert {k for k in before.keys() | after.keys() if before.get(k) != after.get(k)} == {
        "word/styles.xml", "word/document.xml"}

    memo = ParagraphInsertion(target, await d.Paragraph(text="Use", style="Memo Style").resolve())
    replace = StyleEdit.replace(imported.style_anchor("MemoStyle"), d.ParagraphStyleDefinition(name="Memo Style"))
    with pytest.raises(ValueError, match="references"):
        imported.preview([], paragraph_insertions=[memo], resources=defaults, style_edits=[replace])
    assert imported.default_alignment is None
    assert parts(imported.preview([]).to_docx()) == before
