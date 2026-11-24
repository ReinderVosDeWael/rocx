"""Generate the native visual catalogs and optionally compare installed JS output.

Run with the development environment: uv run --no-sync python tools/check_examples.py
This uses python-docx/lxml for verification only, never to generate the catalog.
"""
import argparse
import dataclasses
import hashlib
import io
import json
import pathlib
import posixpath
import re
import subprocess
import sys
import zipfile

import docx
from lxml import etree
from rocx import declarative as d

ROOT = pathlib.Path(__file__).resolve().parents[1]
NS = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
      "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
      "c": "http://schemas.openxmlformats.org/drawingml/2006/chart",
      "m": "http://schemas.openxmlformats.org/officeDocument/2006/math",
      "odr": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
      "cust": "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties",
      "app": "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
      "vt": "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes",
      "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
      "r": "http://schemas.openxmlformats.org/package/2006/relationships"}
EDIT_LABEL = "Imported editor: this paragraph changes from left to center in the edited catalog."
EDIT_TABLE_LABEL = "Imported editor table: bold label"
INLINE_LABEL = "Imported editor inline structure: "
INLINE_LINK_RUNS = ["jump to ", "CatalogBookmark"]
HEADER_INLINE_LABEL = "Default-header inline structure: "
HEADER_BATCH_LABEL = "Combined editor batch: this paragraph is centered and followed by a table in both image previews."
HEADER_LINK_RUNS = ["shared ", "link"]
HEADER_LINK_URL = "https://example.invalid/catalog-header"
CATALOG_SUFFIXES = (".docx", ".template.docx", ".edited.docx", ".header-source.docx",
                    ".header-shared.docx", ".header-local.docx", ".header-first-shared.docx",
                    ".header-first-local.docx", ".run-text.docx", ".footer-even.docx", ".entry-text.docx",
                    ".entry-edits.docx", ".story-blocks.docx", ".objects.docx", ".controls.docx")
REGIONS = {"whole_table": None, "first_row": "firstRow", "last_row": "lastRow", "first_column": "firstCol",
    "last_column": "lastCol", "banding_1_row": "band1Horz", "banding_2_row": "band2Horz",
    "banding_1_column": "band1Vert", "banding_2_column": "band2Vert", "top_left_cell": "nwCell",
    "top_right_cell": "neCell", "bottom_left_cell": "swCell", "bottom_right_cell": "seCell"}


def verify_inventory() -> None:
    """Make added public declarations/fields require an explicit coverage update."""
    inventory = json.loads((ROOT / "examples/showcase/coverage.json").read_text())
    for name, expected in inventory.items():
        cls = getattr(d, name)
        actual = [field.name for field in dataclasses.fields(cls) if field.init] if dataclasses.is_dataclass(cls) else list(cls._deferred_fields)
        assert set(actual) == set(expected.split()), f"Update both catalogs and inventory for {name}: {actual}"
    wrappers = {"BlockChildren", "Deferred", "DeferredProperty", "PropertyRef"}
    assert set(d.__all__) == set(inventory) | wrappers, "Update catalog coverage for changed public exports"
    verify_rust_builder_inventory()
    verify_imported_default_keys()


def verify_imported_default_keys() -> None:
    """Keep the two string-key consumers and the published defaults inventory in sync."""
    core = (ROOT / "src/editor/resource_fields/defaults.rs").read_text()
    fields = re.findall(r'^\s*\w+: "(\w+)", (?:RUN|PARA|FOOT|END|SETTINGS),', core, re.M)
    declarations = (ROOT / "javascript/types/inspection.d.ts").read_text()
    groups = re.findall(r"type Default(?:Text|Toggle|Integer) = ([^;]+);", declarations)
    declared = {"tabStops"} | set(re.findall(r'"(\w+)"', " ".join(groups)))
    assert len(fields) == len(declared) and set(fields) == declared, "Update typed default keys"
    guide = (ROOT / "docs/EDITOR-RESOURCES.md").read_text()
    assert all(f"`{field}`" in guide for field in fields), "Document every imported default key"


def verify_rust_builder_inventory() -> None:
    """Require builders and assignment tests for public authoring value fields.

    This source guard covers the repository's flat property records, not arbitrary
    Rust syntax. Compilation and behavioral tests verify the generated methods.
    """
    assignments = (ROOT / "tests/authoring_builders.rs").read_text()
    run_tests = (ROOT / "tests/run_format_builders.rs").read_text()
    for path in (ROOT / "src/document").glob("*.rs"):
        source = path.read_text()
        for record in re.finditer(r"pub struct (\w+) \{([^}]+)\}", source):
            name, body = record.groups()
            if name.startswith("Resolved") or name == "Diagnostic":
                continue  # Immutable snapshots and infrastructure are not authoring records.
            fields = set(re.findall(r"pub (?:r#)?(\w+):", body))
            if not fields:
                continue
            block = re.match(r"\s*setters!\(" + name + r" \{(.*?)\}\);", source[record.end():], re.S)
            assert block, f"Keep {name} builders next to the struct"
            methods = re.findall(r"(with_\w+)\s*=>\s*(?:r#)?(\w+):", block[1])
            assert {field for _, field in methods} == fields, f"Missing {name} builder fields"
            assert all(method == f"with_{field}" for method, field in methods)
            tests = run_tests if name == "RunFormat" else re.search(
                r"check!\(\w+,\s*" + name + r",(.*?)\n\);", assignments, re.S)
            assert tests, f"Add {name} builder assignment tests"
            tested = run_tests if name == "RunFormat" else tests[1]
            assert all(method in tested for method, _ in methods), f"Untested {name} builders"


def exact_bytes(rust: bytes, candidate: bytes, *, language: str = "Python") -> str:
    """Compare the ZIP containers themselves, with part-level failure diagnostics."""
    if rust != candidate:
        with zipfile.ZipFile(io.BytesIO(rust)) as a, zipfile.ZipFile(io.BytesIO(candidate)) as b:
            names = set(a.namelist()) | set(b.namelist())
            changed = [name for name in sorted(names) if name not in a.namelist() or name not in b.namelist() or a.read(name) != b.read(name)]
        raise AssertionError(f"DOCX bytes differ: Rust={len(rust)}, {language}={len(candidate)}; changed parts={changed or 'ZIP container metadata/compression only'}")
    return hashlib.sha256(rust).hexdigest()


def verify_inline_structure(document) -> None:
    paragraphs = document.xpath(
        "/w:document/w:body/w:p[w:r/w:t=$label]", namespaces=NS, label=INLINE_LABEL
    )
    assert len(paragraphs) == 1
    children = paragraphs[0].xpath("*[not(self::w:pPr)]", namespaces=NS)
    assert len(children) == 2
    run, link = children
    assert run.tag == f"{{{NS['w']}}}r"
    assert run.xpath("w:t/text()", namespaces=NS) == [INLINE_LABEL]
    assert link.tag == f"{{{NS['w']}}}hyperlink"
    assert link.xpath("@w:anchor", namespaces=NS) == ["CatalogBookmark"]
    assert not link.xpath("@odr:id", namespaces=NS)
    assert [child.tag for child in link] == [f"{{{NS['w']}}}r"] * len(INLINE_LINK_RUNS)
    assert [child.xpath("w:t/text()", namespaces=NS) for child in link] == [
        [text] for text in INLINE_LINK_RUNS
    ]


def verify_edited(original: bytes, edited: bytes) -> None:
    """Check the labeled body edit and byte preservation of every other part."""
    from rocx.editor import ImportedDocument

    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(io.BytesIO(edited)) as result:
        assert set(source.namelist()) == set(result.namelist())
        for name in source.namelist():
            if name != "word/document.xml":
                assert source.read(name) == result.read(name), name
        for package, expected in [(source, "left"), (result, "center")]:
            document = etree.fromstring(package.read("word/document.xml"))
            verify_inline_structure(document)
            paragraphs = document.xpath(
                "/w:document/w:body/w:p[w:r/w:t=$label]", namespaces=NS, label=EDIT_LABEL
            )
            assert len(paragraphs) == 1
            assert paragraphs[0].xpath("w:pPr/w:jc/@w:val", namespaces=NS) == [expected]
            tables = document.xpath("/w:document/w:body/w:tbl[.//w:t[text()=$label]]",
                                    namespaces=NS, label=EDIT_TABLE_LABEL)
            if expected == "left":
                assert not tables
                continue
            assert len(tables) == 1
            table = tables[0]
            assert paragraphs[0].getnext() is table
            assert table.xpath("w:tblPr/w:tblLayout/@w:type", namespaces=NS) == ["fixed"]
            assert table.xpath("w:tblGrid/w:gridCol/@w:w", namespaces=NS) == ["2160", "2160"]
            rows = table.xpath("w:tr", namespaces=NS)
            assert len(rows) == 1
            cells = rows[0].xpath("w:tc", namespaces=NS)
            assert len(cells) == 2
            assert cells[0].xpath("w:tcPr/w:vAlign/@w:val", namespaces=NS) == ["center"]
            # Compare the editor's direct property projection with the XML
            # from this binding's actual catalog, including the unpatched cell.
            view = ImportedDocument.open(edited).body[table.getparent().index(table)]
            assert view.grid_widths == [2160, 2160]
            assert view.properties["layout"] == {"type": "fixed"}
            assert view.children[0].children[0].properties["verticalAlignment"] == {"val": "center"}
            assert "verticalAlignment" not in view.children[0].children[1].properties
            nested = [t for t in ImportedDocument.open(edited).part_tables() if t.depth == 1 and t.grid_widths == [1440]]
            assert len(nested) == 1 and nested[0].properties["layout"]["type"] == "fixed"
            assert cells[0].xpath(".//w:t/text()", namespaces=NS) == [EDIT_TABLE_LABEL]
            assert cells[0].xpath(".//w:r[w:t=$label]/w:rPr/w:b/@w:val",
                                  namespaces=NS, label=EDIT_TABLE_LABEL) == ["1"]
            second_paragraph, = cells[1].xpath("w:p", namespaces=NS)
            second_runs = second_paragraph.xpath("*[not(self::w:pPr)]", namespaces=NS)
            assert [run.tag for run in second_runs] == [f"{{{NS['w']}}}r"] * 2
            assert [run.xpath("w:t/text()", namespaces=NS) for run in second_runs] == [
                ["Fixed-width "], ["second column"],
            ]


def verify_header_inline_structure(root, rels) -> None:
    """Check direct run and hyperlink boundaries and the owner-local relationship."""
    paragraph, = root.xpath("/w:hdr/w:p[w:r/w:t=$label]", namespaces=NS, label=HEADER_INLINE_LABEL)
    run, link = paragraph.xpath("*[not(self::w:pPr)]", namespaces=NS)
    assert run.tag == f"{{{NS['w']}}}r"
    assert run.xpath("w:t/text()", namespaces=NS) == [HEADER_INLINE_LABEL]
    assert link.tag == f"{{{NS['w']}}}hyperlink"
    assert not link.xpath("@w:anchor", namespaces=NS)
    assert [child.tag for child in link] == [f"{{{NS['w']}}}r"] * len(HEADER_LINK_RUNS)
    assert [child.xpath("w:t/text()", namespaces=NS) for child in link] == [
        [text] for text in HEADER_LINK_RUNS
    ]
    link_id, = link.xpath("@odr:id", namespaces=NS)
    relationship, = [r for r in rels if r.get("Id") == link_id]
    assert relationship.get("Type") == f"{NS['odr']}/hyperlink"
    assert relationship.get("Target") == HEADER_LINK_URL
    assert relationship.get("TargetMode") == "External"


def verify_header_scopes(original: bytes, shared: bytes, local: bytes) -> None:
    """Check combined body/header edits, image scope and untouched payloads."""
    def relationships_name(part):
        directory, filename = posixpath.split(part)
        return posixpath.join(directory, "_rels", f"{filename}.rels")

    def headers(package):
        document = etree.fromstring(package.read("word/document.xml"))
        relationships = etree.fromstring(package.read("word/_rels/document.xml.rels"))
        targets = {r.get("Id"): posixpath.normpath(posixpath.join("word", r.get("Target"))).lstrip("/")
                   for r in relationships}
        sections = document.xpath("//w:sectPr", namespaces=NS)
        assert len(sections) == 2
        effective = []
        current = None
        for section in sections:
            refs = section.xpath("w:headerReference[@w:type='default']/@odr:id", namespaces=NS)
            if refs:
                assert len(refs) == 1
                current = targets[refs[0]]
            assert current
            effective.append(current)
        return effective

    def drawing_count(package, part):
        root = etree.fromstring(package.read(part))
        rels = etree.fromstring(package.read(relationships_name(part)))
        verify_header_inline_structure(root, rels)
        drawings = root.xpath("//w:drawing", namespaces=NS)
        if drawings:
            ids = root.xpath("//w:drawing//a:blip/@odr:embed", namespaces=NS)
            assert len(ids) == len(drawings) == 1
            relationship = [r for r in rels if r.get("Id") == ids[0]]
            assert len(relationship) == 1
            assert relationship[0].get("Type") == f"{NS['odr']}/image"
            target = relationship[0].get("Target")
            image_part = posixpath.normpath(posixpath.join(posixpath.dirname(part), target)).lstrip("/")
            assert package.read(image_part) == (ROOT / "tests/fixtures/images/sample.png").read_bytes()
        return len(drawings)

    for data, edited in [(original, False), (shared, True), (local, True)]:
        verify_inherited_stories(data)
        document = docx.Document(io.BytesIO(data))
        target = document.paragraphs[0]
        assert target.text == HEADER_BATCH_LABEL
        assert target.alignment == (docx.enum.text.WD_ALIGN_PARAGRAPH.CENTER if edited else None)
        if edited:
            table = list(document.iter_inner_content())[1]
            assert len(document.tables) == 1 and isinstance(table, docx.table.Table)
            assert [cell.text for cell in table.rows[0].cells] == [
                EDIT_TABLE_LABEL, "Fixed-width second column",
            ]
            assert table.cell(0, 0).paragraphs[0].runs[0].bold
        else:
            assert not document.tables

    with zipfile.ZipFile(io.BytesIO(original)) as source, \
            zipfile.ZipFile(io.BytesIO(shared)) as shared_zip, \
            zipfile.ZipFile(io.BytesIO(local)) as local_zip:
        original_headers = headers(source)
        assert original_headers[0] == original_headers[1]
        assert drawing_count(source, original_headers[0]) == 0
        shared_headers = headers(shared_zip)
        local_headers = headers(local_zip)
        assert shared_headers == original_headers
        assert local_headers[0] == original_headers[0] and local_headers[1] != local_headers[0]
        assert drawing_count(shared_zip, shared_headers[0]) == 1
        assert drawing_count(local_zip, local_headers[0]) == 0
        assert drawing_count(local_zip, local_headers[1]) == 1

        for result, mutable in [
            (shared_zip, {"word/document.xml", original_headers[0], relationships_name(original_headers[0]),
                          "[Content_Types].xml"}),
            (local_zip, {"word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"}),
        ]:
            source_names, result_names = set(source.namelist()), set(result.namelist())
            assert source_names <= result_names
            for name in source_names - mutable:
                assert source.read(name) == result.read(name), name
            for name in result_names - source_names:
                assert name.startswith("word/media/") or re.fullmatch(
                    r"word/(?:rocx-header\d+\.xml|_rels/(?:rocx-)?header\d+\.xml\.rels)", name
                ), name


def verify_inherited_stories(data: bytes) -> None:
    """The labeled non-default-header slots are explicit once, then inherited."""
    document = docx.Document(io.BytesIO(data))
    first, second = document.sections
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        body = etree.fromstring(package.read("word/document.xml"))
        rels = etree.fromstring(package.read("word/_rels/document.xml.rels"))
        relationships = {r.get("Id"): r for r in rels}
        source, inherited = body.xpath("//w:sectPr", namespaces=NS)
        parts = set()
        for kind in ("header", "footer"):
            for variant, prefix in (("default", ""), ("first", "first_page_"), ("even", "even_page_")):
                if (kind, variant) == ("header", "default"):
                    continue  # Covered separately by verify_header_scopes.
                path = f"w:{kind}Reference[@w:type='{variant}']/@odr:id"
                rel_id, = source.xpath(path, namespaces=NS)
                assert not inherited.xpath(path, namespaces=NS)
                rel = relationships[rel_id]
                assert rel.get("Type") == f"{NS['odr']}/{kind}"
                part = posixpath.normpath(posixpath.join("word", rel.get("Target"))).lstrip("/")
                assert part not in parts
                parts.add(part)
                root = etree.fromstring(package.read(part))
                assert root.tag == f"{{{NS['w']}}}{'hdr' if kind == 'header' else 'ftr'}"
                label = f"Story inspection: {variant} {kind}"
                assert root.xpath("w:p/w:r/w:t/text()", namespaces=NS) == [label]
                explicit, alias = (getattr(s, f"{prefix}{kind}") for s in (first, second))
                assert not explicit.is_linked_to_previous and alias.is_linked_to_previous
                assert explicit.paragraphs[1].text == alias.paragraphs[1].text == label


def verify_plain_workflow(data: bytes) -> None:
    """Read back the same introductory document from Rust, Python or JavaScript."""
    output = docx.Document(io.BytesIO(data))
    blocks = list(output.iter_inner_content())
    assert all(isinstance(block, docx.text.paragraph.Paragraph) for block in blocks)
    assert [paragraph.text for paragraph in blocks] == [
        "Hello, world!", "A second paragraph, with Unicode: café 🌍",
    ]


def verify_javascript_workflow(data: bytes, *, standalone: bool = False) -> None:
    """Read back the report shared by named, legacy and catalog authoring."""
    output = docx.Document(io.BytesIO(data))
    blocks = list(output.iter_inner_content())
    start, = [i for i, block in enumerate(blocks)
              if isinstance(block, docx.text.paragraph.Paragraph)
              and block.text == "JavaScript authoring: Café 東京"]
    paragraph, table, image = blocks[start:start + 3]
    if standalone:
        assert len(blocks) == 3 and start == 0
    assert [(r.text, r.bold, r.italic) for r in paragraph.runs] == [
        ("JavaScript authoring: ", True, False), ("Café 東京", None, True),
    ]
    assert paragraph.alignment == docx.enum.text.WD_ALIGN_PARAGRAPH.CENTER
    assert isinstance(table, docx.table.Table)
    assert [len(row.cells) for row in table.rows] == [2, 2]
    assert [cell.text for cell in table.rows[0].cells] == ["Item", "Details"]
    assert table.cell(1, 0).text == "Pages"
    detail, nested, trailing = table.cell(1, 1).iter_inner_content()
    assert isinstance(detail, docx.text.paragraph.Paragraph)
    assert detail.text == "2\tReport details\nVerified"
    assert detail.runs[0].bold is True
    hyperlink, = detail.hyperlinks
    assert hyperlink.url == "https://example.org/report"
    assert hyperlink.runs[0].text == "Report details" and hyperlink.runs[0].italic is True
    assert isinstance(nested, docx.table.Table) and nested.cell(0, 0).text == "Nested detail"
    assert isinstance(trailing, docx.text.paragraph.Paragraph) and trailing.text == ""
    assert isinstance(image, docx.text.paragraph.Paragraph)
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        document = etree.fromstring(package.read("word/document.xml"))
        inline, = document.xpath("//wp:inline[wp:docPr/@descr=$description]", namespaces=NS,
                                 description="JavaScript authoring sample image")
        image_in_body, = document.xpath(
            "//w:p[w:r/w:t[text()='JavaScript authoring: ']]"
            "/following-sibling::*[2]/w:r/w:drawing/wp:inline", namespaces=NS)
        assert inline is image_in_body
        assert inline.xpath("wp:extent/@cx | wp:extent/@cy", namespaces=NS) == ["914400", "457200"]
        embed, = inline.xpath(".//a:blip/@odr:embed", namespaces=NS)
        rel = output.part.rels[embed]
        assert rel.reltype == f"{NS['odr']}/image" and not rel.is_external
        target = str(rel.target_part.partname).lstrip("/")
        assert target.startswith("word/media/")
        assert package.read(target) == (ROOT / "tests/fixtures/images/sample.png").read_bytes()


def verify_content(data: bytes, template: bytes) -> None:
    """Read actual generated output, not source text or self-reported coverage."""
    verify_javascript_workflow(data)
    output = docx.Document(io.BytesIO(data))
    with zipfile.ZipFile(io.BytesIO(data)) as package, zipfile.ZipFile(io.BytesIO(template)) as seed:
        document = etree.fromstring(package.read("word/document.xml"))
        styles = etree.fromstring(package.read("word/styles.xml"))
        text = "\n".join(document.xpath("//w:t/text()", namespaces=NS))
        for caption, expected in [("Builder bold unspecified; ", []),
                ("Builder bold enabled; ", ["1"]),
                ("Builder bold disabled, italic enabled.", ["0"])]:
            runs = document.xpath("//w:r[w:t[text()=$caption]]", caption=caption, namespaces=NS)
            assert len(runs) == 1
            assert runs[0].xpath("w:rPr/w:b/@w:val", namespaces=NS) == expected
        assert runs[0].xpath("w:rPr/w:i/@w:val", namespaces=NS) == ["1"]
        def values(root, path: str) -> set:
            return set(root.xpath(path, namespaces=NS))
        for topic in ["01 Paragraphs", "02 Run formatting", "03 Styles", "04 Tables", "05 Conditional", "06 Lists", "07 Inline images", "08 Authoring", "09 Section"]:
            assert topic in text, f"Missing catalog topic: {topic}"
        assert "{{replace}}" not in text and "Template REPLACED: cross-run replacement." == output.paragraphs[0].text
        assert "Templates with an OPC signature origin are rejected before any output is written." in text
        assert "Custom document properties: Project (Atlas) is retained; Review status changes to Draft & review." in text
        assert "Boolean custom properties: Review complete changes from false to true; Published is explicitly false." in text
        assert "Template compatibility: unsupported mandatory namespaces in edited WordprocessingML are rejected." in text
        assert "Extended document properties: Company is Atlas & Co.; Manager is Taylor." in text
        extended_type = f"{NS['odr']}/extended-properties"
        app_rel, = [rel for rel in output.part.package.rels.values() if rel.reltype == extended_type]
        assert app_rel.target_part.content_type == "application/vnd.openxmlformats-officedocument.extended-properties+xml"
        app = etree.fromstring(app_rel.target_part.blob)
        assert app.findtext("app:Company", namespaces=NS) == "Atlas & Co."
        assert app.findtext("app:Manager", namespaces=NS) == "Taylor"
        custom_type = f"{NS['odr']}/custom-properties"
        relation, = [rel for rel in output.part.package.rels.values() if rel.reltype == custom_type]
        part = relation.target_part
        assert part.content_type == "application/vnd.openxmlformats-officedocument.custom-properties+xml"
        def custom_properties(root):
            return [(n.get("name"), n.get("pid"), [(v.tag, v.text) for v in n])
                    for n in root.findall("cust:property", namespaces=NS)]
        string, boolean = (f"{{{NS['vt']}}}{tag}" for tag in ("lpwstr", "bool"))
        assert custom_properties(etree.fromstring(seed.read("docProps/custom.xml"))) == [
            ("Project", "2", [(string, "Atlas")]),
            ("Review complete", "3", [(boolean, "false")]),
            ("Review status", "4", [(string, "Pending")]),
        ]
        assert custom_properties(etree.fromstring(part.blob)) == [
            ("Project", "2", [(string, "Atlas")]),
            ("Review complete", "3", [(boolean, "true")]),
            ("Review status", "4", [(string, "Draft & review")]),
            ("Published", "5", [(boolean, "false")]),
        ]
        assert any(p.text == "Template retained paragraph: insertion appears before this." for p in output.paragraphs[2:])
        for level in range(1, 10):
            paragraph = next(p for p in output.paragraphs if p.text == f"This is a Heading {level}")
            assert paragraph.style.style_id == f"Heading{level}"
        assert values(document, "//w:pPr/w:jc/@w:val") >= {"left", "center", "right", "both", "distribute", "mediumKashida", "highKashida", "lowKashida", "thaiDistribute"}
        assert values(document, "//w:spacing/@w:lineRule") >= {"auto", "exact", "atLeast"}
        assert values(document, "//w:tabs/w:tab/@w:val") == {"left", "center", "right", "decimal", "bar", "clear", "start", "end"}
        assert values(document, "//w:tabs/w:tab/@w:leader") == {"none", "dot", "hyphen", "underscore", "heavy", "middleDot"}
        for prop in ["b", "i", "strike", "caps", "smallCaps", "bCs", "iCs", "rtl", "cs"]:
            assert values(document, f"//w:rPr/w:{prop}/@w:val") >= {"0", "1"}, prop
        assert values(document, "//w:rPr/w:u/@w:val") >= {"none", "single", "double", "dotted", "dash", "wave", "words"}
        assert len(values(document, "//w:color/@w:themeColor")) == 16
        assert values(document, "//w:vertAlign/@w:val") == {"superscript", "subscript"}
        assert document.xpath("//w:t[@xml:space='preserve']", namespaces=NS)
        assert document.xpath("//w:r[not(*)]", namespaces=NS), "Explicit empty run"
        assert values(document, "//w:tcPr/w:textDirection/@w:val") == {"lrTb", "tbRl", "btLr", "lrTbV", "tbRlV", "tbLrV"}
        assert values(document, "//w:trHeight/@w:hRule") == {"exact", "atLeast"}
        assert values(document, "//w:tblHeader/@w:val") >= {"0", "1"}
        assert values(document, "//w:cantSplit/@w:val") >= {"0", "1"}
        assert values(document, "//w:tblLayout/@w:type") >= {"fixed", "autofit"}
        assert document.xpath("//w:tcPr/w:gridSpan[@w:val='2']", namespaces=NS)
        assert document.xpath("//w:vMerge[@w:val='restart']", namespaces=NS)
        assert document.xpath("//w:vMerge[not(@w:val) or @w:val='continue']", namespaces=NS)
        assert document.xpath("//w:tc/w:tbl", namespaces=NS), "Nested tables"
        assert document.xpath("//w:tcMar/w:start[@w:w='0']", namespaces=NS)
        borders = document.xpath("//w:tcBorders", namespaces=NS)[0]
        assert [etree.QName(n).localname for n in borders] == ["top", "start", "bottom", "end"]
        for region, wire in REGIONS.items():
            style = styles.xpath("w:style[@w:styleId=$id]", namespaces=NS, id=f"Region{region}")[0]
            target = style if wire is None else style.xpath("w:tblStylePr[@w:type=$wire]", namespaces=NS, wire=wire)[0]
            if wire is not None:
                assert [etree.QName(n).localname for n in target] == ["pPr", "rPr", "tcPr"]
            for prop in ["pPr/w:spacing", "pPr/w:tabs", "pPr/w:bidi", "rPr/w:lang", "rPr/w:rFonts", "rPr/w:szCs", "rPr/w:rtl", "tcPr/w:shd"]:
                assert target.xpath(f"w:{prop}", namespaces=NS), (region, prop)
        activated = document.xpath("//w:tbl[w:tblPr/w:tblLook[@w:val='01E0']]", namespaces=NS)
        assert len(activated) == 13
        assert {n.xpath("string(w:tblPr/w:tblStyle/@w:val)", namespaces=NS) for n in activated} == {f"Region{region}" for region in REGIONS}
        for table in activated:
            look = table.find(f"{{{NS['w']}}}tblPr/{{{NS['w']}}}tblLook")
            assert [look.get(f"{{{NS['w']}}}{name}") for name in ["firstRow", "lastRow", "firstColumn", "lastColumn", "noHBand", "noVBand"]] == ["1", "1", "1", "1", "0", "0"]
        assert document.xpath("//w:tblLook[@w:val='0600']", namespaces=NS), "Explicit all-disabled activation sample"
        assert document.xpath("//w:tblLook[@w:val='04A0']", namespaces=NS), "Existing library defaults retained elsewhere"
        for name in ["Catalog Existing", "Catalog Existing Character"]:
            assert output.styles[name].font.bold is True and output.styles[name].font.italic is True
        assert output.styles["Catalog Child"].base_style.name == "Catalog Base"
        assert styles.xpath("w:style[@w:styleId='CatalogChild']/w:link[@w:val='CatalogCharacter']", namespaces=NS)
        assert styles.xpath("w:style[@w:styleId='CatalogCharacter']/w:link[@w:val='CatalogChild']", namespaces=NS)
        assert styles.xpath("w:docDefaults/w:rPrDefault/w:rPr/w:sz[@w:val='22']", namespaces=NS)
        assert styles.xpath("w:docDefaults/w:pPrDefault/w:pPr/w:spacing[@w:line='264']", namespaces=NS)
        nums = etree.fromstring(package.read("word/numbering.xml"))
        for alignment in ["left", "center", "right", "start", "end"]:
            tables = document.xpath("//w:tbl[.//w:t[text()=$caption]]", caption=f"Table alignment: {alignment}", namespaces=NS)
            assert len(tables) == 1
            assert tables[0].xpath("string(w:tblPr/w:jc/@w:val)", namespaces=NS) == alignment
            assert tables[0].xpath("string(w:tblGrid/w:gridCol/@w:w)", namespaces=NS) == "2160"
            assert not tables[0].xpath(".//w:pPr/w:jc", namespaces=NS)
        num = nums.xpath("w:num[@w:numId='60']", namespaces=NS)[0]
        abstract_id = num.xpath("string(w:abstractNumId/@w:val)", namespaces=NS)
        abstract = nums.xpath("w:abstractNum[@w:abstractNumId=$identity]", identity=abstract_id, namespaces=NS)[0]
        assert abstract.xpath("w:lvl/w:lvlJc/@w:val", namespaces=NS) == ["left", "center", "right", "start", "end"]
        assert num.xpath("w:lvlOverride/w:lvl/w:lvlJc/@w:val", namespaces=NS) == ["end", "start"]
        assert len(abstract.xpath("w:lvl[not(w:lvlJc)]", namespaces=NS)) == 4
        assert len(nums.xpath("w:abstractNum/w:lvl[@w:ilvl='2']/w:lvlRestart[@w:val='0']", namespaces=NS)) == 1
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:lvlRestart[@w:val='0']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='1']/w:lvl/w:lvlRestart[@w:val='1']", namespaces=NS)
        assert len(nums.xpath("//w:lvlRestart", namespaces=NS)) == 3
        assert len(nums.xpath("w:abstractNum/w:lvl[@w:ilvl='2']/w:suff[@w:val='tab']", namespaces=NS)) == 1
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:suff[@w:val='space']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='1']/w:lvl/w:suff[@w:val='nothing']", namespaces=NS)
        assert len(nums.xpath("//w:suff", namespaces=NS)) == 3
        assert len(nums.xpath("w:abstractNum/w:lvl[@w:ilvl='2']/w:isLgl[@w:val='1']", namespaces=NS)) == 1
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:isLgl[@w:val='0']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='1']/w:lvl/w:isLgl[@w:val='1']", namespaces=NS)
        assert len(nums.xpath("//w:isLgl", namespaces=NS)) == 3
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:startOverride[@w:val='7']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='8']/w:startOverride[@w:val='0']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:numFmt[@w:val='lowerLetter']", namespaces=NS)
        assert nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='1']/w:lvl/w:lvlText[@w:val='(%2)']", namespaces=NS)
        instance = nums.xpath("w:num[@w:numId='60']", namespaces=NS)[0]
        abstract_id = instance.find(f"{{{NS['w']}}}abstractNumId").get(f"{{{NS['w']}}}val")
        abstract = nums.xpath("w:abstractNum[@w:abstractNumId=$id]", namespaces=NS, id=abstract_id)[0]
        assert abstract.xpath("w:lvl[@w:ilvl='0']/w:numFmt[@w:val='decimal']", namespaces=NS)
        assert abstract.xpath("w:lvl[@w:ilvl='1']/w:numFmt[@w:val='lowerRoman']", namespaces=NS)
        assert nums.xpath("count(w:abstractNum[@w:abstractNumId='100'])", namespaces=NS) == 1
        for concrete_id in ("61", "62"):
            assert nums.xpath(
                "w:num[@w:numId=$id]/w:abstractNumId/@w:val", namespaces=NS, id=concrete_id
            ) == ["100"]
        assert nums.xpath("w:num[@w:numId='62']/w:lvlOverride/w:startOverride/@w:val", namespaces=NS) == ["4"]
        assert nums.xpath(
            "w:abstractNum[@w:abstractNumId='100']/w:styleLink/@w:val", namespaces=NS
        ) == ["CatalogBullets"]
        assert styles.xpath(
            "w:style[@w:styleId='CatalogBullets'][@w:type='numbering']/w:pPr/w:numPr/w:numId/@w:val",
            namespaces=NS,
        ) == ["61"]
        assert nums.xpath(
            "w:num[@w:numId='63']/w:abstractNumId/@w:val", namespaces=NS
        ) == ["101"]
        inherited = nums.xpath("w:abstractNum[@w:abstractNumId='101']", namespaces=NS)[0]
        assert inherited.xpath("w:numStyleLink/@w:val", namespaces=NS) == ["CatalogBullets"]
        assert not inherited.xpath("w:lvl", namespaces=NS)
        assert "Inherited numbering style: numId=63" in text
        assert "Numbering style: Catalog Bullets" in text
        assert "Shared abstract numbering: numId=62" in text
        assert abstract.xpath("w:lvl[@w:ilvl='0']/w:rPr/w:b/@w:val", namespaces=NS) == ["1"]
        assert not abstract.xpath("w:lvl[@w:ilvl='1']/w:rPr", namespaces=NS)
        replacement = nums.xpath("w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:rPr", namespaces=NS)[0]
        assert replacement.xpath("w:b/@w:val", namespaces=NS) == ["0"]
        assert replacement.xpath("w:i/@w:val", namespaces=NS) == ["1"]
        base_indent = abstract.xpath("w:lvl[@w:ilvl='0']/w:pPr/w:ind", namespaces=NS)[0]
        assert base_indent.xpath("@w:left", namespaces=NS) == ["480"]
        assert base_indent.xpath("@w:hanging", namespaces=NS) == ["240"]
        assert replacement.getparent().xpath("w:pPr/w:ind/@w:left", namespaces=NS) == ["720"]
        assert abstract.xpath("w:lvl[@w:ilvl='0']/w:pStyle/@w:val", namespaces=NS) == ["Heading1"]
        assert replacement.getparent().xpath("w:pStyle/@w:val", namespaces=NS) == ["Heading2"]
        assert "Numbering level style links: abstract level 0" in text
        assert "Numbering level paragraph properties: abstract level 0" in text
        assert "Numbering label formatting: abstract level 0 requests bold" in text
        assert "Concrete numbering-level replacements (numId=60)" in text
        assert values(document, "//w:numPr/w:ilvl/@w:val") >= set(map(str, range(9)))
        assert values(document, "//w:bookmarkStart/@w:name") == {"CatalogBookmark", "OverlappingBookmark"}
        assert document.xpath("//w:hyperlink[@w:anchor='CatalogBookmark']", namespaces=NS)
        assert len(document.xpath("//w:fldSimple", namespaces=NS)) >= 5
        assert len(document.xpath("//w:fldChar[@w:fldCharType='begin']", namespaces=NS)) >= 6
        assert "[TOC cached placeholder]" in text and "[nested PAGE cache]" in text
        control = document.xpath("//w:p/w:sdt[w:sdtPr/w:tag/@w:val='review-caption']", namespaces=NS)
        assert len(control) == 1
        assert control[0].xpath("w:sdtPr/w:id/@w:val", namespaces=NS) == ["5001"]
        assert control[0].xpath("w:sdtPr/w:alias/@w:val", namespaces=NS) == ["Review caption"]
        assert control[0].xpath("w:sdtPr/w:lock/@w:val", namespaces=NS) == ["contentLocked"]
        assert control[0].xpath("w:sdtPr/w:richText", namespaces=NS)
        assert control[0].xpath("w:sdtContent/w:r/w:t/text()", namespaces=NS) == ["controlled caption"]
        block_control = document.xpath("//w:body/w:sdt[w:sdtPr/w:tag/@w:val='review-block']", namespaces=NS)
        assert len(block_control) == 1
        assert block_control[0].xpath("w:sdtPr/w:id/@w:val", namespaces=NS) == ["5002"]
        assert block_control[0].xpath("w:sdtPr/w:lock/@w:val", namespaces=NS) == ["sdtLocked"]
        assert block_control[0].xpath("w:sdtContent/w:p/w:r/w:t/text()", namespaces=NS) == ["Controlled block paragraph"]
        assert block_control[0].xpath("w:sdtContent/w:tbl//w:t/text()", namespaces=NS) == ["Controlled table cell"]
        assert "Block content control: a controlled paragraph and table" in text
        plain = document.xpath("//w:p/w:sdt[w:sdtPr/w:tag/@w:val='plain-review']", namespaces=NS)[0]
        assert plain.xpath("w:sdtPr/w:text/@w:multiLine", namespaces=NS) == ["1"]
        assert plain.xpath("w:sdtContent/w:r/w:t/text()", namespaces=NS) == ["first line", "second line"]
        assert plain.xpath("count(w:sdtContent/w:r/w:br)", namespaces=NS) == 1
        assert "Plain-text control: one run, multiline enabled." in text
        dropdown = document.xpath("//w:p/w:sdt[w:sdtPr/w:tag/@w:val='review-status']", namespaces=NS)[0]
        options = dropdown.xpath("w:sdtPr/w:dropDownList", namespaces=NS)[0]
        assert options.xpath("@w:lastValue", namespaces=NS) == ["accepted"]
        assert options.xpath("w:listItem/@w:value", namespaces=NS) == ["pending", "accepted"]
        assert options.xpath("w:listItem/@w:displayText", namespaces=NS) == ["Pending", "Accepted"]
        assert dropdown.xpath("w:sdtContent/w:r/w:t/text()", namespaces=NS) == ["Accepted"]
        target = "//w:sdt[w:sdtPr/w:tag/@w:val='template-control']/w:sdtContent/w:r"
        old = etree.fromstring(seed.read("word/document.xml")).xpath(target, namespaces=NS)[0]
        edited = document.xpath(target, namespaces=NS)[0]
        assert old.xpath("w:t/text()", namespaces=NS) == ["old controlled value"]
        assert edited.xpath("w:t/text()", namespaces=NS) == ["updated controlled value"]
        assert edited.xpath("w:rPr/w:b/@w:val", namespaces=NS) == ["1"]
        assert "Template plain-text control replacement:" in text
        assert "Drop-down control: Review status" in text
        row_control = document.xpath("//w:tbl/w:sdt[w:sdtPr/w:tag/@w:val='review-row']", namespaces=NS)[0]
        assert row_control.xpath("w:sdtPr/w:id/@w:val", namespaces=NS) == ["5003"]
        cell_control = row_control.xpath("w:sdtContent/w:tr/w:sdt[w:sdtPr/w:tag/@w:val='review-cell']", namespaces=NS)[0]
        assert cell_control.xpath("w:sdtPr/w:id/@w:val", namespaces=NS) == ["5004"]
        assert cell_control.xpath("w:sdtContent/w:tc/w:p/w:r/w:t/text()", namespaces=NS) == ["Controlled row and cell"]
        assert "Table content controls: one row and its cell" in text
        assert "Inline content control: rich text" in text
        assert "Tracked text revisions: inserted text and deleted text" in text
        original = etree.fromstring(seed.read("word/document.xml"))
        assert original.xpath("//w:ins/w:r/w:t[text()='accepted template wording']", namespaces=NS)
        assert original.xpath("//w:del/w:r/w:delText[text()='removed template wording']", namespaces=NS)
        assert document.xpath("//w:p/w:r/w:t[text()='accepted template wording']", namespaces=NS)
        assert not document.xpath("//w:delText[text()='removed template wording']", namespaces=NS)
        assert original.xpath("//w:moveFromRangeStart[@w:name='TemplateMove']", namespaces=NS)
        assert original.xpath("//w:moveToRangeStart[@w:name='TemplateMove']", namespaces=NS)
        assert not document.xpath("//*[@w:name='TemplateMove']", namespaces=NS)
        assert document.xpath("//w:p/w:r/w:t[text()='moved template wording']", namespaces=NS) == [
            document.xpath("//w:p[w:r/w:t='Template accepted move: ']/w:r[last()]/w:t", namespaces=NS)[0],
        ]
        assert "Template revision action:" in text
        inserted = document.xpath("//w:ins[w:r/w:t='inserted wording']", namespaces=NS)
        deleted = document.xpath("//w:del[w:r/w:delText='old wording']", namespaces=NS)
        assert len(inserted) == len(deleted) == 1
        assert inserted[0].xpath("@w:author | @w:date", namespaces=NS) == [
            "Catalog editor", "2026-11-20T00:00:00Z",
        ]
        assert deleted[0].xpath("@w:author", namespaces=NS) == ["Catalog editor"]
        assert inserted[0].xpath("@w:id", namespaces=NS) != deleted[0].xpath("@w:id", namespaces=NS)
        for role in ("moveFrom", "moveTo"):
            start = document.xpath(f"//w:{role}RangeStart[@w:name='CatalogMove']", namespaces=NS)[0]
            wrapper = start.getnext()
            end = wrapper.getnext()
            assert etree.QName(wrapper).localname == role
            assert etree.QName(end).localname == f"{role}RangeEnd"
            assert start.xpath("@w:id", namespaces=NS) == end.xpath("@w:id", namespaces=NS)
            assert start.xpath("@w:id", namespaces=NS) != wrapper.xpath("@w:id", namespaces=NS)
            assert start.xpath("@w:date", namespaces=NS) == ["2026-11-20T00:00:00Z"]
            assert wrapper.xpath("w:r/w:t/text()", namespaces=NS) == ["relocated wording"]
        assert "Tracked text move: CatalogMove" in text
        settings = etree.fromstring(package.read("word/settings.xml"))
        for name, start, restart, number_format in [
            ("footnote", "3", "eachPage", "lowerRoman"), ("endnote", "7", "eachSect", "upperLetter"),
        ]:
            owner = settings.find(f"w:{name}Pr", namespaces=NS)
            assert owner.xpath("w:numFmt/@w:val", namespaces=NS) == [number_format]
            assert owner.xpath("w:numStart/@w:val", namespaces=NS) == [start]
            assert owner.xpath("w:numRestart/@w:val", namespaces=NS) == [restart]
        assert settings.xpath("w:trackRevisions/@w:val", namespaces=NS) == ["0"]
        assert settings.xpath("w:mirrorMargins/@w:val", namespaces=NS) == ["1"]
        assert settings.xpath("w:gutterAtTop/@w:val", namespaces=NS) == ["0"]
        assert "Document page settings: mirrored margins enabled" in text
        assert settings.xpath("w:footnotePr/w:pos/@w:val", namespaces=NS) == ["beneathText"]
        assert settings.xpath("w:endnotePr/w:pos/@w:val", namespaces=NS) == ["sectEnd"]
        assert "Note numbering: lower-Roman footnotes start at 3" in text
        for name, tag, label in [("footnotes", "footnote", "Footnote body"), ("endnotes", "endnote", "Endnote body"), ("comments", "comment", "Structured comment")]:
            root = etree.fromstring(package.read(f"word/{name}.xml"))
            assert label in "".join(root.xpath("//w:t/text()", namespaces=NS))
            assert root.xpath(f"//w:{tag}", namespaces=NS)
            if tag == "comment":
                assert root.xpath("w:comment[@w:author='Structured reviewer']/@w:initials", namespaces=NS) == ["SR"]
            else:
                assert root.xpath(f"count(w:{tag}[@w:id='0'])", namespaces=NS) == 1
        assert values(document, "//w:sectPr/w:type/@w:val") >= {"nextPage", "nextColumn", "continuous", "evenPage", "oddPage"}
        for prop in ["bidi", "rtlGutter"]:
            assert values(document, f"//w:sectPr/w:{prop}/@w:val") == {"0", "1"}
        lines = document.xpath("//w:sectPr/w:lnNumType", namespaces=NS)
        assert [line.xpath("@w:restart", namespaces=NS) for line in lines] == [
            ["newPage"], ["newSection"], ["continuous"], ["continuous"],
        ]
        for line in lines:
            assert line.xpath("@w:countBy | @w:start | @w:distance", namespaces=NS) == ["5", "0", "240"]
        for label, expected in [("Paragraph line numbers: suppressed.", "1"),
                                ("Paragraph line numbers: suppression explicitly off.", "0")]:
            nodes = document.xpath(
                "//w:p[.//w:t[text()=$label]]/w:pPr/w:suppressLineNumbers/@w:val",
                namespaces=NS, label=label,
            )
            assert nodes == [expected] * 5
        grids = document.xpath("(//w:sectPr)[position() > 1]/w:docGrid", namespaces=NS)
        assert [grid.xpath("@w:type", namespaces=NS) for grid in grids] == [
            ["default"], ["lines"], ["linesAndChars"], ["snapToChars"], ["snapToChars"],
        ]
        for grid in grids:
            assert grid.xpath("@w:linePitch", namespaces=NS) == ["360"]
            assert grid.xpath("@w:charSpace", namespaces=NS) == ["-4096"]
        layouts = document.xpath(
            "//w:p[.//w:t[text()='Paragraph layout: contextual spacing enabled, mirrored indents explicitly off.']]",
            namespaces=NS,
        )
        assert len(layouts) == 5
        for paragraph in layouts:
            assert paragraph.xpath("w:pPr/w:contextualSpacing/@w:val", namespaces=NS) == ["1"]
            assert paragraph.xpath("w:pPr/w:mirrorIndents/@w:val", namespaces=NS) == ["0"]
        snapped = document.xpath(
            "//w:p[.//w:t[text()='Grid snapping: paragraph enabled, run explicitly off.']]",
            namespaces=NS,
        )
        assert len(snapped) == 5
        for paragraph in snapped:
            assert paragraph.xpath("w:pPr/w:snapToGrid/@w:val", namespaces=NS) == ["1"]
            assert paragraph.xpath("w:r/w:rPr/w:snapToGrid/@w:val", namespaces=NS) == ["0"]
        assert "Document grid: mode=snapToChars" in text
        assert not document.xpath("(//w:sectPr)[last()]/w:lnNumType", namespaces=NS)
        assert "Line numbering: clear previous section settings." in text
        assert "Line numbering: every 5 lines" in text
        assert values(document, "//w:sectPr/w:vAlign/@w:val") == {"top", "center", "bottom", "both"}
        assert values(document, "//w:sectPr/w:pgSz/@w:orient") >= {"portrait", "landscape"}
        assert values(document, "//w:sectPr/w:pgMar/@w:header") == {"720"}
        assert values(document, "//w:sectPr/w:pgMar/@w:footer") == {"360"}
        page_borders = document.xpath("(//w:sectPr)[1]/w:pgBorders", namespaces=NS)[0]
        assert [page_borders.get(f"{{{NS['w']}}}{key}") for key in ("offsetFrom", "display", "zOrder")] == [
            "page", "firstPage", "back",
        ]
        assert [etree.QName(border).localname for border in page_borders] == ["top", "left", "bottom", "right"]
        for side, expected in [("top", ["single", "12", "6", "203060"]),
                               ("left", ["dashed", "8", "4", "606060"]),
                               ("bottom", ["single", "12", "6", "203060"]),
                               ("right", ["dashed", "8", "4", "606060"])]:
            border = page_borders.find(f"w:{side}", namespaces=NS)
            assert [border.get(f"{{{NS['w']}}}{key}") for key in ("val", "sz", "space", "color")] == expected
        assert "Page borders: navy single top/bottom" in text
        assert "Header/footer distances: 720 and 360 twips" in text
        assert len(document.xpath("//w:headerReference", namespaces=NS)) >= 3
        assert len(document.xpath("//w:footerReference", namespaces=NS)) >= 3
        assert {p.rsplit(".", 1)[-1] for p in package.namelist() if p.startswith("word/media/")} >= {"png", "jpg", "gif", "bmp", "tiff"}
        anchored, = document.xpath("//wp:anchor[wp:wrapSquare and .//a:blip]", namespaces=NS)
        assert anchored.xpath("wp:positionH[@relativeFrom='column']/wp:posOffset/text()", namespaces=NS) == ["3175000"]
        assert anchored.xpath("wp:positionV[@relativeFrom='page']/wp:align/text()", namespaces=NS) == ["bottom"]
        square, = anchored.xpath("wp:wrapSquare", namespaces=NS)
        assert square.get("wrapText") == "largest"
        assert [square.get(key) for key in ("distT", "distB", "distL", "distR")] == ["0", "25400", "38100", "50800"]
        assert anchored.get("relativeHeight") == "4"
        assert [anchored.get(key) for key in ("locked", "layoutInCell", "allowOverlap")] == ["1", "0", "0"]
        assert "Floating picture: column-relative horizontal offset" in text
        polygon, = document.xpath("//wp:wrapTight/wp:wrapPolygon", namespaces=NS)
        assert [etree.QName(point).localname for point in polygon] == ["start", "lineTo", "lineTo"]
        assert [(point.get("x"), point.get("y")) for point in polygon] == [
            ("0", "0"), ("21600", "21600"), ("0", "21600"),
        ]
        assert "tight text wrapping around an authored triangular contour" in text
        canvases = document.xpath("//wp:inline//a:graphicData/*[local-name()='lockedCanvas']", namespaces=NS)
        assert [canvas.xpath("a:sp/a:spPr/a:prstGeom/@prst", namespaces=NS) for canvas in canvases] == [
            ["rect"], ["ellipse"], [],
        ]
        assert [canvas.xpath("a:sp/a:spPr/a:solidFill/a:srgbClr/@val", namespaces=NS) for canvas in canvases] == [
            ["2050B0"], ["209050"], [],
        ]
        assert canvases[1].xpath("a:sp/a:spPr/a:ln/@w", namespaces=NS) == ["19050"]
        assert canvases[1].xpath("a:sp/a:spPr/a:ln/a:solidFill/a:srgbClr/@val", namespaces=NS) == ["103020"]
        body, = canvases[0].xpath("a:sp/a:txSp/a:txBody", namespaces=NS)
        assert body.xpath("a:p/a:r/a:t/text()", namespaces=NS) == ["Blue", "Text"]
        assert body.xpath("a:bodyPr/@anchor", namespaces=NS) == ["b"]
        assert body.xpath("a:p/a:pPr/@algn", namespaces=NS) == ["ctr"] * 3
        assert body.xpath("a:p/a:r/a:rPr/@sz", namespaces=NS) == ["1425"] * 2
        assert body.xpath("a:p/a:r/a:rPr/@b", namespaces=NS) == ["0"] * 2
        assert body.xpath("a:p/a:r/a:rPr/@i", namespaces=NS) == ["1"] * 2
        assert body.xpath("a:p/a:r/a:rPr/a:latin/@typeface", namespaces=NS) == ["Arial"] * 2
        assert body.xpath("a:p/a:r/a:rPr/a:solidFill/a:srgbClr/@val", namespaces=NS) == ["FFFFFF"] * 2
        assert body.xpath("a:p/a:endParaRPr/@sz", namespaces=NS) == ["1425"] * 3
        assert body.xpath("a:p/a:endParaRPr/@i", namespaces=NS) == ["1"] * 3
        assert body.xpath("a:p/a:endParaRPr/a:latin/@typeface", namespaces=NS) == ["Arial"] * 3
        assert not canvases[1].xpath(".//a:txBody", namespaces=NS)
        box, = canvases[2].xpath("a:txSp", namespaces=NS)
        assert not canvases[2].xpath("a:sp", namespaces=NS)
        assert [etree.QName(child).localname for child in box] == ["txBody", "xfrm"]
        assert box.xpath("a:txBody/a:p/a:r/a:t/text()", namespaces=NS) == ["Standalone", "Text box"]
        assert box.xpath("a:xfrm/a:ext/@cx | a:xfrm/a:ext/@cy", namespaces=NS) == ["1828800", "609600"]
        assert box.xpath("a:txBody/a:p/a:r/a:rPr/a:latin/@typeface", namespaces=NS) == ["Arial"] * 2
        assert box.xpath("a:txBody/a:p/a:r/a:rPr/@b", namespaces=NS) == ["0"] * 2
        assert "Standalone inline text box: 144 by 48 points" in text
        floating, = document.xpath("//wp:anchor[.//a:graphicData/*[local-name()='lockedCanvas']]", namespaces=NS)
        assert floating.xpath("wp:positionH[@relativeFrom='margin']/wp:posOffset/text()", namespaces=NS) == ["2794000"]
        assert floating.xpath("wp:positionV[@relativeFrom='paragraph']/wp:posOffset/text()", namespaces=NS) == ["152400"]
        assert floating.xpath("wp:wrapSquare/@wrapText | wp:wrapSquare/@distL", namespaces=NS) == ["bothSides", "76200"]
        assert floating.xpath("wp:extent/@cx | wp:extent/@cy", namespaces=NS) == ["1371600", "304800"]
        assert floating.xpath(".//a:graphicData/*/a:txSp/a:txBody/a:p/a:r/a:t/text()", namespaces=NS) == ["Floating note"]
        assert "Floating text box: 220 pt from the margin" in text
        assert "Shape text style: blue rectangle, 14.25 pt white Arial text" in text
        assert "Inline DrawingML shapes: a blue rectangle with text and a green ellipse with a dark outline." in text
        caption = "Inline column chart: North 8, South 13.5, West 5; one series without an embedded workbook."
        chart_inline, = document.xpath(
            "//w:p[preceding-sibling::*[1][w:r/w:t=$caption]]//wp:inline[.//c:chart]",
            caption=caption, namespaces=NS,
        )
        assert chart_inline.xpath("wp:extent/@cx | wp:extent/@cy", namespaces=NS) == ["3657600", "2286000"]
        chart_id, = chart_inline.xpath(".//c:chart/@odr:id", namespaces=NS)
        relation = output.part.rels[chart_id]
        assert not relation.is_external and relation.reltype == f"{NS['odr']}/chart"
        part = relation.target_part
        assert part.content_type == "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
        assert not part.rels
        chart = etree.fromstring(part.blob)
        assert chart.tag == f"{{{NS['c']}}}chartSpace"
        assert not chart.xpath("c:externalData", namespaces=NS)
        plot, = chart.xpath("c:chart/c:plotArea", namespaces=NS)
        assert plot.xpath("c:barChart/c:barDir/@val", namespaces=NS) == ["col"]
        series, = plot.xpath("c:barChart/c:ser", namespaces=NS)
        assert series.xpath("c:cat/c:strLit/c:pt/c:v/text()", namespaces=NS) == ["North", "South", "West"]
        assert series.xpath("c:val/c:numLit/c:pt/c:v/text()", namespaces=NS) == ["8", "13.5", "5"]
        for cache in series.xpath("c:cat/c:strLit | c:val/c:numLit", namespaces=NS):
            assert cache.xpath("c:ptCount/@val | c:pt/@idx", namespaces=NS) == ["3", "0", "1", "2"]
        axis_ids = plot.xpath("c:barChart/c:axId/@val", namespaces=NS)
        assert len(set(axis_ids)) == 2
        assert axis_ids == plot.xpath("c:catAx/c:axId/@val | c:valAx/c:axId/@val", namespaces=NS)
        assert axis_ids[::-1] == plot.xpath("c:catAx/c:crossAx/@val | c:valAx/c:crossAx/@val", namespaces=NS)
        equation, = document.xpath("//m:oMath", namespaces=NS)
        assert equation.getparent().tag == f"{{{NS['w']}}}p"
        assert equation.getprevious().findtext("w:t", namespaces=NS) == "Inline equation: stacked fraction "
        assert equation.getnext().findtext("w:t", namespaces=NS) == "."
        fraction, = equation
        assert fraction.tag == f"{{{NS['m']}}}f"
        assert [child.tag for child in fraction] == [f"{{{NS['m']}}}num", f"{{{NS['m']}}}den"]
        assert fraction.xpath("m:num/m:r/m:t/text() | m:den/m:r/m:t/text()", namespaces=NS) == ["1", "2"]
        cropped = document.xpath("//wp:inline[.//a:srcRect]", namespaces=NS)
        assert len(cropped) == 1
        assert cropped[0].xpath("wp:extent/@cx | wp:extent/@cy", namespaces=NS) == ["762000", "508000"]
        rect = cropped[0].xpath(".//a:srcRect", namespaces=NS)[0]
        assert [rect.get(key) for key in ("l", "t", "r", "b")] == ["10000", "5000", "20000", "15000"]
        transform, = cropped[0].xpath(".//a:xfrm", namespaces=NS)
        assert [transform.get(key) for key in ("rot", "flipH", "flipV")] == ["1800000", "1", "0"]
        assert "Image crop and rotation: 10% left" in text
        assert document.xpath("//wp:docPr[@descr='Accessible png sample']", namespaces=NS)
        assert text.count("Shared reference: computed once and used twice.") == 2
        assert text.count("Resolved-child reuse: immutable paragraph used twice.") == 2
        assert text.count("Resolved-inline reuse: immutable run used twice.") == 2
        assert "excluded producer must not execute" not in text
        assert output.core_properties.title == "All features visual inspection"
        assert package.read("word/theme/theme1.xml") == seed.read("word/theme/theme1.xml"), "Untouched opaque theme payload retained"


def generate(directory: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    """Run both independent API programs, including each program's repeat check."""
    directory.mkdir(parents=True, exist_ok=True)
    rust, python = (directory / f"all_features-{name}.docx" for name in ["rust", "python"])
    subprocess.run(["cargo", "run", "--quiet", "--locked", "--example", "all_features", "--", str(rust.resolve())], cwd=ROOT, check=True)
    subprocess.run([sys.executable, str(ROOT / "examples/all_features.py"), str(python.resolve())], cwd=ROOT, check=True)
    return rust, python


def verify_first_header_scopes(original: bytes, first_shared: bytes, first_local: bytes) -> None:
    header = docx.Document(io.BytesIO(original)).sections[0].first_page_header.part
    part_name = str(header.partname).lstrip("/")
    rels_name = str(header.partname.rels_uri).lstrip("/")
    for data, expected, local in ((first_shared, [1, 1], False), (first_local, [0, 1], True)):
        document = docx.Document(io.BytesIO(data))
        headers = [section.first_page_header for section in document.sections]
        assert [h.is_linked_to_previous for h in headers] == [False, not local]
        assert [len(h._element.xpath(".//w:drawing")) for h in headers] == expected
        image, = headers[1]._element.xpath(".//a:blip/@r:embed")
        assert headers[1].part.related_parts[image].blob == (ROOT / "tests/fixtures/images/sample.png").read_bytes()
        with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(io.BytesIO(data)) as changed:
            mutable = {"[Content_Types].xml"}
            mutable.update({"word/document.xml", "word/_rels/document.xml.rels"} if local
                           else {part_name, rels_name})
            assert set(source.namelist()) <= set(changed.namelist())
            for name in set(source.namelist()) - mutable:
                assert source.read(name) == changed.read(name), name


def verify_entry_text(original: bytes, changed: bytes) -> None:
    owners = {f"word/{kind}s.xml" for kind in ("comment", "footnote", "endnote")}
    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(io.BytesIO(changed)) as output:
        relations = {f"word/_rels/{kind}s.xml.rels" for kind in ("comment", "footnote", "endnote")}
        assert set(output.namelist()) == set(source.namelist()) | relations
        assert {p for p in source.namelist() if source.read(p) != output.read(p)} == owners | (relations & set(source.namelist()))
        for kind in ("comment", "footnote", "endnote"):
            owner = f"word/{kind}s.xml"
            before, after = [etree.fromstring(z.read(owner)) for z in (source, output)]
            text, = after.xpath(".//w:t[text()=$label]", namespaces=NS, label=f"Edited {kind} entry")
            idx = after.xpath(".//w:t", namespaces=NS).index(text)
            before.xpath(".//w:t", namespaces=NS)[idx].text = text.text
            link = text.getparent().getnext()
            assert link.tag == f"{{{NS['w']}}}hyperlink"
            assert link.xpath("./w:r/w:t/text()", namespaces=NS) == ["Review link"]
            assert link.xpath("./w:r/w:rPr/w:sz/@w:val", namespaces=NS) == ["27"]
            rid = link.get(f"{{{NS['odr']}}}id")
            rel_path = f"word/_rels/{kind}s.xml.rels"
            rels = etree.fromstring(output.read(rel_path))
            added, = [r for r in rels if r.get("Id") == rid]
            assert added.get("Target") == "https://example.com/entry-review"
            assert added.get("TargetMode") == "External" and added.get("Type").endswith("/hyperlink")
            rels.remove(added)
            old_rels = etree.fromstring(source.read(rel_path)) if rel_path in source.namelist() else []
            assert [dict(r.attrib) for r in rels] == [dict(r.attrib) for r in old_rels]
            link.getparent().remove(link)
            assert etree.tostring(before, method="c14n", exclusive=True) == etree.tostring(after, method="c14n", exclusive=True)


def verify_even_footer(data: bytes) -> None:
    footers = [s.even_page_footer for s in docx.Document(io.BytesIO(data)).sections]
    assert all("Edited shared even footer" in f.paragraphs[1].text for f in footers)
    assert [len(f._element.xpath(".//w:drawing")) for f in footers] == [0, 1]
    rid, = footers[1]._element.xpath(".//a:blip/@r:embed")
    assert footers[1].part.related_parts[rid].blob == (ROOT / "tests/fixtures/images/sample.png").read_bytes()


def verify_run_text(original: bytes, replaced: bytes) -> None:
    edited_doc = docx.Document(io.BytesIO(replaced))
    assert edited_doc.element.xpath("//w:sdtPr[w:id/@w:val='5001']/w:alias/@w:val") == ["Imported editor caption"]
    assert edited_doc.core_properties.title == "Edited resource sample"
    assert edited_doc.core_properties.subject == "Imported resource edits"
    assert edited_doc.styles["Reviewed List"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["9049"]
    assert edited_doc.part.numbering_part.element.xpath("w:abstractNum[@w:abstractNumId='9048']/w:styleLink/@w:val") == ["ReviewedList"]
    assert edited_doc.core_properties.author == "Review team"
    paragraphs = [p.text for p in edited_doc.paragraphs]
    assert "Ranged 📝 + point" + EDIT_LABEL[8:] in paragraphs
    anchor = paragraphs.index("Imported editor: replaced this run by its anchor. " + "".join(INLINE_LINK_RUNS))
    assert paragraphs[anchor + 1] == "Imported editor: inserted paragraph."
    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(io.BytesIO(replaced)) as changed:
        assert set(source.namelist()) == set(changed.namelist())
        mutable = {"word/document.xml", "docProps/core.xml", "word/styles.xml", "word/numbering.xml", "word/settings.xml"}
        mutable.update({"docProps/app.xml", "docProps/custom.xml", "word/comments.xml"})
        app = etree.fromstring(changed.read("docProps/app.xml"))
        custom = etree.fromstring(changed.read("docProps/custom.xml"))
        assert app.findtext("{*}Manager") == "Review lead"
        assert custom.findtext("{*}property[@name='Reviewed']/{*}bool") == "false"
        assert {n for n in source.namelist() if source.read(n) != changed.read(n)} == mutable
        styles = etree.fromstring(changed.read("word/styles.xml"))
        assert styles.xpath("w:docDefaults/w:rPrDefault/w:rPr/w:b/@w:val", namespaces=NS) == ["0"]
        assert not styles.xpath("w:docDefaults/w:pPrDefault/w:pPr/w:spacing/@w:after", namespaces=NS)
        settings = etree.fromstring(changed.read("word/settings.xml"))
        assert settings.xpath("w:footnotePr/w:numStart/@w:val", namespaces=NS) == ["5"]
        assert styles.xpath("w:docDefaults/w:pPrDefault/w:pPr/w:jc/@w:val", namespaces=NS) == ["center"]
        numbers = etree.fromstring(changed.read("word/numbering.xml"))
        comments = etree.fromstring(changed.read("word/comments.xml"))
        assert comments.xpath("w:comment[@w:author='Structured reviewer']/w:p/w:pPr/w:numPr/w:numId/@w:val", namespaces=NS) == ["9049"]
        assert numbers.xpath("w:num[@w:numId='64']/w:abstractNumId/@w:val", namespaces=NS) == ["100"]
        assert numbers.xpath("w:num[@w:numId='65']/w:lvlOverride[@w:ilvl='0']/w:startOverride/@w:val", namespaces=NS) == ["12"]
        for aid, fmt in [("102", "upperLetter"), ("103", "upperRoman")]:
            assert numbers.xpath("w:abstractNum[@w:abstractNumId=$aid]/w:lvl/w:numFmt/@w:val", namespaces=NS, aid=aid) == [fmt]
        assert numbers.xpath("w:abstractNum[@w:abstractNumId='100']/w:styleLink/@w:val", namespaces=NS) == ["CatalogBullets"]
        assert numbers.xpath("w:abstractNum[@w:abstractNumId='101']/w:numStyleLink/@w:val", namespaces=NS) == ["CatalogBullets"]
        assert numbers.xpath("w:abstractNum[@w:abstractNumId='100']/w:lvl[@w:ilvl='1']/w:pStyle/@w:val", namespaces=NS) == ["Heading2"]
        assert edited_doc.core_properties.comments == "Reviewed numbering"
        assert numbers.xpath("w:abstractNum[@w:abstractNumId='100']/w:lvl[@w:ilvl='0']/w:start/@w:val", namespaces=NS) == ["7"]
        assert numbers.xpath("w:abstractNum[@w:abstractNumId='9048']/w:lvl[@w:ilvl='0']/w:start/@w:val", namespaces=NS) == ["2"]
        instance, = numbers.xpath("w:num[@w:numId='9049']", namespaces=NS)
        assert instance.xpath("w:abstractNumId/@w:val", namespaces=NS) == ["9048"]
        assert instance.xpath("w:lvlOverride[@w:ilvl='0']/w:startOverride/@w:val", namespaces=NS) == ["6"]
        assert instance.xpath("w:lvlOverride[@w:ilvl='0']/w:lvl/w:numFmt/@w:val", namespaces=NS) == ["lowerRoman"]
        numbered, = [p for p in edited_doc.paragraphs if p.text.startswith("Ranged 📝")]
        assert numbered._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["9049"]


def verify_javascript_catalog(rust: pathlib.Path, directory: pathlib.Path) -> None:
    """Require every file and check content and untouched payloads separately."""
    artifacts = {}
    for suffix in CATALOG_SUFFIXES:
        path = directory / f"all_features-javascript{suffix}"
        data = path.read_bytes()
        try:
            digest = exact_bytes(rust.with_suffix(suffix).read_bytes(), data, language="JavaScript")
        except AssertionError as error:
            raise AssertionError(f"{path}: {error}") from error
        artifacts[suffix] = data
        print(f"Byte-identical {path}: {len(data)} bytes; SHA-256 {digest}")
    verify_catalog_readback(artifacts)


def verify_catalog_readback(files: dict[str, bytes]) -> None:
    """Inspect source/result payloads within a single binding's catalog."""
    final, source = files[".docx"], files[".header-source.docx"]
    verify_objects(final, files[".objects.docx"])
    verify_entry_text(final, files[".entry-text.docx"])
    control_doc = docx.Document(io.BytesIO(files[".controls.docx"]))
    assert control_doc.element.xpath("//w:sdt[w:sdtPr/w:tag/@w:val='review-status']/w:sdtContent/w:r/w:t/text()") == ["Pending"]
    assert control_doc.element.xpath("//w:sdtPr[w:alias/@w:val='Imported plain-text sample']/w:text/@w:multiLine") == ["0"]
    changed = files[".entry-edits.docx"]
    comment, = [c for c in docx.Document(io.BytesIO(changed)).comments if c.author == "Structured reviewer"]
    assert comment.initials == "ED"
    with zipfile.ZipFile(io.BytesIO(final)) as before, zipfile.ZipFile(io.BytesIO(changed)) as after:
        assert set(before.namelist()) == set(after.namelist())
        assert {p for p in before.namelist() if before.read(p) != after.read(p)} == {"word/footnotes.xml", "word/comments.xml", "word/document.xml"}
        notes = etree.fromstring(after.read("word/footnotes.xml"))
        note, = notes.xpath("./w:footnote[@w:id='47']", namespaces=NS)
        assert note.xpath("./w:p/w:r/w:t/text()", namespaces=NS) == ["Lifecycle note", "Second note paragraph"]
        assert note.xpath("./w:p[1]/w:r[1]/w:footnoteRef", namespaces=NS)
        main = etree.fromstring(after.read("word/document.xml"))
        ref, = main.xpath(".//w:footnoteReference[@w:id='47']", namespaces=NS)
        assert ref.getparent().getprevious().xpath("w:t/text()", namespaces=NS) == ["Footnote reference"]
    verify_even_footer(files[".footer-even.docx"])
    story_bytes = files[".story-blocks.docx"]
    doc = docx.Document(io.BytesIO(story_bytes))
    original = docx.Document(io.BytesIO(source))
    one, two = doc.sections
    assert len(one.header.tables) == 1 and not two.header.tables
    assert two.header.part.partname == original.sections[1].header.part.partname
    for part in [one.header, two.first_page_footer]:
        link, = part.paragraphs[-1].hyperlinks
        assert link.address == "https://example.com/story-block" and link.text == "Story block link"
    assert one.header.paragraphs[1].hyperlinks[0].address == original.sections[0].header.paragraphs[1].hyperlinks[0].address
    with zipfile.ZipFile(io.BytesIO(source)) as before, zipfile.ZipFile(io.BytesIO(story_bytes)) as after:
        assert set(before.namelist()) <= set(after.namelist())
        assert {p for p in before.namelist() if before.read(p) != after.read(p)} == {
            "[Content_Types].xml", "word/_rels/document.xml.rels", "word/document.xml"}
    verify_content(final, files[".template.docx"])
    verify_run_text(final, files[".run-text.docx"])
    verify_edited(final, files[".edited.docx"])
    verify_header_scopes(source, files[".header-shared.docx"], files[".header-local.docx"])
    verify_first_header_scopes(source, files[".header-first-shared.docx"], files[".header-first-local.docx"])


def verify_objects(original: bytes, output: bytes) -> None:
    document = docx.Document(io.BytesIO(output))
    assert any("Inline equation:" in p.text for p in document.paragraphs)
    with zipfile.ZipFile(io.BytesIO(original)) as src, zipfile.ZipFile(io.BytesIO(output)) as dst:
        additions = set(dst.namelist()) - set(src.namelist())
        note = next(part for part in document.part.related_parts.values() if str(part.partname).endswith("/footnotes.xml"))
        note_part = str(note.partname).lstrip("/")
        note_rels = str(note.partname.rels_uri).lstrip("/")
        assert len(additions - {note_rels}) == 2
        note_xml = etree.fromstring(note.blob)
        original_note = etree.fromstring(src.read(note_part))
        drawing_ids = "//w:footnote[@w:id='0']//wp:docPr/@id"
        embeds = "//w:footnote[@w:id='0']//a:blip/@odr:embed"
        previous = original_note.xpath(drawing_ids, namespaces=NS)
        edited = note_xml.xpath(drawing_ids, namespaces=NS)
        added, = set(edited) - set(previous)
        assert [item for item in edited if item != added] == previous
        new_embed, *preserved_embeds = note_xml.xpath(embeds, namespaces=NS)
        assert edited[0] == added
        assert preserved_embeds == original_note.xpath(embeds, namespaces=NS)
        assert new_embed in preserved_embeds
        assert src.read(note_rels) == dst.read(note_rels)
        embed = new_embed
        asset = note.related_parts[embed]
        assert asset.blob == (ROOT / "tests/fixtures/images/sample.png").read_bytes()
        assert str(asset.partname).lstrip("/") in src.namelist()
        new_chart, = [part for part in additions if "rocx-chart" in part]
        chart = etree.fromstring(dst.read(new_chart))
        assert chart.xpath("//c:ser/c:cat/c:strLit/c:pt/c:v/text()", namespaces=NS) == ["Final", "Revised"]
        assert chart.xpath("//c:ser/c:val/c:numLit/c:pt/c:v/text()", namespaces=NS) == ["12","8"]
        for name in src.namelist():
            if name not in {"word/document.xml", "word/_rels/document.xml.rels", "[Content_Types].xml",note_part,note_rels}:
                assert src.read(name) == dst.read(name), name
        root = etree.fromstring(dst.read("word/document.xml"))
        assert root.xpath("//wp:docPr[@descr='Edited picture']", namespaces=NS)
        assert root.xpath("//a:sp[a:spPr/a:prstGeom[@prst='ellipse']]//a:t[text()='Edited shape']", namespaces=NS)
        assert root.xpath("//w:sdt[w:sdtPr/w:tag[@w:val='object-sibling']]//w:t/text()", namespaces=NS) == [EDIT_LABEL]
        assert root.xpath("//m:f[m:num/m:r/m:t='3' and m:den/m:r/m:t='7']", namespaces=NS)
        ids = root.xpath("//wp:docPr/@id", namespaces=NS) + note_xml.xpath("//wp:docPr/@id", namespaces=NS)
        assert len(ids) == len(set(ids))
        rid, = root.xpath("//c:chart/@odr:id", namespaces=NS)
        assert document.part.related_parts[rid].blob == dst.read(new_chart)


def main() -> None:
    """Generate visible artifacts and report the exact-byte SHA-256."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=pathlib.Path, default=ROOT / "examples")
    parser.add_argument("--javascript-dir", type=pathlib.Path, action="append", default=[],
                        help="compare every file from catalog-node.mjs (repeat for multiple runtimes)")
    args = parser.parse_args()
    verify_inventory()
    rust, python = generate(args.output_dir)
    native = {}
    for suffix in CATALOG_SUFFIXES:
        data = rust.with_suffix(suffix).read_bytes()
        digest = exact_bytes(data, python.with_suffix(suffix).read_bytes())
        print(f"Byte-identical {suffix}: {len(data)} bytes; SHA-256 {digest}")
        native[suffix] = data
    verify_catalog_readback(native)
    print(f"Rust:   {rust}\nPython: {python}")
    for directory in args.javascript_dir:
        verify_javascript_catalog(rust, directory)


if __name__ == "__main__":
    main()
