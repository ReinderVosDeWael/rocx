"""Shared compiler policy for OPC signature origin relationships."""
import io
from zipfile import ZipFile

from lxml import etree
import pytest

from rocx import declarative as d

REL = "http://schemas.openxmlformats.org/package/2006/relationships"
CT = "http://schemas.openxmlformats.org/package/2006/content-types"
ORIGIN_TYPE = f"{REL}/digital-signature/origin"


def with_origin(data, target, relationship_type, media_type):
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w") as target_zip:
        for path in source.namelist():
            payload = source.read(path)
            if path == "_rels/.rels":
                root = etree.fromstring(payload)
                etree.SubElement(root, f"{{{REL}}}Relationship", Id="signatureOrigin",
                                 Type=relationship_type, Target=f"/{target}")
                payload = etree.tostring(root)
            if path == "[Content_Types].xml":
                root = etree.fromstring(payload)
                etree.SubElement(root, f"{{{CT}}}Override", PartName=f"/{target}",
                                 ContentType=media_type)
                payload = etree.tostring(root)
            target_zip.writestr(path, payload)
        target_zip.writestr(target, b"")
    return output.getvalue()


async def test_signature_origin_at_relocated_path_blocks_even_empty_edits(tmp_path):
    document = d.Document([])
    source = tmp_path / "template.docx"
    signed = with_origin(await document.to_bytes(), "custom/relocated.origin", ORIGIN_TYPE,
                         "application/vnd.openxmlformats-package.digital-signature-origin")
    source.write_bytes(signed)
    template = d.DocumentTemplate(source)
    diagnostic = "digitally signed templates are not supported"
    with pytest.raises(ValueError, match=diagnostic):
        await document.to_bytes(template)

    destination = tmp_path / "output.docx"
    destination.write_bytes(b"existing destination")
    with pytest.raises(ValueError, match=diagnostic):
        await document.save(destination, template)
    with pytest.raises(ValueError, match=diagnostic):
        await document.save(source, template)
    assert destination.read_bytes() == b"existing destination"
    assert source.read_bytes() == signed


async def test_signature_like_path_and_relationship_are_not_a_signature_origin(tmp_path):
    document = d.Document([])
    source = tmp_path / "unsigned.docx"
    lookalike = "_xmlsignatures/origin.sigs"
    source.write_bytes(with_origin(await document.to_bytes(), lookalike, f"{ORIGIN_TYPE}-other",
                                   "application/octet-stream"))
    result = await document.to_bytes(d.DocumentTemplate(source))
    with ZipFile(io.BytesIO(result)) as package:
        assert package.read(lookalike) == b""
