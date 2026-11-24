"""Combined catalog/numbering/defaults/metadata transaction on an independent package."""
from io import BytesIO

import docx
from lxml import etree
import pytest
from rocx import declarative as d
from rocx.editor import ImportedDocument, ResourceEdit as R, StyleEdit
from test_resource_fields import pack, unpack


def fixture():
    word = docx.Document()
    word.add_paragraph("Formatting on this run stays direct").runs[0].bold = True
    buf = BytesIO()
    word.save(buf)
    parts = unpack(buf.getvalue())
    # Remove each original relationship and its payload. The conventional
    # metadata locations below then hold real, relocated assets of other types.
    removed = {"docProps/core.xml", "docProps/app.xml", "word/numbering.xml", "word/settings.xml", "word/styles.xml"}
    types = etree.fromstring(parts["[Content_Types].xml"])
    for override in list(types):
        if override.get("PartName", "")[1:] in removed:
            types.remove(override)
    for path in removed:
        del parts[path]
    for owner, kinds in [("_rels/.rels", {"core-properties", "extended-properties"}),
                         ("word/_rels/document.xml.rels", {"numbering", "styles", "settings"})]:
        rels = etree.fromstring(parts[owner])
        for rel in list(rels):
            if rel.get("Type").rsplit("/", 1)[-1] in kinds:
                rels.remove(rel)
        parts[owner] = etree.tostring(rels)
    for old, new, owner_rels, old_target, new_target in [
        ("word/theme/theme1.xml", "docProps/app.xml", "word/_rels/document.xml.rels", "theme/theme1.xml", "../docProps/app.xml"),
        ("customXml/itemProps1.xml", "docProps/core.xml", "customXml/_rels/item1.xml.rels", "itemProps1.xml", "../docProps/core.xml"),
    ]:
        parts[new] = parts.pop(old)
        types.find(f"{{*}}Override[@PartName='/{old}']").set("PartName", "/" + new)
        edges = etree.fromstring(parts[owner_rels])
        edges.find(f"{{*}}Relationship[@Target='{old_target}']").set("Target", new_target)
        parts[owner_rels] = etree.tostring(edges)
    parts["[Content_Types].xml"] = etree.tostring(types)
    return parts


def rels(parts, owner):
    return {(n.get("Type").rsplit("/", 1)[-1], n.get("Id"), n.get("Target"))
            for n in etree.fromstring(parts[owner])}


def mixed_edits():
    styles = [StyleEdit.add(d.NumberingStyleDefinition("New batch list", 602)),
              StyleEdit.add(d.ParagraphStyleDefinition(name="Disposable"))]
    edits = [R.document_default("bold", False), R.metadata("title", ""),
             R.add_instance(603, 601), R.document_default("trackRevisions", False),
             R.metadata("created", "2020-02-29T00:00:00Z"), R.custom_property("Signed", False),
             R.metadata("manager", "Reviewer"), R.document_default("justification", "end"),
             R.add_abstract(601, [d.NumberingLevel()]), R.add_instance(602, 601)]
    return styles, edits


def test_mixed_creation_rollback_reciprocal_links_and_final_guards():
    original = fixture()
    source = ImportedDocument.open(pack(original))
    styles, edits = mixed_edits()
    for conflict in [R.default_alignment(None), R.core_title(None), R.custom_property("Signed", None),
                     R.remove_instance(602), R.document_default("bold", None), R.metadata("manager", ""),
                     R.abstract_start(601, 0, None)]:
        with pytest.raises(ValueError):
            source.preview([], resources=[*edits, conflict], style_edits=styles)
        assert source.styles is None and source.metadata_field("created") is None
    assert unpack(source.preview([], resources=[R.core_title(None), R.custom_property("Signed", None),
                                                R.document_default("bold", None)]).to_docx()) == original
    data = source.preview([], resources=edits, style_edits=styles).to_docx()
    # Input order need not match XML order: the style owns instance 602 in
    # either batch even when its abstract declaration comes last.
    reverse = docx.Document(BytesIO(source.preview([], resources=list(reversed(edits)),
                                                  style_edits=styles).to_docx()))
    assert reverse.styles["New batch list"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["602"]
    result = unpack(data)
    assert result.keys() - original.keys() == {"docProps/core1.xml", "docProps/app1.xml", "docProps/custom.xml",
        "word/rocx-styles1.xml", "word/rocx-settings1.xml", "word/rocx-numbering1.xml"}
    assert {k for k in original if original[k] != result[k]} == {
        "_rels/.rels", "word/_rels/document.xml.rels", "[Content_Types].xml"}
    for owner, added in [("_rels/.rels", {"core-properties", "extended-properties", "custom-properties"}),
                         ("word/_rels/document.xml.rels", {"styles", "settings", "numbering"})]:
        assert rels(original, owner) <= rels(result, owner)
        new = rels(result, owner) - rels(original, owner)
        assert len(new) == len(added) and {k for k, _, _ in new} == added
    readback = docx.Document(BytesIO(data))
    assert readback.core_properties.created.year == 2020 and readback.paragraphs[0].runs[0].bold is True
    assert readback.styles["New batch list"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["602"]
    assert readback.part.numbering_part.element.xpath("w:abstractNum[@w:abstractNumId='601']/w:styleLink/@w:val") == ["Newbatchlist"]
    doc = ImportedDocument.open(data)
    assert doc.metadata_field("title") == doc.core_title == "" and doc.custom_property("Signed") is False
    assert doc.document_defaults == {"bold": False, "trackRevisions": False, "justification": "end"}
    with pytest.raises(ValueError, match="references"):
        doc.preview([], resources=[R.remove_instance(602), R.metadata("title", "uncommitted")])
    with pytest.raises(ValueError):
        doc.preview([], resources=[R.remove_abstract(601)], style_edits=[StyleEdit.remove(doc.style_anchor("Newbatchlist"))])
    final = unpack(doc.preview([], resources=[R.remove_instance(603)],
                              style_edits=[StyleEdit.remove(doc.style_anchor("Disposable"))]).to_docx())
    assert {k for k in result if final[k] != result[k]} == {"word/rocx-styles1.xml", "word/rocx-numbering1.xml"}
