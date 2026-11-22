"""Documentation assembly checks do not require the optional MkDocs environment."""
import importlib.util
import pathlib

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("build_docs", ROOT / "tools/build_docs.py")
docs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(docs)


def test_snippet_is_extracted_from_the_executable_source(tmp_path) -> None:
    source = tmp_path / "example.rs"
    source.write_text("ignored\n// docs:start demo\nlet answer = 42;\n// docs:end demo\n")
    assert docs.snippet(source, "demo") == "let answer = 42;"
    source.write_text(source.read_text() + "// docs:start demo\n")
    with pytest.raises(ValueError, match="one nonempty snippet"):
        docs.snippet(source, "demo")


def test_language_tab_snippets_retain_indentation_and_have_no_marker_comments() -> None:
    result = docs.expand_snippets("    {{snippet:examples/plain.rs:quickstart}}")
    assert "    fn main()" in result
    assert "docs:start" not in result
    assert "{{snippet:" not in result


def test_snippets_cannot_read_outside_examples() -> None:
    with pytest.raises(ValueError, match="must come from examples"):
        docs.expand_snippets("{{snippet:pyproject.toml:secret}}")


def test_paired_contract_examples_form_tabs_without_swallowing_intervening_guidance() -> None:
    paired = '```rust\nlet answer = 42;\n```\n\n```python\nanswer = 42\n```'
    result = docs.language_tabs(paired)
    assert '=== "Rust"' in result and '=== "Python"' in result
    assert '    let answer = 42;' in result and '    answer = 42' in result
    separate = paired.replace('\n\n```python', '\n\nImportant difference.\n\n```python')
    assert docs.language_tabs(separate) == separate


def test_published_docs_are_an_allowlist_without_internal_handoffs() -> None:
    assert {"PLAN", "DEPENDENCY-AUDIT", "REORGANIZATION"}.isdisjoint(docs.CONTRACTS)
    assert "FORMATTING" in docs.CONTRACTS
    for name in docs.CONTRACTS:
        assert (ROOT / f"docs/{name}.md").is_file()


def test_source_links_are_rewritten_without_copying_private_documents() -> None:
    source = ROOT / "docs/FORMATTING.md"
    destination = pathlib.Path("contracts/FORMATTING.md")
    mapping = {(ROOT / "docs/STYLES.md").resolve(): pathlib.Path("contracts/STYLES.md")}
    result = docs.rewrite_links("[styles](STYLES.md#section) [plan](PLAN.md)", source, destination, mapping)
    assert "[styles](STYLES.md#section)" in result
    assert "/blob/main/docs/PLAN.md" in result


def test_link_checker_supports_project_subpaths_and_checks_anchors(tmp_path) -> None:
    (tmp_path / "index.html").write_text('<a href="/project/guide/#example">Guide</a>')
    guide = tmp_path / "guide"
    guide.mkdir()
    (guide / "index.html").write_text('<h1 id="example">Example</h1><a href="../">Home</a>')
    assert docs.check_links(tmp_path, "/project/") == 2
    (guide / "index.html").write_text('<h1 id="different">Example</h1>')
    with pytest.raises(ValueError, match="missing anchor"):
        docs.check_links(tmp_path, "/project/")


@pytest.mark.parametrize("href", ["/missing-project/", "missing.docx", "../escape.html"])
def test_link_checker_rejects_missing_downloads_and_bad_project_links(tmp_path, href) -> None:
    (tmp_path / "index.html").write_text(f'<a href="{href}">Bad link</a>')
    with pytest.raises(ValueError, match="Broken local documentation links"):
        docs.check_links(tmp_path, "/project/")


def test_rustdoc_encoded_ids_and_source_ranges_are_checked_not_skipped() -> None:
    page = docs.Page('<div id="impl-From%3CT%3E"></div><a id=1></a><a id=2></a>')
    assert docs.has_anchor(page, "impl-From%3CT%3E", False)
    assert docs.has_anchor(page, "1-2", True)
    assert not docs.has_anchor(page, "1-3", True)
    assert not docs.has_anchor(page, "2-1", True)
    assert not docs.has_anchor(page, "1-2", False)
    assert not docs.has_anchor(page, "impl-Missing%3CT%3E", False)
