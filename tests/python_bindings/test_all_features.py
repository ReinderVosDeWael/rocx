"""The visual catalogs must retain semantic coverage and exact ZIP byte parity."""
import importlib.util
import io
import pathlib
import zipfile

from lxml import etree
import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("check_examples", ROOT / "tools/check_examples.py")
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


@pytest.fixture(scope="module")
def catalogs(tmp_path_factory):
    """Generate independently through the two public authoring APIs."""
    return checks.generate(tmp_path_factory.mktemp("catalogs"))


def test_all_features_catalogs_have_exact_byte_parity_and_expected_content(catalogs) -> None:
    rust, python = catalogs
    checks.verify_inventory()
    assert checks.exact_bytes(rust.read_bytes(), python.read_bytes())
    checks.exact_bytes(rust.with_suffix(".template.docx").read_bytes(), python.with_suffix(".template.docx").read_bytes())
    checks.verify_content(rust.read_bytes(), rust.with_suffix(".template.docx").read_bytes())


def test_builder_inventory_rejects_a_missing_setter(tmp_path, monkeypatch) -> None:
    source_dir = tmp_path / "src/document"
    source_dir.mkdir(parents=True)
    tests_dir = tmp_path / "tests"
    tests_dir.mkdir()
    for name in ["authoring_builders.rs", "run_format_builders.rs"]:
        (tests_dir / name).write_text((ROOT / "tests" / name).read_text())
    source = (ROOT / "src/document/values.rs").read_text()
    (source_dir / "values.rs").write_text(source.replace("    with_bold => bold: optional<bool>,\n", "", 1))
    monkeypatch.setattr(checks, "ROOT", tmp_path)
    with pytest.raises(AssertionError, match="Missing RunFormat builder fields"):
        checks.verify_rust_builder_inventory()


def test_exact_byte_check_rejects_container_only_differences(catalogs) -> None:
    rust, _ = catalogs
    original = rust.read_bytes()
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as target:
        for name in source.namelist():
            target.writestr(name, source.read(name))
    with pytest.raises(AssertionError, match="ZIP container metadata/compression only"):
        checks.exact_bytes(original, output.getvalue())


def test_content_check_rejects_losing_explicit_false_from_a_builder(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                node = root.xpath("//w:r[w:t[text()='Builder bold disabled, italic enabled.']]/w:rPr/w:b", namespaces=checks.NS)[0]
                node.getparent().remove(node)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_content_check_detects_a_missing_feature_even_if_both_languages_agree(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                for node in root.xpath("//w:tcPr/w:textDirection", namespaces=checks.NS):
                    node.getparent().remove(node)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


@pytest.mark.parametrize("element, value", [
    ("isLgl", "1"), ("suff", "nothing"), ("lvlRestart", "1"), ("lvlJc", "left"),
    ("abstractNumId", "1"), ("rPr/w:b", "1"), ("pStyle", "Heading1"),
])
def test_content_check_rejects_a_changed_numbering_request(catalogs, element: str, value: str) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/numbering.xml":
                root = etree.fromstring(data)
                path = ("w:num[@w:numId='62']/w:abstractNumId" if element == "abstractNumId"
                        else f"w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:{element}")
                node = root.xpath(path, namespaces=checks.NS)[0]
                node.set(f"{{{checks.NS['w']}}}val", value)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_a_changed_table_alignment(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                node = root.xpath("//w:tbl[.//w:t[text()='Table alignment: end']]/w:tblPr/w:jc", namespaces=checks.NS)[0]
                node.set(f"{{{checks.NS['w']}}}val", "left")
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())
