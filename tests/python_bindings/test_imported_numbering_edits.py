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


def test_create_numbering_part_next_to_relocated_main_without_claiming_orphan():
    doc = docx.Document()
    doc.add_paragraph("first list")
    buf = BytesIO()
    doc.save(buf)
    parts = {k.replace("word/", "custom/"): v for k,v in unpack(buf.getvalue()).items()}
    parts["_rels/.rels"] = parts["_rels/.rels"].replace(b"word/document.xml", b"custom/document.xml")
    parts["[Content_Types].xml"] = parts["[Content_Types].xml"].replace(b"/word/", b"/custom/")
    rels = "custom/_rels/document.xml.rels"
    root = etree.fromstring(parts[rels])
    num_rel, = [n for n in root if n.get("Type").endswith("/numbering")]
    root.remove(num_rel)
    parts[rels] = etree.tostring(root)
    # Also reserve a matching stem with different case; it must not be adopted.
    parts["custom/ROCX-Numbering1.xml"] = parts["custom/numbering.xml"]
    doc = ImportedDocument.open(pack(parts))
    assert doc.numbering_instances == []
    with pytest.raises(ValueError, match="referenced"):
        doc.preview([], resources=[R.add_abstract(90, [NumberingLevel()]), R.add_instance(1, 90)])
    result = doc.preview([], resources=[R.add_instance(91, 90), R.add_abstract(90, [NumberingLevel()]),
                                        R.paragraph_numbering(doc.body[0].anchor, (91, 0))]).to_docx()
    after = unpack(result)
    assert after.keys() - parts.keys() == {"custom/rocx-numbering2.xml"}
    assert {n for n in parts if after[n] != parts[n]} == {"custom/document.xml", rels, "[Content_Types].xml"}
    word = docx.Document(BytesIO(result))
    assert str(word.part.numbering_part.partname) == "/custom/rocx-numbering2.xml"
    assert word.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["91"]


def test_replace_rich_levels_relink_and_preserve_local_metadata():
    from rocx.declarative import ParagraphDefaults, RunDefaults
    rich = NumberingLevel(label_format=RunDefaults(bold=True, size=12), paragraph_format=ParagraphDefaults(left_indent=12))
    src = ImportedDocument.open(pack(fixture()))
    parts = unpack(src.preview([], resources=[R.add_abstract(80, [rich, NumberingLevel(level=1)]),
        R.add_instance(90, 80), R.instance_level(90, 1, NumberingLevel(level=1, label_format=RunDefaults(italic=True))),
        R.paragraph_numbering(src.body[0].anchor, (90, 1))]).to_docx())
    root = etree.fromstring(parts[NUMBERS])
    ns = {"w": qn("w:lvl").split("}")[0][1:]}
    definition, = root.xpath("w:abstractNum[@w:abstractNumId='80']", namespaces=ns)
    definition.insert(0, etree.Comment("kept with abstract"))
    parts[NUMBERS] = etree.tostring(root)
    src = ImportedDocument.open(pack(parts))
    equivalent = src.preview([], resources=[
        R.replace_abstract(80, [rich, NumberingLevel(level=1)]),
        R.instance_level(90, 1, NumberingLevel(level=1, label_format=RunDefaults(italic=True))),
        R.relink_instance(90, 80),
    ]).to_docx()
    assert unpack(equivalent) == parts
    for invalid in [
        [R.abstract_level(80, 1, None)], [R.replace_abstract(80, [NumberingLevel()])],
        [R.add_abstract(81, [NumberingLevel()]), R.relink_instance(90, 81)],
        [R.replace_abstract(80, [rich]), R.abstract_level(80, 1, None)],
    ]:
        with pytest.raises(ValueError):
            src.preview([], resources=invalid)
    output = src.preview([], resources=[
        R.relink_instance(90, 81), R.replace_abstract(80, [NumberingLevel(start=7)]),
        R.add_abstract(81, [NumberingLevel(), NumberingLevel(level=1)]),
        R.instance_level(90, 1, NumberingLevel(level=1, format="bullet", text="•")),
    ]).to_docx()
    after = unpack(output)
    assert {n for n in parts if parts[n] != after[n]} == {NUMBERS}
    assert b"<!--kept with abstract-->" in after[NUMBERS]
    dest = ImportedDocument.open(output)
    assert dest.paragraph_numbering(dest.body[0].anchor) == (90, 1)
    n, = (n for n in dest.numbering_instances if n.num_id == 90)
    assert (n.abstract_id, n.replacement_levels) == (81, [1])
    from rocx.declarative import NumberingDefinition, NumberingStartOverride
    replace = R.replace_instance(NumberingDefinition(90, abstract_id=81, start_overrides=[NumberingStartOverride(0, 2)]))
    with pytest.raises(ValueError, match="conflicting"):
        dest.preview([], resources=[replace, R.instance_start(90, 1, None)])
    replaced = ImportedDocument.open(dest.preview([], resources=[replace]).to_docx())
    n, = (n for n in replaced.numbering_instances if n.num_id == 90)
    assert (n.abstract_id, n.replacement_levels, n.start_overrides) == (81, [], [(0, 2)])
    expanded = ImportedDocument.open(dest.preview([], resources=[R.abstract_level(80, 1, rich_level(1))]).to_docx())
    assert any(a.abstract_id == 80 and a.level == 1 for a in expanded.abstract_starts)
    shrunk = ImportedDocument.open(expanded.preview([], resources=[R.abstract_level(80, 1, None)]).to_docx())
    assert not any(a.abstract_id == 80 and a.level == 1 for a in shrunk.abstract_starts)
    definition.find(qn("w:lvl")).find(qn("w:rPr")).append(etree.Comment("keep inside formatting"))
    parts[NUMBERS] = etree.tostring(root)
    opaque = ImportedDocument.open(pack(parts))
    with pytest.raises(ValueError, match="unmodeled"):
        opaque.preview([], resources=[R.abstract_level(80, 0, NumberingLevel())])


def rich_level(level):
    from rocx.declarative import RunDefaults
    return NumberingLevel(level=level, label_format=RunDefaults(bold=True))


def test_number_empty_shared_footer_by_physical_snapshot_address():
    from docx.enum.section import WD_SECTION
    word = docx.Document()
    word.sections[0].footer.paragraphs[0].text = ""
    word.add_section(WD_SECTION.NEW_PAGE)
    buf = BytesIO()
    word.save(buf)
    parts = unpack(buf.getvalue())
    src = ImportedDocument.open(buf.getvalue())
    aliases = [s for s in src.stories if s.kind == "footer" and s.variant == "default"]
    assert len(aliases) == 2 and aliases[0].part_name == aliases[1].part_name
    targets = [src.story_paragraph_anchor(s.part_name, 0) for s in aliases]
    edit = lambda target: R.story_paragraph_numbering(target, (1, 0))
    with pytest.raises(ValueError, match="duplicate"):
        src.preview([], resources=list(map(edit, targets)))
    with pytest.raises(ValueError):
        src.story_paragraph_anchor("word/styles.xml", 0)
    data = src.preview([], resources=[edit(targets[0])]).to_docx()
    final = unpack(data)
    assert {n for n in parts if final[n] != parts[n]} == {aliases[0].part_name}
    assert docx.Document(BytesIO(data)).sections[1].footer.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["1"]
    reopened = ImportedDocument.open(data)
    fresh = reopened.story_paragraph_anchor(aliases[0].part_name, 0)
    assert reopened.story_paragraph_numbering(fresh) == (1, 0)
    with pytest.raises(ValueError, match="snapshot"):
        reopened.preview([], resources=[edit(targets[0])])
