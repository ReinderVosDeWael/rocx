"""Independent developer validation against edition-pinned ECMA Transitional XSDs.

Run through uv's test group. This tool is not imported by the document engine.
It checks base schemas after supported MCE preprocessing, OPC targets, and r:*
references. It is not Word layout validation or a validator for every extension.
"""
import argparse
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import posixpath
import tempfile
from urllib.request import urlopen
from zipfile import ZipFile

from lxml import etree

SOURCES = {
    "ecma": ("https://ecma-international.org/wp-content/uploads/ECMA-376-4_5th_edition_december_2016.zip", "bd25da1109f73762356596918bf5ff8b74a1331642dba5f1c1d1dfc6bed34ecd"),
    "xml": ("https://www.w3.org/2001/xml.xsd", "61960fb3131e38022caad5360e2f33a3382578ab3c80cd58bd74320ede61b20c"),
}
XML = "http://www.w3.org/XML/1998/namespace"
XS = "http://www.w3.org/2001/XMLSchema"
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
BASE = {W, A, R, XML,
        "http://schemas.openxmlformats.org/drawingml/2006/picture",
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
        "http://schemas.openxmlformats.org/officeDocument/2006/math"}


class ValidationError(ValueError):
    pass


def default_directory():
    return Path(os.environ.get("RV_DOCX_SCHEMA_DIR", Path(tempfile.gettempdir()) / "rv-docx-validation-schemas"))


def download(directory):
    directory.mkdir(parents=True, exist_ok=True)
    payloads = {}
    for key, (url, digest) in SOURCES.items():
        with urlopen(url, timeout=45) as response:
            data = response.read()
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValidationError(f"{key} specification download changed; review its source and digest")
        payloads[key] = data
    with ZipFile(io.BytesIO(payloads["ecma"])) as archive:
        with ZipFile(io.BytesIO(archive.read("OfficeOpenXML-XMLSchema-Transitional.zip"))) as schemas:
            for name in schemas.namelist():
                if name.endswith(".xsd"):
                    (directory / Path(name).name).write_bytes(schemas.read(name))
    (directory / "xml.xsd").write_bytes(payloads["xml"])
    (directory / "SOURCE.json").write_text(json.dumps(SOURCES, indent=2) + "\n")


def parser():
    return etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)


def prefix_uris(element, value):
    result = set()
    for prefix in value.split():
        uri = element.nsmap.get(prefix)
        if uri is None:
            raise ValidationError(f"undeclared MCE prefix {prefix}")
        result.add(uri)
    return result


def preprocess(element, inherited=frozenset(), process=frozenset()):
    """P3 §§7–9: choose known alternatives and remove ignorable base-unknown markup."""
    ignorable = inherited | prefix_uris(element, element.get(f"{{{MC}}}Ignorable", ""))
    must = prefix_uris(element, element.get(f"{{{MC}}}MustUnderstand", ""))
    if must - BASE:
        raise ValidationError(f"unsupported mandatory extension: {must - BASE}")
    process = set(process)
    for token in element.get(f"{{{MC}}}ProcessContent", "").split():
        prefix, local = token.split(":", 1)
        uri = element.nsmap.get(prefix)
        if uri is None:
            raise ValidationError(f"undeclared ProcessContent prefix {prefix}")
        process.add((uri, local))
    for name in list(element.attrib):
        uri = etree.QName(name).namespace
        if uri == MC or uri in ignorable - BASE:
            del element.attrib[name]
    for child in list(element):
        if not isinstance(child.tag, str):
            continue
        name = etree.QName(child)
        if name.namespace == MC and name.localname == "AlternateContent":
            choices = child.findall(f"{{{MC}}}Choice")
            selected = next((choice for choice in choices if prefix_uris(choice, choice.get("Requires", "")) <= BASE), None)
            if selected is None:
                selected = child.find(f"{{{MC}}}Fallback")
            if selected is None:
                raise ValidationError("AlternateContent has no understood choice or fallback")
            preprocess(selected, ignorable, process)
            index = element.index(child)
            for nested in list(selected):
                element.insert(index, nested)
                index += 1
            element.remove(child)
        elif name.namespace in ignorable - BASE:
            if (name.namespace, name.localname) in process:
                preprocess(child, ignorable, process)
                index = element.index(child)
                for nested in list(child):
                    element.insert(index, nested)
                    index += 1
            element.remove(child)
        else:
            preprocess(child, ignorable, process)


def owner_of(relationship_part):
    if relationship_part == "_rels/.rels":
        return ""
    directory, filename = relationship_part.rsplit("_rels/", 1)
    return directory + filename[:-5]


def target_of(owner, target):
    if "\\" in target or "?" in target or "#" in target:
        raise ValidationError(f"invalid internal target {target}")
    path = posixpath.normpath(target.lstrip("/") if target.startswith("/") else posixpath.join(posixpath.dirname(owner), target))
    if path == ".." or path.startswith("../"):
        raise ValidationError(f"target escapes package root: {target}")
    return path


class Validator:
    def __init__(self, directory):
        directory = Path(directory)
        self._temporary = tempfile.TemporaryDirectory(prefix="rv-docx-xsd-")
        prepared = Path(self._temporary.name)
        # ECMA imports the XML namespace without a location. Supply the official
        # W3C schema locally, without changing OOXML types/content models.
        xml_schema = directory / "xml.xsd"
        if not xml_schema.exists():
            xml_schema = directory.parent / "xml.xsd"
        if not xml_schema.exists():
            raise ValidationError("xml.xsd is missing; use --download-schemas")
        (prepared / "xml.xsd").write_bytes(xml_schema.read_bytes())
        for source in directory.glob("*.xsd"):
            if source.name == "xml.xsd":
                continue
            tree = etree.parse(str(source), parser())
            for imported in tree.findall(f"{{{XS}}}import"):
                if imported.get("namespace") == XML and not imported.get("schemaLocation"):
                    imported.set("schemaLocation", "xml.xsd")
            tree.write(str(prepared / source.name), encoding="UTF-8", xml_declaration=True)
        self.schemas = {
            W: etree.XMLSchema(etree.parse(str(prepared / "wml.xsd"), parser())),
            A: etree.XMLSchema(etree.parse(str(prepared / "dml-main.xsd"), parser())),
        }

    def validate(self, path_or_bytes):
        stream = io.BytesIO(path_or_bytes) if isinstance(path_or_bytes, bytes) else path_or_bytes
        with ZipFile(stream) as archive:
            if sum(entry.file_size for entry in archive.infolist()) > 256 * 1024 * 1024:
                raise ValidationError("expanded package exceeds validator budget")
            names = [name for name in archive.namelist() if not name.endswith("/")]
            if len(names) != len(set(names)):
                raise ValidationError("duplicate ZIP item names")
            parts = {name: archive.read(name) for name in names}
        canonical = {name.lower(): name for name in parts}
        if len(canonical) != len(parts):
            raise ValidationError("duplicate equivalent part names")
        relationships = {}
        for name, data in parts.items():
            if not name.endswith(".rels"):
                continue
            owner = owner_of(name)
            if owner and owner.lower() not in canonical:
                raise ValidationError(f"missing relationship owner {owner}")
            tree = etree.fromstring(data, parser())
            ids = {}
            for relation in tree.findall(f"{{{REL}}}Relationship"):
                identifier = relation.get("Id")
                if not identifier or identifier in ids:
                    raise ValidationError(f"duplicate or missing relationship ID in {name}")
                ids[identifier] = relation
                if relation.get("TargetMode", "Internal") == "Internal":
                    target = target_of(owner, relation.get("Target", ""))
                    if target.lower() not in canonical:
                        raise ValidationError(f"missing relationship target {target}")
            relationships[owner] = ids
        checked = []
        for name, data in parts.items():
            if not name.endswith(".xml"):
                continue
            element = etree.fromstring(data, parser())
            schema = self.schemas.get(etree.QName(element).namespace)
            if schema is None:
                continue
            for node in element.iter():
                if not isinstance(node.tag, str):
                    continue
                for attr, value in node.attrib.items():
                    if etree.QName(attr).namespace == R and value not in relationships.get(name, {}):
                        raise ValidationError(f"undeclared relationship reference {value} in {name}")
            base = deepcopy(element)
            preprocess(base)
            if not schema.validate(base):
                raise ValidationError(f"{name}: {schema.error_log.last_error}")
            checked.append(name)
        if not checked:
            raise ValidationError("no supported WordprocessingML/DrawingML parts were validated")
        return checked


def main():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("documents", nargs="*", type=Path)
    arguments.add_argument("--schema-dir", type=Path, default=default_directory())
    arguments.add_argument("--download-schemas", action="store_true")
    options = arguments.parse_args()
    if options.download_schemas:
        download(options.schema_dir)
    if options.documents:
        validator = Validator(options.schema_dir)
        for document in options.documents:
            checked = validator.validate(document)
            print(f"Validated {document}: {len(checked)} schema-covered parts and OPC relationships")


if __name__ == "__main__":
    main()
