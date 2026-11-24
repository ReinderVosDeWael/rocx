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


def test_metadata_parts_and_types_round_trip_on_independent_document():
    parts = fixture()
    src = ImportedDocument.open(pack(parts))
    md, cp = ResourceEdit.metadata, ResourceEdit.custom_property
    values = dict(creator="reviewer", description="", keywords="a & b", category="sample",
                  version="v3", identifier="id-π", language="el", contentStatus="ready",
                  revision="7", lastModifiedBy="editor",
                  created="2000-02-29T13:01:12Z", modified="2002-03-01T00:00:00Z",
                  lastPrinted="2001-11-04T03:02:01Z", company="Acme", manager="Lead")
    edits = [md(k, v) for k, v in values.items()] + [cp("Reviewed", False), cp("Label", "")]
    data = src.preview([], resources=edits).to_docx()
    result = ImportedDocument.open(data)
    assert {k: result.metadata_field(k) for k in values} == values
    assert result.custom_property("Reviewed") is False
    assert result.custom_property("Label") == ""
    assert src.custom_property("Reviewed") is None
    document = docx.Document(BytesIO(data))
    assert document.core_properties.author == "reviewer"
    assert document.core_properties.created.year == 2000
    assert document.core_properties.revision == 7
    output = unpack(data)
    assert set(output) - set(parts) == {"docProps/custom.xml"}
    assert {p for p in parts if parts[p] != output[p]} == {
        "properties/info.xml", "docProps/app.xml", "_rels/.rels", "[Content_Types].xml"}
    assert b"<!--retain--><?sample opaque?>" in output["properties/info.xml"]
    assert unpack(result.preview([], resources=edits).to_docx()) == output
    with pytest.raises(ValueError, match="snapshot"):
        result.preview([ParagraphAlignmentEdit(src.body[0].anchor, "center")], resources=edits)
    cleared = ImportedDocument.open(result.preview([], resources=[
        *(md(k, None) for k in values), cp("Reviewed", None), cp("Label", True)]).to_docx())
    assert all(cleared.metadata_field(k) is None for k in values)
    assert cleared.custom_property("Reviewed") is None and cleared.custom_property("Label") is True

    for bad in ([md("created", "2025-02-29T00:00:00Z")], [md("manager", "a")] * 2,
                [cp("Label", None), cp("Label", False)], [cp("  ", "a")]):
        with pytest.raises(ValueError):
            result.preview([], resources=[md("creator", "uncommitted"), *bad])
    assert result.metadata_field("creator") == "reviewer"


def test_custom_guards_relocated_part_and_preserves_opaque_variant():
    parts = fixture()
    cp = ResourceEdit.custom_property
    seeded = ImportedDocument.open(pack(parts)).preview([], resources=[cp("Safe", True), cp("Opaque", "x")])
    parts = unpack(seeded.to_docx())
    path = "properties/named.xml"
    root = etree.fromstring(parts.pop("docProps/custom.xml"))
    opaque = root.find("{*}property[@name='Opaque']")
    opaque[0].tag = "{http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes}i4"
    opaque[0].text = "128"
    parts[path] = etree.tostring(root)
    for name in ("_rels/.rels", "[Content_Types].xml"):
        parts[name] = parts[name].replace(b"docProps/custom.xml", path.encode())
    src = ImportedDocument.open(pack(parts))
    with pytest.raises(ValueError, match="unsupported custom property type"):
        src.custom_property("Opaque")
    with pytest.raises(ValueError, match="unsupported custom property type"):
        src.preview([], resources=[cp("Safe", False), cp("Opaque", None)])
    edited = unpack(src.preview([], resources=[cp("Safe", False)]).to_docx())
    assert {n for n in edited if edited[n] != parts[n]} == {path}
    after = etree.fromstring(edited[path]).find("{*}property[@name='Opaque']")
    assert dict(after.attrib) == dict(opaque.attrib) and (after[0].tag, after[0].text) == (opaque[0].tag, "128")
    assert src.custom_property("Safe") is True


@pytest.mark.parametrize("type_uri,valid", [
    ("http://purl.org/dc/terms/", True), ("urn:unrelated", False)])
def test_metadata_date_uses_original_qname_scope(type_uri, valid):
    parts = fixture()
    key = "properties/info.xml"
    parts[key] = parts[key].replace(b'xsi:type="dcterms:W3CDTF"',
                                    f'xmlns:q="{type_uri}" xsi:type="q:W3CDTF"'.encode())
    source = ImportedDocument.open(pack(parts))
    edit = ResourceEdit.metadata("created", "2032-02-29T01:00:02Z")
    if valid:
        out = unpack(source.preview([], resources=[edit]).to_docx())
        assert {p for p in out if out[p] != parts[p]} == {key}
        node = etree.fromstring(out[key]).find("{http://purl.org/dc/terms/}created")
        assert node.get("{http://www.w3.org/2001/XMLSchema-instance}type") == "q:W3CDTF"
        assert node.nsmap["q"] == type_uri and node.text == "2032-02-29T01:00:02Z"
    else:
        with pytest.raises(ValueError, match="date type"):
            source.metadata_field("created")
        with pytest.raises(ValueError, match="date type"):
            source.preview([], resources=[edit])
        assert unpack(source.preview([]).to_docx()) == parts


def test_metadata_respects_source_mandatory_namespace():
    parts = fixture()
    key = "properties/info.xml"
    parts[key] = parts[key].replace(b"<cp:coreProperties ", b'<cp:coreProperties '
        b'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
        b'xmlns:ext="urn:unknown" mc:MustUnderstand="ext" ')
    src = ImportedDocument.open(pack(parts))
    with pytest.raises(ValueError, match="mandatory namespace"):
        src.preview([], resources=[ResourceEdit.metadata("creator", "x")])
    assert unpack(src.preview([]).to_docx()) == parts
