"""Document page-layout flags preserve settings and physical section geometry."""
import io
import zipfile

import docx
from lxml import etree

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def settings(data):
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/settings.xml"))


async def test_page_layout_defaults_preserve_omission_and_merge_explicit_false(tmp_path):
    base = await d.Document([]).to_bytes()
    assert settings(base).find(f"{W}mirrorMargins") is None
    assert settings(base).find(f"{W}gutterAtTop") is None
    template = tmp_path / "page-layout.docx"
    initial = d.Document([d.Section([], properties=d.SectionProperties(
        page_margins={"left": 685800, "right": 457200},
    ))], defaults=d.DocumentDefaults(
        mirror_margins=True, gutter_at_top=True, footnote_position="beneathText",
    ))
    await initial.save(template)
    data = await d.Document([], defaults=d.DocumentDefaults(
        gutter_at_top=lambda: False,
    )).to_bytes(d.DocumentTemplate(template))
    root = settings(data)
    assert root.find(f"{W}mirrorMargins").get(f"{W}val") == "1"
    assert root.find(f"{W}gutterAtTop").get(f"{W}val") == "0"
    assert root.find(f"{W}footnotePr/{W}pos").get(f"{W}val") == "beneathText"
    section = docx.Document(io.BytesIO(data)).sections[-1]
    assert section.left_margin == 685800
    assert section.right_margin == 457200
