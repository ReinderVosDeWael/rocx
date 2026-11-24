"""Independent Word fixtures exercising imported markup and lexical value spaces."""
from io import BytesIO
from zipfile import ZipFile

import docx
import pytest
from docx.oxml.ns import qn
from lxml import etree

from rocx.editor import ImportedDocument, InlineEdit, ResourceEdit, TableStructureEdit, TextInsertion


MC = "{http://schemas.openxmlformats.org/markup-compatibility/2006}"
EXT = "urn:editor-fixture:extension"


def save(document):
    stream = BytesIO()
    document.save(stream)
    return stream.getvalue()


def parts(data):
    with ZipFile(BytesIO(data)) as z:
        return {p: z.read(p) for p in z.namelist()}


def rewrite(data, part, mutate):
    files = parts(data)
    root = etree.fromstring(files[part])
    mutate(root)
    files[part] = etree.tostring(root)
    stream = BytesIO()
    with ZipFile(stream, "w") as z:
        for name, payload in files.items():
            z.writestr(name, payload)
    return stream.getvalue()


def wrapper(node, *, rule="ext:wrapper", name="wrapper"):
    element = etree.Element(f"{{{EXT}}}{name}", nsmap={"ext": EXT})
    element.set(MC + "Ignorable", "ext")
    element.set(MC + "ProcessContent", rule)
    element.append(node)
    return element


def wrap_property(root, tag):
    prop = root.find(".//" + qn(tag))
    parent, index = prop.getparent(), prop.getparent().index(prop)
    parent.insert(index, wrapper(prop))


@pytest.mark.parametrize("value", [None, "autofit"])
def test_table_layout_cannot_shadow_unwrapped_singleton(value):
    source = docx.Document()
    source.add_table(rows=1, cols=1).autofit = False
    data = rewrite(save(source), "word/document.xml", lambda n: wrap_property(n, "w:tblLayout"))
    with pytest.raises(ValueError, match="ProcessContent"):
        doc = ImportedDocument.open(data)
        doc.preview([], table_structure=[TableStructureEdit.layout(doc.body[0].anchor, value)])


@pytest.mark.parametrize("value", [None, 7])
def test_numbering_cannot_shadow_unwrapped_start(value):
    data = rewrite(save(docx.Document()), "word/numbering.xml", lambda n: wrap_property(n, "w:start"))
    doc = ImportedDocument.open(data)
    with pytest.raises(ValueError, match="ProcessContent"):
        doc.preview([], resources=[ResourceEdit.abstract_start(0, 0, value)])
    assert parts(doc.preview([]).to_docx()) == parts(data)


def bookmark(suffix, id_):
    node = etree.Element(qn("w:bookmark" + suffix))
    node.set(qn("w:id"), id_)
    if suffix == "Start":
        node.set(qn("w:name"), "Protected")
    return node


def test_point_cannot_cross_unwrapped_bookmark():
    source = docx.Document()
    source.add_paragraph()._p.append(wrapper(bookmark("Start", "2")))
    p = source.add_paragraph()
    p.add_run("😀").bold = True
    p.add_run("e\u0301").italic = True
    source.add_paragraph()._p.append(bookmark("End", "2"))
    with pytest.raises(ValueError, match="ProcessContent"):
        doc = ImportedDocument.open(save(source))
        doc.preview([], text_insertions=[TextInsertion(doc.body[1].anchor, 1, "Ω")])


def test_unwrapped_bookmark_guards_only_affected_comment_part():
    source = docx.Document()
    p = source.add_paragraph("body")
    reviewed = source.add_paragraph("reviewed")
    comment = source.add_comment(reviewed.runs[0], text="review", author="Ada")
    content = comment.paragraphs[0].runs[-1]._r
    content.addprevious(wrapper(bookmark("Start", "4")))
    content.addnext(bookmark("End", "4"))
    original = save(source)
    doc = ImportedDocument.open(original)
    data = doc.preview([], text_insertions=[TextInsertion(doc.body[0].anchor, len(p.text), "Ω")]).to_docx()
    assert docx.Document(BytesIO(data)).paragraphs[0].text == "bodyΩ"
    assert parts(data)["word/comments.xml"] == parts(original)["word/comments.xml"]
    with pytest.raises(ValueError, match="ProcessContent"):
        entry, = doc.entry_stories
        run = entry.blocks[0].inline_structure[-2]
        doc.edit_entry_inlines(entry.part_name, [InlineEdit.set_bold(run.anchor, True)])


@pytest.mark.parametrize("uri, rule, rejects", [
    (EXT, "ext:wrapper", True), (EXT, "ext:*", True),
    (EXT, "ext:other", False), ("urn:shadowed", "ext:wrapper", False),
])
def test_inherited_process_content_matches_expanded_names_only(uri, rule, rejects):
    source = docx.Document()
    source.add_paragraph("before")
    source.add_paragraph("after")

    def extension(root):
        body = root.find(qn("w:body"))
        p = etree.Element(qn("w:p"), nsmap={"ext": EXT})
        p.set(MC + "Ignorable", "ext")
        p.set(MC + "ProcessContent", rule)
        outer = etree.SubElement(p, f"{{{uri}}}wrapper", nsmap={"ext": uri})
        outer.set(MC + "Ignorable", "ext")
        # An unmatched opaque subtree may itself contain a matching wrapper;
        # ignored descendants must not prevent an unrelated edit.
        outer.append(wrapper(bookmark("Start", "2")))
        body.replace(body[0], p)

    data = rewrite(save(source), "word/document.xml", extension)

    def edit():
        doc = ImportedDocument.open(data)
        return doc.preview([], text_insertions=[TextInsertion(doc.body[1].anchor, 0, "!")]).to_docx()

    if rejects:
        with pytest.raises(ValueError, match="ProcessContent"):
            edit()
    else:
        assert docx.Document(BytesIO(edit())).paragraphs[1].text == "!after"


def test_arbitrary_precision_decimal_bookmark_ids_pair_by_value():
    source = docx.Document()
    p = source.add_paragraph()
    number = "123456789012345678901234567890"
    p._p.append(bookmark("Start", " +000" + number + " "))
    p._p.append(bookmark("End", number))
    p = source.add_paragraph()
    p.add_run("😀").bold = True
    p.add_run("e\u0301").italic = True
    doc = ImportedDocument.open(save(source))
    result = doc.preview([], text_insertions=[TextInsertion(doc.body[1].anchor, 1, "Ω")]).to_docx()
    first, second = docx.Document(BytesIO(result)).paragraphs[1].runs
    assert (first.text, first.bold, second.text, second.italic) == ("😀", True, "Ωe\u0301", True)


def test_string_permission_ids_do_not_pair_by_numeric_value():
    source = docx.Document()
    p = source.add_paragraph()
    for tag, value in [("w:permStart", "01"), ("w:permEnd", "1")]:
        node = etree.SubElement(p._p, qn(tag))
        node.set(qn("w:id"), value)
    source.add_paragraph("unsafe")
    doc = ImportedDocument.open(save(source))
    with pytest.raises(ValueError, match="outside fields and ranges"):
        doc.preview([], text_insertions=[TextInsertion(doc.body[1].anchor, 0, "Ω")])


def test_numbering_integer_whitespace_and_lexical_noop():
    def pad(root):
        abstract = root.find(qn("w:abstractNum"))
        level = abstract.find(qn("w:lvl"))
        abstract.set(qn("w:abstractNumId"), " +000 ")
        level.set(qn("w:ilvl"), "\t-0")
        level.find(qn("w:start")).set(qn("w:val"), "\n+001 ")

    original = rewrite(save(docx.Document()), "word/numbering.xml", pad)
    doc = ImportedDocument.open(original)
    level = doc.abstract_starts[0]
    assert (level.abstract_id, level.level, level.start) == (0, 0, 1)
    assert parts(doc.preview([], resources=[ResourceEdit.abstract_start(0, 0, 1)]).to_docx()) == parts(original)
    data = doc.preview([], resources=[ResourceEdit.abstract_start(0, 0, 9)]).to_docx()
    assert ImportedDocument.open(data).abstract_starts[0].start == 9


def test_mixed_case_core_media_type_is_shared_across_resource_apis():
    def upper(root):
        n = next(n for n in root if n.get("PartName") == "/docProps/core.xml")
        n.set("ContentType", n.get("ContentType").upper())

    original = rewrite(save(docx.Document()), "[Content_Types].xml", upper)
    doc = ImportedDocument.open(original)
    edited = doc.preview([], resources=[ResourceEdit.core_title("Ω"), ResourceEdit.metadata("creator", "Ada")]).to_docx()
    result = ImportedDocument.open(edited)
    assert (result.core_title, result.metadata_field("creator")) == ("Ω", "Ada")
    output = parts(edited)
    assert {k for k, v in parts(original).items() if output[k] != v} == {"docProps/core.xml"}


@pytest.mark.parametrize("value", [None, "fixed"])
def test_layout_noop_preserves_every_part(value):
    source = docx.Document()
    table = source.add_table(rows=1, cols=1)
    if value == "fixed":
        table.autofit = False
    original = save(source)
    doc = ImportedDocument.open(original)
    edit = TableStructureEdit.layout(doc.body[0].anchor, value)
    assert parts(doc.preview([], table_structure=[edit]).to_docx()) == parts(original)

    if value is not None:
        duplicate = etree.SubElement(table._tbl.tblPr, qn("w:tblLayout"))
        duplicate.set(qn("w:type"), value)
        doc = ImportedDocument.open(save(source))
        with pytest.raises(ValueError, match="ambiguous table properties"):
            doc.preview([], table_structure=[TableStructureEdit.layout(doc.body[0].anchor, value)])
