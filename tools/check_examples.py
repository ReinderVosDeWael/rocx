"""Generate both visual catalogs; require exact bytes and independent coverage.

Run with the development environment: uv run --no-sync python tools/check_examples.py
This uses python-docx/lxml for verification only, never to generate the catalog.
"""
import argparse
import dataclasses
import hashlib
import io
import json
import pathlib
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
      "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
      "r": "http://schemas.openxmlformats.org/package/2006/relationships"}
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


def exact_bytes(rust: bytes, python: bytes) -> str:
    """Compare the ZIP containers themselves, with part-level failure diagnostics."""
    if rust != python:
        with zipfile.ZipFile(io.BytesIO(rust)) as a, zipfile.ZipFile(io.BytesIO(python)) as b:
            names = set(a.namelist()) | set(b.namelist())
            changed = [name for name in sorted(names) if name not in a.namelist() or name not in b.namelist() or a.read(name) != b.read(name)]
        raise AssertionError(f"DOCX bytes differ: Rust={len(rust)}, Python={len(python)}; changed parts={changed or 'ZIP container metadata/compression only'}")
    return hashlib.sha256(rust).hexdigest()


def verify_content(data: bytes, template: bytes) -> None:
    """Read actual generated output, not source text or self-reported coverage."""
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
        assert "Concrete numbering-level replacements (numId=60)" in text
        assert values(document, "//w:numPr/w:ilvl/@w:val") >= set(map(str, range(9)))
        assert values(document, "//w:bookmarkStart/@w:name") == {"CatalogBookmark", "OverlappingBookmark"}
        assert document.xpath("//w:hyperlink[@w:anchor='CatalogBookmark']", namespaces=NS)
        assert len(document.xpath("//w:fldSimple", namespaces=NS)) >= 5
        assert len(document.xpath("//w:fldChar[@w:fldCharType='begin']", namespaces=NS)) >= 6
        assert "[TOC cached placeholder]" in text and "[nested PAGE cache]" in text
        for name, tag, label in [("footnotes", "footnote", "Footnote body"), ("endnotes", "endnote", "Endnote body"), ("comments", "comment", "Structured comment")]:
            root = etree.fromstring(package.read(f"word/{name}.xml"))
            assert label in "".join(root.xpath("//w:t/text()", namespaces=NS))
            assert root.xpath(f"//w:{tag}", namespaces=NS)
        assert values(document, "//w:sectPr/w:type/@w:val") >= {"nextPage", "nextColumn", "continuous", "evenPage", "oddPage"}
        for prop in ["bidi", "rtlGutter"]:
            assert values(document, f"//w:sectPr/w:{prop}/@w:val") == {"0", "1"}
        assert values(document, "//w:sectPr/w:vAlign/@w:val") == {"top", "center", "bottom", "both"}
        assert values(document, "//w:sectPr/w:pgSz/@w:orient") >= {"portrait", "landscape"}
        assert len(document.xpath("//w:headerReference", namespaces=NS)) >= 3
        assert len(document.xpath("//w:footerReference", namespaces=NS)) >= 3
        assert {p.rsplit(".", 1)[-1] for p in package.namelist() if p.startswith("word/media/")} >= {"png", "jpg", "gif", "bmp", "tiff"}
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


def main() -> None:
    """Generate visible artifacts and report the exact-byte SHA-256."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=pathlib.Path, default=ROOT / "examples")
    args = parser.parse_args()
    verify_inventory()
    rust, python = generate(args.output_dir)
    digest = exact_bytes(rust.read_bytes(), python.read_bytes())
    exact_bytes(rust.with_suffix(".template.docx").read_bytes(), python.with_suffix(".template.docx").read_bytes())
    verify_content(rust.read_bytes(), rust.with_suffix(".template.docx").read_bytes())
    print(f"Byte-identical DOCX: {len(rust.read_bytes())} bytes; SHA-256 {digest}")
    print(f"Rust:   {rust}\nPython: {python}")


if __name__ == "__main__":
    main()
