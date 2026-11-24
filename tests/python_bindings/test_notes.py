"""Exact note/structured-comment stories and imported note graph checks."""
import io
import pathlib
import posixpath
import zipfile
from collections.abc import Callable

import docx
from lxml import etree
import pytest
from rocx import declarative as d

W="{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
R="{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
TARGET="https://example.invalid/λ?q=a%20b&x=2#frag"
IMAGE=pathlib.Path(__file__).parents[1]/"fixtures/images/sample.png"


def declaration() -> d.Document:
    """Build note and comment stories containing formatted text, images and tables."""
    def image() -> d.ImageRun:
        return d.ImageRun(data=IMAGE)
    def table(text: str) -> d.Table:
        nested=d.Table(rows=[d.TableRow(children=[d.TableCell(children=[d.Paragraph(text="nested table")])])])
        return d.Table(rows=[d.TableRow(children=[d.TableCell(children=[d.Paragraph(text=text)]),d.TableCell(children=[nested])])])
    comment=d.Comment([d.Paragraph(children=[d.TextRun("Rich λ",bold=True),d.Hyperlink(TARGET,[d.TextRun("comment link")]),image()]),table("comment table")],author="Ada",date="2026-11-18T01:02:03Z",initials="AL")
    return d.Document([d.Section(children=[
        d.Paragraph(children=[d.TextRun("body λ"),d.NoteReference.footnote(0),d.NoteReference.footnote(2),d.NoteReference.endnote(0)]),
        d.Paragraph(children=[d.RangeStart.structured_comment("review",comment),d.TextRun("reviewed"),d.RangeEnd("review")])])],notes=[
            d.Note.footnote(0,[d.Paragraph(children=[d.TextRun("Foot λ",bold=True),d.Hyperlink(TARGET,[d.TextRun("note link")]),image()]),table("note table")]),
            d.Note.footnote(2,[d.Paragraph(text="Other foot")]),d.Note.endnote(0,[d.Paragraph(children=[d.TextRun("End Ω"),image()])])],timestamp="2026-11-18T00:00:00Z")


async def test_note_parts_and_comment_content_have_exact_owned_resources() -> None:
    snapshot=await declaration().resolve()
    data=await snapshot.to_bytes()
    output=docx.Document(io.BytesIO(data))
    assert [p.text for p in output.paragraphs]==["body λ","reviewed"]
    comment=list(output.comments)[0]
    assert comment.author=="Ada"
    assert comment.paragraphs[0].text=="Rich λcomment link"
    assert comment.tables[0].cell(0,0).text=="comment table"
    assert comment.tables[0].cell(0,1).tables[0].cell(0,0).text=="nested table"
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        for owner,kind,text in [("footnotes","footnote","Foot λ"),("endnotes","endnote","End Ω")]:
            root=etree.fromstring(package.read(f"word/{owner}.xml"))
            entries={int(n.get(f"{W}id")):n for n in root if n.tag==f"{W}{kind}"}
            assert entries[0].get(f"{W}type","normal")=="normal"
            assert text in "".join(entries[0].itertext())
            assert entries[0].find(f".//{W}{kind}Ref") is not None
            assert {n.get(f"{W}type") for n in entries.values() if n.get(f"{W}type")}=={"separator","continuationSeparator"}
            settings=etree.fromstring(package.read("word/settings.xml"))
            selected=settings.findall(f"{W}{kind}Pr/{W}{kind}")
            assert len(selected)==2
            assert all(entries[int(n.get(f"{W}id"))].get(f"{W}type") in {"separator","continuationSeparator"} for n in selected)
            assert etree.fromstring(package.read(f"word/_rels/{owner}.xml.rels")) is not None
        root=etree.fromstring(package.read("word/comments.xml"))
        assert root[0].get(f"{W}date")=="2026-11-18T01:02:03Z"
        assert root[0].get(f"{W}initials")=="AL"
    assert data==await snapshot.to_bytes()


async def test_deferred_note_fields_and_comment_metadata_accept_generic_references() -> None:
    source=d.Note(kind=lambda:"footnote",id=lambda:5,children=lambda:[d.Paragraph(text="async note")])
    reference=d.NoteReference(source.deferred("kind"),source.deferred("id"))
    note=d.Note(source.deferred("kind"),source.deferred("id"),source.deferred("children"))
    hidden=d.Note.footnote(9,lambda: (_ for _ in ()).throw(AssertionError("hidden note ran")))
    hidden.condition=False
    source_comment=d.Comment(children=lambda:[d.Paragraph(text="async comment")],author=lambda:"Async Ada",date=lambda:"2024-02-29T00:00:00Z",initials=lambda:"AA")
    comment=d.Comment(source_comment.deferred("children"),author=source_comment.deferred("author"),date=source_comment.deferred("date"),initials=source_comment.deferred("initials"))
    data=await d.Document([d.Section(children=[d.Paragraph(children=[reference,d.RangeStart.structured_comment("async",comment),d.TextRun("anchor"),d.RangeEnd("async")])])],notes=lambda:[note,hidden]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert b"async note" in package.read("word/footnotes.xml")
        root=etree.fromstring(package.read("word/comments.xml"))
        assert root[0].get(f"{W}author")=="Async Ada"
        assert root[0].get(f"{W}date")=="2024-02-29T00:00:00Z"


@pytest.mark.parametrize("notes,children",[
    ([],[d.NoteReference.footnote(7)]),
    ([d.Note.footnote(1,[]),d.Note.footnote(1,[])],[]),
    ([d.Note.footnote(1,[d.Paragraph(children=[d.NoteReference.endnote(2)])])],[]),
])
async def test_missing_duplicate_and_nested_notes_fail_with_locations(notes: list[object],children: list[object]) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=children)])],notes=notes).to_bytes()
    assert error.value.rule=="invalid_content"
    assert error.value.location


async def test_note_references_and_nested_comments_are_rejected_in_illegal_stories() -> None:
    with pytest.raises(ValueError):
        await d.Document([d.Section(children=[],headers={"default":d.Header(children=[d.Paragraph(children=[d.NoteReference.footnote(1)])])})],notes=[d.Note.footnote(1,[])]).to_bytes()
    content=d.Comment([d.Paragraph(children=[d.RangeStart.comment("nested","no"),d.TextRun("text"),d.RangeEnd("nested")])])
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.RangeStart.structured_comment("outer",content),d.TextRun("body"),d.RangeEnd("outer")])])]).to_bytes()
    assert error.value.rule=="invalid_content"


@pytest.mark.parametrize("date",["bad","2026-02-29T00:00:00Z","2026-01-01T24:00:00Z"])
async def test_comment_dates_are_checked_before_serialization(date: str) -> None:
    comment=d.Comment([d.Paragraph(text="content")],date=date)
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.RangeStart.structured_comment("review",comment),d.TextRun("body"),d.RangeEnd("review")])])]).to_bytes()
    assert error.value.rule=="invalid_content"
    assert error.value.location.endswith("date")


def rewrite(data: bytes, part: str, change: Callable[[etree._Element], None]) -> bytes:
    """Mutate one readable XML part without breaking the ZIP container."""
    result=io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(data)) as source,zipfile.ZipFile(result,"w") as target:
        for name in source.namelist():
            payload=source.read(name)
            if name==part:
                root=etree.fromstring(payload)
                change(root)
                payload=etree.tostring(root)
            target.writestr(name,payload)
    return result.getvalue()


def large_id_package(data: bytes) -> bytes:
    """Retain a valid imported decimal note ID beyond machine-integer precision."""
    identifier="1"+"0"*5000
    def definition(root: etree._Element) -> None:
        next(n for n in root if n.get(f"{W}id")=="0").set(f"{W}id",identifier)
    def reference(root: etree._Element) -> None:
        next(n for n in root.iter(f"{W}footnoteReference") if n.get(f"{W}id")=="0").set(f"{W}id",identifier)
    return rewrite(rewrite(data,"word/footnotes.xml",definition),"word/document.xml",reference)


async def test_large_imported_note_ids_can_be_retained_without_narrowing_them(tmp_path: pathlib.Path) -> None:
    source=large_id_package(await declaration().to_bytes())
    template=tmp_path/"large-id.docx"
    template.write_bytes(source)
    data=await d.Document([]).to_bytes(d.DocumentTemplate(template))
    with zipfile.ZipFile(io.BytesIO(source)) as original,zipfile.ZipFile(io.BytesIO(data)) as output:
        assert output.read("word/footnotes.xml")==original.read("word/footnotes.xml")


async def test_imported_ids_and_note_types_are_validated_before_output(tmp_path: pathlib.Path) -> None:
    source=await declaration().to_bytes()
    template=tmp_path/"source.docx"
    template.write_bytes(source)
    output=tmp_path/"output.docx"
    output.write_bytes(b"keep")
    collision=d.Document([],notes=[d.Note.footnote(0,[])])
    with pytest.raises(ValueError) as error:
        await collision.save(output,d.DocumentTemplate(template))
    assert error.value.rule=="invalid_content"
    assert output.read_bytes()==b"keep"
    def special(root: etree._Element) -> None:
        next(n for n in root if n.get(f"{W}id")=="0").set(f"{W}type","separator")
    template.write_bytes(rewrite(source,"word/footnotes.xml",special))
    with pytest.raises(ValueError) as error:
        await d.Document([]).to_bytes(d.DocumentTemplate(template))
    assert error.value.rule=="invalid_content"


@pytest.mark.parametrize("kind",["footnote","endnote"])
async def test_relocated_note_owners_and_supported_story_edits_are_relationship_driven(tmp_path: pathlib.Path,kind: str) -> None:
    source=await declaration().to_bytes()
    template=tmp_path/"relocated.docx"
    with zipfile.ZipFile(io.BytesIO(source)) as original,zipfile.ZipFile(template,"w") as target:
        for name in original.namelist():
            payload=original.read(name)
            destination={f"word/{kind}s.xml":"annotations/notes.xml",f"word/_rels/{kind}s.xml.rels":"annotations/_rels/notes.xml.rels"}.get(name,name)
            if name=="[Content_Types].xml":
                payload=payload.replace(f"/word/{kind}s.xml".encode(),b"/annotations/notes.xml")
            elif name=="word/_rels/document.xml.rels":
                payload=payload.replace(f'Target="{kind}s.xml"'.encode(),b'Target="../annotations/notes.xml"')
            elif name==f"word/_rels/{kind}s.xml.rels":
                payload=payload.replace(b'Target="media/',b'Target="../word/media/')
            target.writestr(destination,payload)
    from rocx.editor import ImportedDocument, InlineEdit
    imported = ImportedDocument.open(template.read_bytes())
    note = next(e for e in imported.entry_stories if e.kind == kind and e.id == "0")
    reference = next(n for n in imported.body[0].inline_structure
                     if any(i.kind == kind + "Reference" and i.value == "0" for i in n.items))
    numbered = imported.edit_note_reference(note.part_name, reference.anchor, "renumber", id=12)
    with zipfile.ZipFile(template) as old, zipfile.ZipFile(io.BytesIO(numbered)) as new:
        assert {p for p in old.namelist() if old.read(p) != new.read(p)} == {note.part_name, "word/document.xml"}
        assert etree.fromstring(new.read(note.part_name)).find(f"{W}{kind}[@{W}id='12']") is not None
    anchor = next(r.anchor for r in note.blocks[0].inline_structure if any(
        i.kind == "text" for i in getattr(r, "items", [])))
    new_url = "https://example.org/relocated-owner"
    link = await d.Hyperlink(new_url, [d.TextRun("review")]).resolve()
    changed = imported.edit_entry_inlines(note.part_name, [InlineEdit.insert_after(anchor, link)])
    with zipfile.ZipFile(template) as old, zipfile.ZipFile(io.BytesIO(changed)) as new:
        relpath = "annotations/_rels/notes.xml.rels"
        assert {p for p in old.namelist() if old.read(p) != new.read(p)} == {note.part_name, relpath}
        added, = etree.fromstring(new.read(note.part_name)).xpath(
            ".//w:hyperlink[w:r/w:t='review']", namespaces={"w": W[1:-1]})
        owned, = [r for r in etree.fromstring(new.read(relpath)) if r.get("Id") == added.get(R + "id")]
        assert owned.get("Target") == new_url and owned.get("TargetMode") == "External"
    addition=d.Document([d.Section(children=[d.Paragraph(children=[d.NoteReference(kind,9)])])],notes=[
        d.Note(kind,9,[d.Paragraph(children=[d.Hyperlink(TARGET,[d.TextRun("new link")]),d.ImageRun(data=IMAGE)])])])
    original_text="Other foot" if kind=="footnote" else "End Ω"
    data=await addition.to_bytes(d.DocumentTemplate(template,replacements={original_text:"Edited note"},scope="supported_stories"))
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert f"word/{kind}s.xml" not in package.namelist()
        root=etree.fromstring(package.read("annotations/notes.xml"))
        assert "Edited note" in "".join(root.itertext())
        new=next(n for n in root if n.get(f"{W}id")=="9")
        assert "new link" in "".join(new.itertext())
        rels=etree.fromstring(package.read("annotations/_rels/notes.xml.rels"))
        for link in new.iter(f"{W}hyperlink"):
            relation=next(n for n in rels if n.get("Id")==link.get(f"{R}id"))
            assert relation.get("Target")==TARGET
            assert relation.get("TargetMode")=="External"
        for blip in new.iter("{http://schemas.openxmlformats.org/drawingml/2006/main}blip"):
            relation=next(n for n in rels if n.get("Id")==blip.get(f"{R}embed"))
            target=posixpath.normpath(posixpath.join("annotations",relation.get("Target")))
            assert package.read(target)==IMAGE.read_bytes()


async def test_note_and_comment_snapshot_images_are_captured_before_rendering(tmp_path: pathlib.Path) -> None:
    image=tmp_path/"image.png"
    image.write_bytes(IMAGE.read_bytes())
    note=d.Note.footnote(1,[d.Paragraph(children=[d.ImageRun(data=image)])])
    comment=d.Comment([d.Paragraph(children=[d.ImageRun(data=image)])])
    document=d.Document([d.Section(children=[d.Paragraph(children=[d.NoteReference.footnote(1),
        d.RangeStart.structured_comment("review",comment),d.TextRun("body"),d.RangeEnd("review")])])],notes=[note])
    snapshot=await document.resolve()
    image.unlink()
    data=await snapshot.to_bytes()
    assert data==await snapshot.to_bytes()


@pytest.mark.parametrize("kind", ["footnote", "endnote"])
async def test_physical_entry_text_preserves_definition_and_reference_identity(kind):
    from rocx.editor import ImportedDocument, RunTextEdit

    data = large_id_package(await declaration().to_bytes())
    owner = f"word/{kind}s.xml"
    imported = ImportedDocument.open(data)
    note = next(e for e in imported.entry_stories if e.kind == kind and not e.note_type)
    run = next(r for r in note.blocks[0].inline_structure if any(
        i.kind == "text" for i in getattr(r, "items", [])))
    change = RunTextEdit(run.anchor, "Revised story text")
    wrong = "word/endnotes.xml" if kind == "footnote" else "word/footnotes.xml"
    with pytest.raises(ValueError, match="owner mismatch"):
        imported.edit_story_text(wrong, [change])
    special = next(e for e in imported.entry_stories if e.kind == kind and e.note_type == "separator")
    with pytest.raises(ValueError, match="reserved note entry"):
        imported.edit_story_text(owner, [change, RunTextEdit(special.blocks[0].inline_structure[0].anchor, "unsafe")])
    changed = imported.edit_story_text(owner, [change])
    with zipfile.ZipFile(io.BytesIO(data)) as source, zipfile.ZipFile(io.BytesIO(changed)) as output:
        assert set(source.namelist()) == set(output.namelist())
        assert all(source.read(n) == output.read(n) for n in source.namelist() if n != owner)
        root = etree.fromstring(source.read(owner))
        selected, = [e for e in root if e.get(W + "id") == note.id]
        next(t for t in selected.iter(W + "t") if t.text == run.items[0].value).text = "Revised story text"
        assert etree.tostring(root, method="c14n", exclusive=True) == etree.tostring(
            etree.fromstring(output.read(owner)), method="c14n", exclusive=True)


async def test_entry_text_refuses_ambiguous_decimal_id():
    from rocx.editor import ImportedDocument, RunTextEdit

    owner = "word/footnotes.xml"
    def duplicate(root):
        next(e for e in root if e.get(W + "id") == "2").set(W + "id", "+000")
    imported = ImportedDocument.open(rewrite(await declaration().to_bytes(), owner, duplicate))
    note = next(e for e in imported.entry_stories if e.kind == "footnote" and e.id == "+000")
    run = next(n for n in note.blocks[0].inline_structure if any(i.kind == "text" for i in n.items))
    with pytest.raises(ValueError, match="ambiguous story entry ID"):
        imported.edit_story_text(owner, [RunTextEdit(run.anchor, "ambiguous")])


async def test_independent_comment_inline_batch_keeps_anchor_and_annotation_reference():
    from rocx.editor import ImportedDocument, InlineEdit

    source = docx.Document()
    reviewed = source.add_paragraph("reviewed body").runs[0]
    comment = source.add_comment(reviewed, text="Review", author="Ada", initials="AL")
    comment.paragraphs[0].add_run(" this")
    buffer = io.BytesIO()
    source.save(buffer)
    imported = ImportedDocument.open(buffer.getvalue())
    entry, = imported.entry_stories
    mark, first, second = entry.blocks[0].inline_structure
    bold = InlineEdit.set_bold(first.anchor, True)
    replacement = await d.Hyperlink(TARGET, [d.TextRun("details")]).resolve()
    link = InlineEdit.replace(second.anchor, replacement)
    with pytest.raises(ValueError):
        imported.edit_entry_inlines(entry.part_name, [link, InlineEdit.replace(mark.anchor, replacement)])
    changed = imported.edit_entry_inlines(entry.part_name, [link, bold])
    with pytest.raises(ValueError):
        ImportedDocument.open(changed).edit_entry_inlines(entry.part_name, [bold])
    result = docx.Document(io.BytesIO(changed))
    edited, = result.comments
    assert (edited.comment_id, edited.author, edited.initials) == (comment.comment_id, "Ada", "AL")
    assert edited.paragraphs[0].text == "Reviewdetails" and edited.paragraphs[0].runs[-1].bold
    assert edited.paragraphs[0].hyperlinks[0].address == TARGET
    assert edited.paragraphs[0]._p.xpath(".//w:annotationRef")
    with zipfile.ZipFile(buffer) as before, zipfile.ZipFile(io.BytesIO(changed)) as after:
        assert set(after.namelist()) == set(before.namelist()) | {"word/_rels/comments.xml.rels"}
        assert all(before.read(p) == after.read(p) for p in before.namelist() if p != entry.part_name)


@pytest.mark.parametrize("kind", ["footnote", "endnote"])
async def test_entry_inline_rejects_reference_run_and_special_entry(kind):
    from rocx.editor import ImportedDocument, InlineEdit

    imported = ImportedDocument.open(await declaration().to_bytes())
    note = next(e for e in imported.entry_stories if e.kind == kind and not e.note_type)
    reference, text, *_ = note.blocks[0].inline_structure
    special = next(e for e in imported.entry_stories if e.kind == kind and e.note_type == "separator")
    good = InlineEdit.set_bold(text.anchor, True)
    for protected in (reference, special.blocks[0].inline_structure[0]):
        with pytest.raises(ValueError):
            imported.edit_entry_inlines(note.part_name, [good, InlineEdit.set_bold(protected.anchor, False)])
    with pytest.raises(ValueError, match="owner mismatch"):
        imported.edit_entry_inlines("word/document.xml", [good])
    changed = imported.edit_entry_inlines(note.part_name, [good])
    with zipfile.ZipFile(io.BytesIO(changed)) as output:
        assert etree.fromstring(output.read(note.part_name)).find(f".//{W}{kind}Ref") is not None


@pytest.mark.parametrize("kind", ["footnote", "endnote"])
def test_first_note_lifecycle_on_independent_document(kind):
    from rocx.editor import ImportedDocument

    source = docx.Document()
    source.add_paragraph("anchor")
    stream = io.BytesIO()
    source.save(stream)
    original = stream.getvalue()
    owner = f"word/{kind}s.xml"
    def run(doc, index=1):
        return doc.body[0].inline_structure[index].anchor
    doc = ImportedDocument.open(original)
    created = doc.edit_note_reference(owner, run(doc, 0), "create", id=4, text="My note")
    created_doc = ImportedDocument.open(created)
    marker, = created_doc.body[0].inline_structure[1].items
    assert (marker.kind, marker.value) == (kind + "Reference", "4")
    with pytest.raises(ValueError):
        created_doc.edit_entry(owner,"4","deleteUnusedNote")
    detached = ImportedDocument.open(created_doc.edit_note_reference(owner, run(created_doc), "detach"))
    attached = ImportedDocument.open(detached.edit_note_reference(owner, run(detached,0), "attach", id=4))
    assert attached.body[0].inline_structure[1].items[0].value == "4"
    removed = ImportedDocument.open(detached.edit_entry(owner, "4","deleteUnusedNote"))
    assert removed.entry_stories == []
    # The empty existing part can be reused even without an inspectable entry.
    assert ImportedDocument.open(removed.edit_note_reference(owner,run(removed,0),"create",id=5,text="Reused")).entry_stories[0].id == "5"
    with pytest.raises(ValueError, match="snapshot"):
        created_doc.edit_note_reference(owner, run(doc, 0), "delete")
    with pytest.raises(ValueError):
        created_doc.edit_note_reference(owner, run(created_doc, 0), "create", id=4, text="Collision")
    with pytest.raises(ValueError):
        created_doc.edit_entry(owner, "4", "removeParagraph", index=0)
    replaced = created_doc.edit_entry(owner, "+004", "replaceParagraph", index=0, value="Replaced")
    with zipfile.ZipFile(io.BytesIO(replaced)) as package:
        first = etree.fromstring(package.read(owner))[0][0]
        assert first[0][-1].tag == W + kind + "Ref"
        assert first[1][0].text == "Replaced"
    # An existing unused ordinary definition is a valid destination; changing
    # the reference alone must retain both definition subtrees byte-for-byte.
    def add_unused(root):
        unused = etree.fromstring(etree.tostring(root[0]))
        unused.set(W + "id", "+007")
        root.append(unused)
    spare = rewrite(replaced, owner, add_unused)
    doc = ImportedDocument.open(spare)
    changed = doc.edit_note_reference(owner, run(doc), "retarget", id=7)
    with zipfile.ZipFile(io.BytesIO(spare)) as before, zipfile.ZipFile(io.BytesIO(changed)) as after:
        assert [p for p in before.namelist() if before.read(p) != after.read(p)] == ["word/document.xml"]
        assert etree.fromstring(after.read("word/document.xml")).find(f".//{W}{kind}Reference").get(W+"id") == "7"
    doc = ImportedDocument.open(changed)
    with pytest.raises(ValueError):
        doc.edit_note_reference(owner,run(doc),"renumber",id=4)
    renumbered = ImportedDocument.open(doc.edit_note_reference(owner,run(doc),"renumber",id=9))
    deleted = renumbered.edit_note_reference(owner, run(renumbered), "delete")
    assert docx.Document(io.BytesIO(deleted)).paragraphs[0].text == "anchor"
    with zipfile.ZipFile(io.BytesIO(original)) as before, zipfile.ZipFile(io.BytesIO(deleted)) as after:
        assert set(after.namelist()) == set(before.namelist()) | {owner}
        assert all(before.read(p) == after.read(p) for p in set(before.namelist()) - {
            "word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"})
        assert not etree.fromstring(after.read("word/document.xml")).findall(f".//{W}{kind}Reference")
        assert [n.get(W+"id") for n in etree.fromstring(after.read(owner))] == ["4"]


async def test_reference_graph_refuses_hidden_aliases_and_opaque_discard():
    from rocx.editor import ImportedDocument

    data = await declaration().to_bytes()
    owner = "word/footnotes.xml"
    def add_reference(root):
        etree.SubElement(root,W+"footnoteReference",{W+"id":"+0002"})
    # Not a body paragraph and not in the note part: counting just the selected
    # owner's refs would silently break the other consumer.
    dup = ImportedDocument.open(rewrite(data, "word/settings.xml", add_reference))
    with pytest.raises(ValueError, match="ambiguous"):
        dup.edit_note_reference(owner, dup.body[0].inline_structure[2].anchor, "delete")
    imported = ImportedDocument.open(data)
    with pytest.raises(ValueError, match="rich or opaque"):
        imported.edit_note_reference(owner, imported.body[0].inline_structure[1].anchor, "delete")
    with pytest.raises(ValueError):
        imported.edit_entry(owner, "0", "replaceParagraph", index=0, value="Cannot discard image/link")
    with pytest.raises(ValueError, match="reserved"):
        imported.edit_entry(owner, "-1", "insertParagraph", index=1, value="Special")
    def protected(root):
        first = root.find(f".//{W}footnoteReference")
        first.set(W+"customMarkFollows","1")
    custom = ImportedDocument.open(rewrite(data,"word/document.xml",protected))
    assert custom.body[0].inline_structure[1].items[0].kind == "opaque"
    with pytest.raises(ValueError, match="simple direct"):
        custom.edit_note_reference(owner,custom.body[0].inline_structure[1].anchor,"renumber",id=12)
    def wrap_range(root):
        p = root.find(f".//{W}p")
        p.insert(0,etree.Element(W+"bookmarkStart",{W+"id":"91",W+"name":"protected"}))
        p.append(etree.Element(W+"bookmarkEnd",{W+"id":"91"}))
    protected = ImportedDocument.open(rewrite(data,"word/document.xml",wrap_range))
    text = next(r for r in protected.body[0].inline_structure if any(i.kind == "text" for i in getattr(r,"items",[])))
    with pytest.raises(ValueError, match="protected range"):
        protected.edit_note_reference(owner,text.anchor,"create",id=12,text="Inside bookmark")


def test_base_comment_metadata_and_block_patches_on_independent_document():
    from rocx.editor import ImportedDocument

    source = docx.Document()
    comment = source.add_comment(source.add_paragraph("anchor").runs[0], "Initial", "Ada", "AL")
    stream = io.BytesIO()
    source.save(stream)
    initial = stream.getvalue()
    owner, id = "word/comments.xml", str(comment.comment_id)
    doc = ImportedDocument.open(initial)
    with pytest.raises(ValueError):
        doc.edit_entry(owner,id,"date",value="invalid")
    with pytest.raises(ValueError):
        doc.edit_entry(owner,id,"removeParagraph",index=0)  # annotationRef and last paragraph
    output = initial
    for op, kw in [("author", {"value": "Reviewer"}), ("date", {"value":"2024-02-29T00:00:00Z"}),
                   ("clearDate", {}), ("clearInitials", {}), ("replaceParagraph", {"index":0,"value":"Revised"}), ("insertParagraph", {"index":1,"value":"New"}),
                   ("replaceParagraph", {"index":1, "value":"Updated"}), ("insertParagraph", {"index":2,"value":"Temporary"}),
                   ("removeParagraph", {"index":2})]:
        output = ImportedDocument.open(output).edit_entry(owner,id,op,**kw)
    result, = docx.Document(io.BytesIO(output)).comments
    assert [p.text for p in result.paragraphs] == ["Revised","Updated"]
    assert result.author == "Reviewer" and result.initials is None
    with zipfile.ZipFile(io.BytesIO(initial)) as before, zipfile.ZipFile(io.BytesIO(output)) as after:
        assert {p for p in before.namelist() if before.read(p) != after.read(p)} == {owner}
        root = etree.fromstring(after.read(owner))
        assert root[0].get(W+"date") is None and root[0].find(f".//{W}annotationRef") is not None
