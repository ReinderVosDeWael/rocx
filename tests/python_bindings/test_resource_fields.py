"""Direct imported properties on a python-docx file with a relocated core part."""
from io import BytesIO
from zipfile import ZipFile

import docx
import pytest
from lxml import etree

from rocx.editor import ImportedDocument, ParagraphAlignmentEdit, ResourceEdit


def pack(parts):
    out = BytesIO()
    with ZipFile(out, "w") as archive:
        for name, payload in parts.items():
            archive.writestr(name, payload)
    return out.getvalue()


def unpack(data):
    with ZipFile(BytesIO(data)) as archive:
        return {n: archive.read(n) for n in archive.namelist()}


def fixture():
    source = docx.Document()
    source.core_properties.title = "original"
    source.add_paragraph("shared numbering", style="List Number")
    buf = BytesIO()
    source.save(buf)
    parts = unpack(buf.getvalue())
    parts["properties/info.xml"] = parts.pop("docProps/core.xml").replace(
        b"</cp:coreProperties>", b'<!--retain--><?sample opaque?></cp:coreProperties>')
    for name in ("_rels/.rels", "[Content_Types].xml"):
        parts[name] = parts[name].replace(b"docProps/core.xml", b"properties/info.xml")
    return parts


def test_resource_edits_roundtrip_without_rewriting_numbering_references():
    parts = fixture()
    source = ImportedDocument.open(pack(parts))
    level = source.abstract_starts[0]
    assert (source.core_title, source.default_alignment, level.start) == ("original", None, 1)
    edit = ResourceEdit.abstract_start
    def preview(doc, *edits):
        return doc.preview([], resources=edits).to_docx()

    data = preview(source, ResourceEdit.core_title(""), ResourceEdit.default_alignment("justifyMedium"),
                   edit(level.abstract_id, level.level, 0))
    result = ImportedDocument.open(data)
    assert (result.core_title, result.default_alignment, result.abstract_starts[0].start) == ("", "justifyMedium", 0)
    output = unpack(data)
    assert set(parts) == set(output)
    assert {p for p in parts if parts[p] != output[p]} == {
        "properties/info.xml", "word/styles.xml", "word/numbering.xml"}
    assert b"<!--retain--><?sample opaque?>" in output["properties/info.xml"]
    assert docx.Document(BytesIO(data)).paragraphs[0].style.name == "List Number"
    w = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
    def identities(package):
        numbers = etree.fromstring(package["word/numbering.xml"])
        return [(n.get(w+"numId"), n.find(w+"abstractNumId").get(w+"val")) for n in numbers.findall(w+"num")]
    assert identities(output) == identities(parts)
    assert unpack(preview(result, ResourceEdit.core_title(""), edit(level.abstract_id, level.level, 0))) == output

    cleared = ImportedDocument.open(preview(result, ResourceEdit.core_title(None),
        ResourceEdit.default_alignment(None), edit(level.abstract_id, level.level, None)))
    assert (cleared.core_title, cleared.default_alignment, cleared.abstract_starts[0].start) == (None, None, None)
    for invalid in ([edit(9001, 0, 4)], [edit(level.abstract_id, level.level, 1)] * 2,
                    [ResourceEdit.core_title("x" * 256)]):
        with pytest.raises(ValueError):
            source.preview([ParagraphAlignmentEdit(source.body[0].anchor, "center")], resources=invalid)
    with pytest.raises(ValueError, match="snapshot"):
        result.preview([ParagraphAlignmentEdit(source.body[0].anchor, "center")],
                       resources=[ResourceEdit.core_title("stale batch")])
    assert source.core_title == "original" and unpack(preview(source)) == parts


def test_subject_can_be_set_and_cleared_independently_of_title():
    parts = fixture()
    source = ImportedDocument.open(pack(parts))
    value = "Subject & " + "📄" * 245
    data = source.preview([], resources=[ResourceEdit.core_subject(value),
                                         ResourceEdit.core_title("new title")]).to_docx()
    after = unpack(data)
    assert {n for n in after if parts[n] != after[n]} == {"properties/info.xml"}
    assert b"<!--retain--><?sample opaque?>" in after["properties/info.xml"]
    assert docx.Document(BytesIO(data)).core_properties.subject == value
    edited = ImportedDocument.open(data)
    assert (edited.core_title, edited.core_subject) == ("new title", value)
    for subject in ("", None):
        edited = ImportedDocument.open(edited.preview([], resources=[
            ResourceEdit.core_subject(subject)]).to_docx())
        assert (edited.core_title, edited.core_subject) == ("new title", subject)
    for edits in ([ResourceEdit.core_subject("x" * 256)],
                  [ResourceEdit.core_subject(None), ResourceEdit.core_subject("again")]):
        with pytest.raises(ValueError):
            source.preview([], resources=edits)
    assert unpack(source.preview([]).to_docx()) == parts


@pytest.mark.parametrize("replacement", [
    b"<dc:title>one</dc:title><dc:title>two</dc:title>",
    b'<dc:title xml:lang="de">original</dc:title>',
    b"<dc:title>original<!--retain--></dc:title>",
])
def test_reject_ambiguous_or_annotated_property(replacement):
    parts = fixture()
    name = "properties/info.xml"
    parts[name] = parts[name].replace(b"<dc:title>original</dc:title>", replacement)
    source = ImportedDocument.open(pack(parts))
    with pytest.raises(ValueError):
        _ = source.core_title
    with pytest.raises(ValueError):
        source.preview([], resources=[ResourceEdit.core_title(None)])
    assert unpack(source.preview([]).to_docx()) == parts
