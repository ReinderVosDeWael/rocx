"""Independent developer validation against edition-pinned ECMA Transitional XSDs.

Run through uv's test group. This tool is not imported by the document engine.
It checks base schemas after supported MCE preprocessing, OPC targets, and r:*
references, including standard DrawingML chart links. It is not Word layout
validation or a validator for every extension.
"""
import argparse
from collections.abc import Iterable
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
C = "http://schemas.openxmlformats.org/drawingml/2006/chart"
CUSTOM = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"
EXTENDED = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
VT = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
CT = "http://schemas.openxmlformats.org/package/2006/content-types"
CHART_TYPE = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
CHART_REL = f"{R}/chart"
CUSTOM_TYPE = "application/vnd.openxmlformats-officedocument.custom-properties+xml"
CUSTOM_REL = f"{R}/custom-properties"
EXTENDED_TYPE = "application/vnd.openxmlformats-officedocument.extended-properties+xml"
EXTENDED_REL = f"{R}/extended-properties"
PROPERTY_PARTS = {
    CUSTOM_REL: (CUSTOM_TYPE, CUSTOM, "custom properties"),
    EXTENDED_REL: (EXTENDED_TYPE, EXTENDED, "extended properties"),
}
BASE = {W, A, C, CUSTOM, EXTENDED, VT, R, XML,
        "http://schemas.openxmlformats.org/drawingml/2006/picture",
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
        "http://schemas.openxmlformats.org/officeDocument/2006/math"}


class ValidationError(ValueError):
    pass


def default_directory():
    return Path(os.environ.get("ROCX_SCHEMA_DIR", Path(tempfile.gettempdir()) / "rocx-validation-schemas"))


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
        uri = XML if prefix == "xml" else element.nsmap.get(prefix)
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


def content_types(parts: dict[str, bytes], canonical: dict[str, str]) -> dict[str, str | None]:
    manifest = canonical.get("[content_types].xml")
    if manifest is None:
        return {}
    root = etree.fromstring(parts[manifest], parser())
    overrides = {node.get("PartName", "").removeprefix("/").lower(): node.get("ContentType", "").lower()
                 for node in root.findall(f"{{{CT}}}Override")}
    defaults = {node.get("Extension", "").lower(): node.get("ContentType", "").lower()
                for node in root.findall(f"{{{CT}}}Default")}
    return {name: overrides.get(key, defaults.get(posixpath.splitext(key)[1][1:]))
            for key, name in canonical.items() if name != manifest}


def validate_custom_ids(root: etree._Element, owner: str) -> None:
    ids = set()
    for node in root.findall(f"{{{CUSTOM}}}property"):
        identifier = int(node.get("pid"))  # The XSD has already checked its lexical form and range.
        if identifier < 2 or identifier in ids:
            raise ValidationError(f"invalid or duplicate custom property ID {identifier} in {owner}")
        ids.add(identifier)


def validate_chart_references(root: etree._Element, owner: str, relations: dict[str, etree._Element]) -> None:
    """Check the chart-specific payload and edge beyond the graphicData XSD."""
    for payload in root.iter(f"{{{A}}}graphicData"):
        children = [child for child in payload if isinstance(child.tag, str)]
        references = [child for child in children if child.tag == f"{{{C}}}chart"]
        if payload.get("uri", "").strip() != C:
            if references:
                raise ValidationError(f"incorrect chart graphicData URI in {owner}")
            continue
        if len(references) != 1 or len(children) != 1:
            raise ValidationError(f"expected one chart reference in {owner}")
        identifier = references[0].get(f"{{{R}}}id")
        relation = relations.get(identifier) if identifier is not None else None
        if not identifier or relation is None or relation.get("Type") != CHART_REL:
            raise ValidationError(f"missing owner-local chart relationship for {identifier!r} in {owner}")


def validate_note_graph(parts: dict[str, bytes], relationships: dict[str, dict[str, etree._Element]]) -> None:
    """Check semantic note IDs independently of the Rust catalog and base XSDs."""
    def tree(path: str) -> etree._Element:
        root = etree.fromstring(parts[path], parser())
        preprocess(root)
        return root

    def identifier(node: etree._Element, owner: str) -> str:
        value = node.get(f"{{{W}}}id", "").strip()
        negative = value.startswith("-")
        digits = value[1:] if value.startswith(("-", "+")) else value
        if not digits or any(c not in "0123456789" for c in digits):
            raise ValidationError(f"invalid note ID in {owner}")
        digits = digits.lstrip("0") or "0"
        return f"-{digits}" if negative and digits != "0" else digits

    roots = [target_of("", r.get("Target", "")) for r in relationships.get("", {}).values()
             if r.get("Type") == f"{R}/officeDocument" and r.get("TargetMode", "Internal") == "Internal"]
    canonical = {name.lower(): name for name in parts}
    for main in roots:
        main = canonical[main.lower()]
        definitions = {}
        note_parts = []
        related_kinds = set()
        main_relations = relationships.get(main, {})
        for kind in ("footnote", "endnote"):
            matches = [r for r in main_relations.values() if r.get("Type") == f"{R}/{kind}s"]
            if len(matches) > 1:
                raise ValidationError(f"multiple {kind} parts for {main}")
            for relation in matches:
                if relation.get("TargetMode", "Internal") != "Internal":
                    raise ValidationError(f"external {kind} part relationship")
                owner = canonical[target_of(main, relation.get("Target", "")).lower()]
                root = tree(owner)
                if root.tag != f"{{{W}}}{kind}s":
                    raise ValidationError(f"incorrect {kind} root in {owner}")
                related_kinds.add(kind)
                for note in root.findall(f"{{{W}}}{kind}"):
                    key = kind, identifier(note, owner)
                    if key in definitions:
                        raise ValidationError(f"duplicate note ID in {owner}")
                    definitions[key] = note.get(f"{{{W}}}type", "normal") == "normal"
                note_parts.append((owner, root))
        for relation in main_relations.values():
            if relation.get("Type") == f"{R}/settings" and relation.get("TargetMode", "Internal") == "Internal":
                owner = canonical[target_of(main, relation.get("Target", "")).lower()]
                settings = tree(owner)
                for kind in related_kinds:
                    for reference in settings.findall(f"{{{W}}}{kind}Pr/{{{W}}}{kind}"):
                        if definitions.get((kind, identifier(reference, owner))) is not False:
                            raise ValidationError(f"note settings must target special entries in {owner}")
        owners = [(main, tree(main), True)]
        for relation in main_relations.values():
            if any(relation.get("Type") == f"{R}/{kind}" for kind in ("header", "footer", "comments")):
                if relation.get("TargetMode", "Internal") == "Internal":
                    owner = canonical[target_of(main, relation.get("Target", "")).lower()]
                    owners.append((owner, tree(owner), True))
        owners.extend((owner, root, False) for owner, root in note_parts)
        for owner, root, allowed in owners:
            for node in root.iter():
                for kind in ("footnote", "endnote"):
                    if node.tag != f"{{{W}}}{kind}Reference":
                        continue
                    if not allowed:
                        raise ValidationError(f"nested note reference in {owner}")
                    if definitions.get((kind, identifier(node, owner))) is not True:
                        raise ValidationError(f"missing normal note target or reference to special entry in {owner}")


def validate_field_structure(root: etree._Element, owner: str) -> None:
    """Check balanced complex fields per story, independently of the native codec.

    The base XSD cannot pair markers. Simple fields delimit their own cached
    content; note/comment entries sharing a part are independent stories.
    """
    def story(nodes: Iterable[etree._Element]) -> None:
        stack: list[bool] = []

        def visit(current: etree._Element) -> None:
            if current.tag == f"{{{W}}}fldSimple":
                story(current)
                return
            if current.tag == f"{{{W}}}fldChar":
                kind = current.get(f"{{{W}}}fldCharType")
                if kind == "begin":
                    stack.append(False)
                elif kind == "separate" and stack and not stack[-1]:
                    stack[-1] = True
                elif kind == "end" and stack:
                    stack.pop()
                else:
                    raise ValidationError(f"unbalanced field marker in {owner}")
            if current.tag == f"{{{W}}}instrText" and (not stack or stack[-1]):
                raise ValidationError(f"field instruction outside field codes in {owner}")
            for child in current:
                visit(child)

        for node in nodes:
            visit(node)
        if stack:
            raise ValidationError(f"unclosed field in {owner}")

    if root.tag in {f"{{{W}}}footnotes", f"{{{W}}}endnotes", f"{{{W}}}comments"}:
        for entry in root:
            story([entry])
    else:
        story([root])


class Validator:
    def __init__(self, directory):
        directory = Path(directory)
        self._temporary = tempfile.TemporaryDirectory(prefix="rocx-xsd-")
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
            C: etree.XMLSchema(etree.parse(str(prepared / "dml-chart.xsd"), parser())),
            CUSTOM: etree.XMLSchema(etree.parse(str(prepared / "shared-documentPropertiesCustom.xsd"), parser())),
            EXTENDED: etree.XMLSchema(etree.parse(str(prepared / "shared-documentPropertiesExtended.xsd"), parser())),
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
        media = content_types(parts, canonical)
        charts = {name for name, kind in media.items() if kind == CHART_TYPE}
        property_types = {mime: (namespace, label) for mime, namespace, label in PROPERTY_PARTS.values()}
        properties = {name: property_types[kind] for name, kind in media.items() if kind in property_types}
        linked_properties = {}
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
                internal = relation.get("TargetMode", "Internal") == "Internal"
                is_chart = relation.get("Type") == CHART_REL
                if is_chart and not internal:
                    raise ValidationError(f"chart relationship {identifier} in {name} must be internal")
                kind = relation.get("Type")
                property_spec = PROPERTY_PARTS.get(kind)
                if property_spec is not None and (owner or not internal):
                    _, _, label = property_spec
                    raise ValidationError(f"{label} relationship in {name} must be internal and package-level")
                if internal:
                    target = target_of(owner, relation.get("Target", ""))
                    if target.lower() not in canonical:
                        raise ValidationError(f"missing relationship target {target}")
                    if is_chart and canonical[target.lower()] not in charts:
                        raise ValidationError(f"incorrect chart content type for {target}")
                    if property_spec is not None:
                        mime, _, label = property_spec
                        if kind in linked_properties:
                            raise ValidationError(f"multiple package-level {label} relationships")
                        actual = canonical[target.lower()]
                        if media.get(actual) != mime:
                            raise ValidationError(f"incorrect {label} content type for {target}")
                        linked_properties[kind] = actual
            relationships[owner] = ids
        linked_paths = set(linked_properties.values())
        for path, (_, label) in properties.items():
            if path not in linked_paths:
                raise ValidationError(f"{label} part {path} has no package-level relationship")
        checked = []
        for name, data in parts.items():
            if not name.endswith(".xml") and name not in charts and name not in properties:
                continue
            element = etree.fromstring(data, parser())
            if name in charts and element.tag != f"{{{C}}}chartSpace":
                raise ValidationError(f"incorrect chart root in {name}: expected chartSpace")
            if name in properties:
                expected, label = properties[name]
                if element.tag != f"{{{expected}}}Properties":
                    raise ValidationError(f"incorrect {label} root in {name}")
            namespace = etree.QName(element).namespace
            schema = self.schemas.get(namespace)
            if (schema is None or (namespace == C and name not in charts)
                    or (namespace in {CUSTOM, EXTENDED} and name not in properties)):
                continue
            base = deepcopy(element)
            preprocess(base)
            relations = relationships.get(name, {})
            validate_chart_references(base, name, relations)
            for node in element.iter():
                if not isinstance(node.tag, str):
                    continue
                for attr, value in node.attrib.items():
                    if etree.QName(attr).namespace == R and value not in relations:
                        raise ValidationError(f"undeclared relationship reference {value} in {name}")
            if not schema.validate(base):
                raise ValidationError(f"{name}: {schema.error_log.last_error}")
            if etree.QName(base).namespace == W:
                validate_field_structure(base, name)
            if namespace == CUSTOM:
                validate_custom_ids(base, name)
            checked.append(name)
        if not checked:
            raise ValidationError("no supported WordprocessingML/DrawingML parts were validated")
        validate_note_graph(parts, relationships)
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
