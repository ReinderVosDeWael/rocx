"""Fresh sections own their declared stories and inherit omitted stories."""

import io
import posixpath
from xml.etree import ElementTree as ET
from zipfile import ZipFile

import docx
import pytest

from rocx import declarative as d


W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


def section(label, declared=True):
    return d.Section(
        [d.Paragraph(text=f"Body {label}")],
        headers={"default": d.Header([d.Paragraph(text=f"Header {label}")])} if declared else {},
        footers={"default": d.Footer([d.Paragraph(text=f"Footer {label}")])} if declared else {},
    )


@pytest.fixture(params=[True, False], ids=["own-stories", "inherited-stories"])
async def two_sections(request):
    declared = request.param
    return await d.Document([section("A"), section("B", declared)]).to_bytes(), declared


def story_text(story):
    # Newly created parts also contain an empty styled paragraph.
    return [p.text for p in story.paragraphs if p.text]


@pytest.mark.parametrize("kind", ["header", "footer"])
def test_fresh_section_stories_read_back_independently(two_sections, kind):
    data, declared = two_sections
    first, second = docx.Document(io.BytesIO(data)).sections
    first_story, second_story = getattr(first, kind), getattr(second, kind)
    assert not first_story.is_linked_to_previous
    assert second_story.is_linked_to_previous == (not declared)
    assert story_text(first_story) == [f"{kind.title()} A"]
    assert story_text(second_story) == [f"{kind.title()} {'B' if declared else 'A'}"]


@pytest.mark.parametrize("kind", ["header", "footer"])
def test_fresh_section_story_relationships(two_sections, kind):
    data, declared = two_sections
    with ZipFile(io.BytesIO(data)) as package:
        document = ET.fromstring(package.read("word/document.xml"))
        sections = document.findall(
            f"./{{{W}}}body/{{{W}}}p/{{{W}}}pPr/{{{W}}}sectPr"
        ) + document.findall(f"./{{{W}}}body/{{{W}}}sectPr")
        assert len(sections) == 2
        rels = ET.fromstring(package.read("word/_rels/document.xml.rels"))
        story_rels = {
            rel.attrib["Id"]: rel.attrib["Target"]
            for rel in rels
            if rel.attrib["Type"] == f"{R}/{kind}"
        }
        story_ids = []
        for props in sections:
            refs = props.findall(f"{{{W}}}{kind}Reference")
            assert [ref.attrib[f"{{{W}}}type"] for ref in refs] in ([], ["default"])
            story_ids.append([ref.attrib[f"{{{R}}}id"] for ref in refs])
        first_ids, second_ids = story_ids
        assert len(first_ids) == 1
        assert len(second_ids) == int(declared)
        assert set(first_ids + second_ids) == story_rels.keys()
        assert len(set(story_rels.values())) == 1 + int(declared)
        for target in story_rels.values():
            part = posixpath.normpath(posixpath.join("word", target))
            expected_root = "hdr" if kind == "header" else "ftr"
            assert ET.fromstring(package.read(part)).tag == f"{{{W}}}{expected_root}"


async def test_template_stories_still_append(tmp_path):
    template = tmp_path / "independent.docx"
    source = docx.Document()
    source.sections[0].header.paragraphs[0].text = "Header template"
    source.sections[0].footer.paragraphs[0].text = "Footer template"
    source.save(template)

    data = await d.Document([section("A")]).to_bytes(d.DocumentTemplate(template))
    output = docx.Document(io.BytesIO(data)).sections[0]
    assert story_text(output.header) == ["Header template", "Header A"]
    assert story_text(output.footer) == ["Footer template", "Footer A"]
