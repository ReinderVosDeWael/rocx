"""Selected extended properties, independent of native XML readback."""
import io
from zipfile import ZipFile

import pytest
from lxml import etree

from rocx import declarative as d

NS = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties"
APP = "docProps/app.xml"


def move_app(data, relocated):
    """Relocate the app part, or remove the part and relationship entirely."""
    result = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(result, "w") as target:
        for name in source.namelist():
            payload = source.read(name)
            if name in {"_rels/.rels", "[Content_Types].xml"}:
                root = etree.fromstring(payload)
                for node in list(root):
                    if node.get("Type") == REL:
                        if relocated is None:
                            root.remove(node)
                        else:
                            node.set("Target", f"/{relocated}")
                    if node.get("PartName") == f"/{APP}":
                        if relocated is None:
                            root.remove(node)
                        else:
                            node.set("PartName", f"/{relocated}")
                payload = etree.tostring(root)
            if name == APP:
                if relocated is None:
                    continue
                name = relocated
            target.writestr(name, payload)
    return result.getvalue()


async def test_relocated_extended_properties_update_only_requested_fields(tmp_path):
    original = await d.Document([], company="Atlas", manager="Taylor").to_bytes()
    relocated = "property-bag/extended.xml"
    template = tmp_path / "extended.docx"
    template.write_bytes(move_app(original, relocated))
    document = d.Document([], company=lambda: "")
    data = await document.to_bytes(d.DocumentTemplate(template))
    with ZipFile(template) as source, ZipFile(io.BytesIO(data)) as result:
        assert APP not in result.namelist()
        before, after = [etree.fromstring(package.read(relocated)) for package in (source, result)]
        assert after.findtext(f"{{{NS}}}Company") == ""
        unchanged = lambda root: [(node.tag, node.text) for node in root if node.tag != f"{{{NS}}}Company"]
        assert unchanged(before) == unchanged(after)
        assert after.findtext(f"{{{NS}}}Manager") == "Taylor"


async def test_creates_extended_part_only_when_requested(tmp_path):
    original = await d.Document([]).to_bytes()
    template = tmp_path / "without-app.docx"
    template.write_bytes(move_app(original, None))
    source = d.DocumentTemplate(template)
    with ZipFile(io.BytesIO(await d.Document([]).to_bytes(source))) as package:
        assert APP not in package.namelist()
    with ZipFile(io.BytesIO(await d.Document([], manager="Café & <tea>").to_bytes(source))) as package:
        root = etree.fromstring(package.read(APP))
        assert [(node.tag, node.text) for node in root] == [(f"{{{NS}}}Manager", "Café & <tea>")]
        relations = etree.fromstring(package.read("_rels/.rels"))
        assert [n.get("Target") for n in relations if n.get("Type") == REL] == [APP]


async def test_extended_properties_require_xml_text():
    with pytest.raises(ValueError, match=r"metadata\.manager"):
        await d.Document([], manager="bad\0value").to_bytes()
