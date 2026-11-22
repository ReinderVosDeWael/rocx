"""Template guard for root Markup Compatibility namespace requirements."""
import io
from zipfile import ZipFile

from lxml import etree
import pytest

from rocx import declarative as d

MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
FOREIGN = "urn:example:mandatory-extension"
MAIN = "word/document.xml"


async def with_root(tmp_path, requirement, binding, scope="root"):
    data = await d.Document([]).to_bytes()
    result = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(result, "w") as target:
        for path in source.namelist():
            payload = source.read(path)
            if path == MAIN:
                old = etree.fromstring(payload)
                root = etree.Element(old.tag, nsmap={"main": W, "compat": MC, **binding})
                root.extend(old)
                required = root
                if scope != "root":
                    body = root.find(f"{{{W}}}body")
                    if scope == "alternate":
                        alternate = etree.Element(f"{{{MC}}}AlternateContent")
                        body.insert(0, alternate)
                        body = etree.SubElement(alternate, f"{{{MC}}}Choice", Requires="ext")
                        etree.SubElement(alternate, f"{{{MC}}}Fallback")
                    required = etree.Element(f"{{{W}}}p")
                    body.insert(0, required)
                required.set(f"{{{MC}}}MustUnderstand", requirement)
                payload = etree.tostring(root)
                if scope == "shadowed":
                    # Preserve the paragraph's expanded name while shadowing
                    # the prefix used in the QName-valued requirement.
                    payload = payload.replace(b"<main:p ", f'<leaf:p xmlns:leaf="{W}" xmlns:main="{FOREIGN}" '.encode(), 1)
            target.writestr(path, payload)
    path = tmp_path / "source.docx"
    path.write_bytes(result.getvalue())
    return d.DocumentTemplate(path)


@pytest.mark.parametrize(("prefix", "bindings", "diagnostic"), [
    ("ext", {"ext": FOREIGN}, "unsupported mandatory namespace"),
    ("w", {"w": FOREIGN}, "unsupported mandatory namespace"),
    ("w", {}, "unbound prefix"),
])
async def test_rejects_unsupported_or_missing_root_bindings(tmp_path, prefix, bindings, diagnostic):
    template = await with_root(tmp_path, prefix, bindings)
    with pytest.raises(ValueError, match=diagnostic):
        await d.Document([]).to_bytes(template)


async def test_accepts_recognized_namespace_through_an_alias(tmp_path):
    template = await with_root(tmp_path, "main xml", {})
    output = await d.Document([]).to_bytes(template)
    with ZipFile(io.BytesIO(output)) as package:
        root = etree.fromstring(package.read(MAIN))
        assert root.get(f"{{{MC}}}MustUnderstand") == "main xml"
        assert root.nsmap["main"] == W


async def test_wordprocessingml_descendant_uses_local_shadowing(tmp_path):
    template = await with_root(tmp_path, "main", {}, scope="shadowed")
    with pytest.raises(ValueError, match="unsupported mandatory namespace"):
        await d.Document([]).to_bytes(template)


async def test_preserves_an_opaque_alternative_without_processing_it(tmp_path):
    template = await with_root(tmp_path, "ext", {"ext": FOREIGN}, scope="alternate")
    output = await d.Document([]).to_bytes(template)
    with ZipFile(io.BytesIO(output)) as package:
        root = etree.fromstring(package.read(MAIN))
        choice = root.find(f".//{{{MC}}}Choice")
        assert choice.get("Requires") == "ext"
        assert choice.nsmap["ext"] == FOREIGN
        assert choice[0].get(f"{{{MC}}}MustUnderstand") == "ext"
