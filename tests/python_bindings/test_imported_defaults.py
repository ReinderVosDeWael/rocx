"""Independent python-docx defaults, including a relocated settings part."""
from io import BytesIO
from zipfile import ZipFile

import docx
import pytest
from docx.oxml.ns import qn
from lxml import etree
from rocx.editor import ImportedDocument, ParagraphAlignmentEdit, ResourceEdit


def pack(parts):
    out = BytesIO()
    with ZipFile(out, "w") as archive:
        for path, data in parts.items():
            archive.writestr(path, data)
    return out.getvalue()


def parts(data):
    with ZipFile(BytesIO(data)) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def fixture():
    source = docx.Document()
    source.add_paragraph().add_run("Leave direct formatting alone").bold = True
    original = BytesIO()
    source.save(original)
    package = parts(original.getvalue())
    package["word/options/preferences.xml"] = package.pop("word/settings.xml")
    for filename in ["word/_rels/document.xml.rels", "[Content_Types].xml"]:
        package[filename] = package[filename].replace(b"settings.xml\"", b"options/preferences.xml\"")
    styles = package["word/styles.xml"]
    assert b"<w:rPrDefault><w:rPr>" in styles
    package["word/styles.xml"] = styles.replace(
        b"<w:rPrDefault><w:rPr>", b'<w:rPrDefault><w:rPr><w:b/><!--opaque: stay here-->')
    return package


def preview(doc, values):
    return doc.preview([], resources=[ResourceEdit.document_default(k, v) for k, v in values]).to_docx()


def test_direct_and_absent_defaults_are_independent_of_run_formatting():
    original = fixture()
    source = ImportedDocument.open(pack(original))
    before = source.document_defaults
    assert before["bold"] is True and "mirrorMargins" not in before
    assert before["fontAsciiTheme"] == "minorHAnsi" and before["sizeHalfPoints"] == 22
    changes = {"bold": False, "sizeHalfPoints": 23, "fontEastAsia": "Sample",
               "languageEastAsia": None, "runColor": "123AbC", "colorTint": "A1", "colorTheme": "accent3",
               "justification": "end", "afterTwips": None, "beforeAutoSpacing": False,
               "tabStops": [(-120, "clear", None), (720, "right", "dot")],
               "mirrorMargins": False, "gutterAtTop": True, "footnoteStart": 0,
               "endnotePosition": "docEnd", "endnoteRestart": "eachSect"}
    data = preview(source, changes.items())
    edited = ImportedDocument.open(data)
    assert edited.document_defaults == {k: v for k, v in (before | changes).items() if v is not None}
    assert parts(preview(edited, reversed(list(changes.items())))) == parts(data)
    assert docx.Document(BytesIO(data)).paragraphs[0].runs[0].bold is True
    output = parts(data)
    assert original.keys() == output.keys()
    assert {n for n in original if original[n] != output[n]} == {"word/styles.xml", "word/options/preferences.xml"}
    assert b"<!--opaque: stay here-->" in output["word/styles.xml"]
    defaults = etree.fromstring(output["word/styles.xml"]).find(qn("w:docDefaults"))
    assert defaults.find(".//" + qn("w:b")).get(qn("w:val")) == "0"
    assert [t.get(qn("w:pos")) for t in defaults.iter(qn("w:tab"))] == ["-120", "720"]
    cleared = ImportedDocument.open(preview(edited, [(key, None) for key in changes]))
    assert not (changes.keys() & cleared.document_defaults.keys())
    assert source.document_defaults == before
    with pytest.raises(ValueError, match="snapshot"):
        edited.preview([ParagraphAlignmentEdit(source.body[0].anchor, "left")],
                       resources=[ResourceEdit.document_default("bold", None)])


@pytest.mark.parametrize("changes", [
    [("mirrorMargins", True), ("mirrorMargins", None)],
    [("runColor", "FF0033"), ("colorTint", "invalid")],
    [("bold", 1)], [("lineRule", "unknown")], [("endnoteStart", -2)],
    [("tabStops", [(0, "left", None), (0, "right", None)])],
    [("colorTheme", "accent3")],  # needs an explicit fallback RGB or "auto"
])
def test_invalid_batch_does_not_consume_snapshot(changes):
    doc = ImportedDocument.open(pack(fixture()))
    with pytest.raises(ValueError):
        preview(doc, changes)
    assert ImportedDocument.open(preview(doc, [("bold", False)])).document_defaults["bold"] is False


def test_annotated_or_ambiguous_property_cannot_be_discarded():
    original = fixture()
    original["word/styles.xml"] = original["word/styles.xml"].replace(
        b"<w:b/>", b'<w:b><!--opaque annotation--></w:b>', 1)
    doc = ImportedDocument.open(pack(original))
    with pytest.raises(ValueError, match="annotated"):
        preview(doc, [("bold", None)])
    with pytest.raises(ValueError, match="duplicate"):
        doc.preview([], resources=[ResourceEdit.default_alignment(None),
                                   ResourceEdit.document_default("justification", "both")])
    original["word/styles.xml"] = original["word/styles.xml"].replace(b"<w:b>", b"<w:b/><w:b>", 1)
    doc = ImportedDocument.open(pack(original))
    with pytest.raises(ValueError, match="ambiguous"):
        _ = doc.document_defaults
    with pytest.raises(ValueError, match="ambiguous"):
        preview(doc, [("bold", None)])
    assert parts(doc.preview([]).to_docx()) == original


def test_create_only_absent_default_parts_with_collision_safe_relationships():
    original = fixture()
    relpath = "word/_rels/document.xml.rels"
    rels = etree.fromstring(original[relpath])
    for rel in list(rels):
        if rel.get("Type").rsplit("/", 1)[-1] in ("styles", "settings"):
            rels.remove(rel)
    original[relpath] = etree.tostring(rels)
    # Orphan payloads still belong to the source and must not be adopted or replaced.
    original["word/ROCX-STYLES1.XML"] = b"<opaque/>"
    doc = ImportedDocument.open(pack(original))
    assert doc.document_defaults == {}
    assert parts(preview(doc, [("bold", None), ("footnoteStart", None)])) == original
    data = doc.preview([], resources=[ResourceEdit.default_alignment("center"),
                                     ResourceEdit.document_default("trackRevisions", False)]).to_docx()
    result = parts(data)
    assert result.keys() - original.keys() == {"word/rocx-styles2.xml", "word/rocx-settings1.xml"}
    assert {k for k in original if original[k] != result[k]} == {relpath, "[Content_Types].xml"}
    assert ImportedDocument.open(data).document_defaults == {"justification": "center", "trackRevisions": False}
    assert docx.Document(BytesIO(data)).paragraphs[0].runs[0].bold
