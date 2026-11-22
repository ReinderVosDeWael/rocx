"""Custom property authoring and relationship-driven template updates."""
import io
from zipfile import ZipFile

from lxml import etree
import pytest

from rocx import declarative as d

CP = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"
VT = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties"
DEFAULT = "docProps/custom.xml"
RELOCATED = "metadata/local/custom.xml"


def relocate(data, change, relocated=RELOCATED):
    result = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as old, ZipFile(result, "w") as new:
        for path in old.namelist():
            payload = old.read(path)
            if path in {"_rels/.rels", "[Content_Types].xml"}:
                root = etree.fromstring(payload)
                for child in root:
                    if child.get("Type") == REL:
                        child.set("Target", f"/{relocated}")
                    if child.get("PartName") == f"/{DEFAULT}":
                        child.set("PartName", f"/{relocated}")
                payload = etree.tostring(root)
            if path == DEFAULT:
                path = relocated
                payload = change(etree.fromstring(payload))
            new.writestr(path, payload)
    return result.getvalue()


async def test_custom_properties_are_deferred_sorted_and_allow_empty_strings():
    timestamp = "2026-01-01T00:00:00Z"
    omitted = await d.Document([], timestamp=timestamp).to_bytes()
    assert omitted == await d.Document([], timestamp=timestamp, custom_properties={}).to_bytes()
    calls = []

    async def value():
        calls.append("called")
        return "Café & <tea>"

    snapshot = await d.Document([], custom_properties={"Label": value, "Blank": ""}).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert calls == ["called"]
    with ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read(DEFAULT))
        assert [(p.get("name"), p.get("pid"), p.findtext(f"{{{VT}}}lpwstr")) for p in root] == [
            ("Blank", "2", ""), ("Label", "3", "Café & <tea>"),
        ]


async def test_boolean_custom_properties_are_deferred_and_distinct_from_strings():
    calls = []

    async def approved():
        calls.append("called")
        return False

    snapshot = await d.Document([], custom_properties={
        "Label": "false", "Confirmed": True, "Approved": approved,
    }).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert calls == ["called"]
    with ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read(DEFAULT))
        assert [(p.get("name"), p.get("pid"), p[0].tag, p[0].text) for p in root] == [
            ("Approved", "2", f"{{{VT}}}bool", "false"),
            ("Confirmed", "3", f"{{{VT}}}bool", "true"),
            ("Label", "4", f"{{{VT}}}lpwstr", "false"),
        ]


@pytest.mark.parametrize("invalid", [0, 1, None, ["yes"]])
async def test_non_string_non_boolean_values_are_rejected(invalid):
    with pytest.raises(TypeError, match="strings or booleans"):
        await d.Document([], custom_properties={"Approved": invalid}).to_bytes()


async def test_boolean_template_update_requires_same_unlinked_variant(tmp_path):
    original = await d.Document([], custom_properties={
        "Approved": True, "Count": "7", "Status": "Pending",
    }).to_bytes()

    def from_other_editor(root):
        changed = etree.Element(root.tag, nsmap={None: CP, "vt": "urn:shadow", "v": VT})
        changed.extend(list(root))
        changed[0][0].text = "1"
        changed[1][0].tag = f"{{{VT}}}i4"
        changed[2].set("linkTarget", "StatusBookmark")
        changed[2][0].tag = f"{{{VT}}}bool"
        changed[2][0].text = "true"
        return etree.tostring(changed)

    path = tmp_path / "relocated.docx"
    data = relocate(original, from_other_editor)
    path.write_bytes(data)
    source = d.DocumentTemplate(path)
    edited = await d.Document([], custom_properties={"Approved": False}).to_bytes(source)
    with ZipFile(io.BytesIO(edited)) as package:
        assert DEFAULT not in package.namelist()
        root = etree.fromstring(package.read(RELOCATED))
        assert [(p.get("name"), p.get("pid"), p[0].tag, p[0].text) for p in root] == [
            ("Approved", "2", f"{{{VT}}}bool", "false"),
            ("Count", "3", f"{{{VT}}}i4", "7"),
            ("Status", "4", f"{{{VT}}}bool", "true"),
        ]
        assert root[2].get("linkTarget") == "StatusBookmark"

    for name, value, kind in [
        ("Approved", "false", "string"), ("Count", True, "boolean"), ("Status", False, "boolean"),
    ]:
        with pytest.raises(ValueError, match=f"not an unlinked {kind}"):
            await d.Document([], custom_properties={name: value}).save(path, source)
        assert path.read_bytes() == data


@pytest.mark.parametrize("invalid", [{" ": "value"}, {"bad\0name": "value"}, {"good": "bad\0value"}])
async def test_invalid_custom_property_text_is_rejected(invalid):
    with pytest.raises(ValueError, match="custom propert"):
        await d.Document([], custom_properties=invalid).to_bytes()


async def test_template_retains_unrelated_types_and_handles_shadowed_prefix(tmp_path):
    original = await d.Document([], custom_properties={"Count": "7", "Status": "Pending"}).to_bytes()

    def from_other_editor(root):
        changed = etree.Element(root.tag, nsmap={None: CP, "vt": "urn:shadow", "v": VT})
        for property in root:
            changed.append(property)
        changed[0][0].tag = f"{{{VT}}}i4"
        changed[1].set("pid", "7")
        return etree.tostring(changed)

    template = tmp_path / "relocated.docx"
    template.write_bytes(relocate(original, from_other_editor))
    source = d.DocumentTemplate(template)
    untouched = await d.Document([]).to_bytes(source)
    with ZipFile(template) as seed, ZipFile(io.BytesIO(untouched)) as output:
        assert seed.read(RELOCATED) == output.read(RELOCATED)

    document = d.Document([], custom_properties={"Project": "Atlas", "Status": "Draft"})
    edited = await document.to_bytes(source)
    with ZipFile(io.BytesIO(edited)) as package:
        assert DEFAULT not in package.namelist()
        root = etree.fromstring(package.read(RELOCATED))
        assert [(p.get("name"), p.get("pid"), p[0].tag, p[0].text) for p in root] == [
            ("Count", "2", f"{{{VT}}}i4", "7"),
            ("Status", "7", f"{{{VT}}}lpwstr", "Draft"),
            ("Project", "3", f"{{{VT}}}lpwstr", "Atlas"),
        ]
    with pytest.raises(ValueError, match="not an unlinked string"):
        await d.Document([], custom_properties={"Count": "eight"}).to_bytes(source)


@pytest.mark.parametrize(("mutation", "diagnostic"), [
    ("Linked", "not an unlinked string"),
    ("Ambiguous", "ambiguous imported custom property"),
    ("Duplicate ID", "duplicate imported custom property ID"),
])
async def test_conflicting_imported_identity_is_not_rewritten(tmp_path, mutation, diagnostic):
    original = await d.Document([], custom_properties={"Status": "Pending", "Other": "Keep"}).to_bytes()

    def conflict(root):
        if mutation == "Linked":
            root[1].set("linkTarget", "StatusBookmark")
        elif mutation == "Ambiguous":
            root[0].set("name", "Status")
        else:
            root[0].set("pid", root[1].get("pid"))
        return etree.tostring(root)

    source = tmp_path / "conflict.docx"
    data = relocate(original, conflict)
    source.write_bytes(data)
    template = d.DocumentTemplate(source)
    request = d.Document([], custom_properties={"Status": "Draft"})
    with pytest.raises(ValueError, match=diagnostic):
        await request.save(source, template)
    assert source.read_bytes() == data
