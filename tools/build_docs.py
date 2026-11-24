"""Build the unified static site, executable snippets, catalogs and API references.

Run with the provisioned docs/test environment: uv run --no-sync python tools/build_docs.py.
No dependency installation, Git operations or deployment is performed here.
"""
from __future__ import annotations

import html.parser
import pathlib
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parents[1]
SITE = ROOT / "target/docs-website"
REPOSITORY = "https://github.com/ReinderVosDeWael/rocx"
CONTRACTS = (
    "CONTENT_CONTROLS", "DEFERRED-FIELDS", "FIELDS", "FORMATTING",
    "MIGRATION", "NOTES", "NUMBERING", "RANGES",
    "REVISIONS", "STYLES", "TABLES",
)
PUBLIC_PAGES = {
    "COMPATIBILITY.md": "compatibility.md",
    "docs/CLI.md": "editor-cli.md",
    "docs/VIEWER-COMPATIBILITY-CHECKLIST.md": "viewer-compatibility.md",
    "javascript/README.md": "javascript.md",
}


def published_pages() -> dict[pathlib.Path, pathlib.Path]:
    pages = {f"docs/{name}.md": f"contracts/{name}.md" for name in CONTRACTS}
    pages.update(PUBLIC_PAGES)
    return {(ROOT / source).resolve(): pathlib.Path(destination) for source, destination in pages.items()}


def snippet(path: pathlib.Path, label: str) -> str:
    """Extract one uniquely labeled, nonempty region from an executable example."""
    source = path.read_text(encoding="utf-8").splitlines()
    start = [i for i, line in enumerate(source) if line.strip() in
             {f"// docs:start {label}", f"# docs:start {label}"}]
    end = [i for i, line in enumerate(source) if line.strip() in
           {f"// docs:end {label}", f"# docs:end {label}"}]
    if len(start) != 1 or len(end) != 1 or end[0] <= start[0] + 1:
        raise ValueError(f"{path}: expected one nonempty snippet {label!r}")
    return "\n".join(source[start[0] + 1:end[0]])


def expand_snippets(text: str) -> str:
    """Insert source snippets while retaining indentation inside language tabs."""
    def replace(match: re.Match[str]) -> str:
        indent, relative, label = match.groups()
        path = (ROOT / relative).resolve()
        if not path.is_relative_to(ROOT / "examples"):
            raise ValueError("Documentation snippets must come from examples/")
        return "\n".join(indent + line if line else "" for line in snippet(path, label).splitlines())
    return re.sub(r"(?m)^([ \t]*)\{\{snippet:([^:}]+):([^}]+)\}\}$", replace, text)


def language_tabs(text: str) -> str:
    """Present adjacent paired contract examples as synchronized language tabs.

    Only directly adjacent fences are paired; intervening guidance is untouched.
    """
    def replace(match: re.Match[str]) -> str:
        tabs = []
        for language, code in zip(("rust", "python"), match.groups(), strict=True):
            body = "\n".join("    " + line for line in code.rstrip("\n").splitlines())
            tabs.append(f'=== "{language.title()}"\n\n    ```{language}\n{body}\n    ```')
        return "\n\n".join(tabs)
    return re.sub(r"(?ms)^```rust\n((?:(?!^```).)*?)^```\n+```python\n((?:(?!^```).)*?)^```[ \t]*$", replace, text)


def rewrite_links(text: str, source: pathlib.Path, destination: pathlib.Path,
                  mapping: dict[pathlib.Path, pathlib.Path]) -> str:
    """Keep published guides local and send source/internal-file links to GitHub."""
    def replace(match: re.Match[str]) -> str:
        target = urllib.parse.urlsplit(match[1])
        if target.scheme or target.netloc or not target.path:
            return match[0]
        path = (source.parent / urllib.parse.unquote(target.path)).resolve()
        if path in mapping:
            relative = posixpath.relpath(mapping[path].as_posix(), destination.parent.as_posix())
            return "](" + urllib.parse.urlunsplit(("", "", relative, target.query, target.fragment)) + ")"
        relative = path.relative_to(ROOT).as_posix()
        kind = "tree" if path.is_dir() else "blob"
        url = urllib.parse.urlsplit(f"{REPOSITORY}/{kind}/main/{urllib.parse.quote(relative)}")
        return "](" + urllib.parse.urlunsplit((url.scheme, url.netloc, url.path, target.query, target.fragment)) + ")"
    return re.sub(r"\]\(([^\s)]+)\)", replace, text)


class Page(html.parser.HTMLParser):
    """Collect actual local links and anchors from rendered HTML, without a browser."""
    def __init__(self, text: str) -> None:
        super().__init__(convert_charrefs=True)
        self.links: list[str] = []
        self.ids: set[str] = set()
        self.feed(text)

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        for key in (("id", "name") if tag == "a" else ("id",)):
            if attributes.get(key):
                self.ids.add(attributes[key])
        key = "href" if tag in {"a", "link"} else "src" if tag in {"img", "script"} else None
        if key and attributes.get(key):
            self.links.append(attributes[key])


def has_anchor(page: Page, fragment: str, rust_source: bool) -> bool:
    """Recognize native rustdoc encoded IDs and its JavaScript source-line ranges."""
    if fragment in page.ids or urllib.parse.unquote(fragment) in page.ids:
        return True
    match = re.fullmatch(r"(\d+)-(\d+)", fragment) if rust_source else None
    return bool(match and int(match[1]) <= int(match[2]) and
                match[1] in page.ids and match[2] in page.ids)


def check_links(site: pathlib.Path, base_path: str) -> int:
    """Check local HTML/page/asset targets and fragments under a project subpath."""
    site = site.resolve()
    pages = {path: Page(path.read_text(encoding="utf-8")) for path in site.rglob("*.html")}
    errors = []
    for source, page in pages.items():
        for link in page.links:
            parsed = urllib.parse.urlsplit(link)
            if parsed.scheme or parsed.netloc:
                continue  # No network checks; the site must be locally self-contained.
            relative = urllib.parse.unquote(parsed.path)
            if relative.startswith("/"):
                if not relative.startswith(base_path):
                    errors.append(f"{source.relative_to(site)}: outside project subpath: {link}")
                    continue
                target = site / relative[len(base_path):]
            else:
                target = source.parent / relative if relative else source
            target = target.resolve()
            if not target.is_relative_to(site):
                errors.append(f"{source.relative_to(site)}: link escapes site: {link}")
                continue
            if target.is_dir():
                target /= "index.html"
            if not target.is_file():
                errors.append(f"{source.relative_to(site)}: missing target: {link}")
            elif parsed.fragment and target in pages and not has_anchor(
                    pages[target], parsed.fragment,
                    target.is_relative_to(site / "api/rust/src")):
                errors.append(f"{source.relative_to(site)}: missing anchor: {link}")
    if errors:
        raise ValueError("Broken local documentation links:\n" + "\n".join(errors))
    return len(pages)


def run(*command: str) -> None:
    """Run a checked build step from the repository root."""
    subprocess.run(command, cwd=ROOT, check=True)


def main() -> None:
    """Assemble only public docs, then build and check the complete static artifact."""
    import yaml

    (ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="docs-", dir=ROOT / "target") as temporary:
        staging = pathlib.Path(temporary)
        source = staging / "source"
        shutil.copytree(ROOT / "docs/website", source)
        mapping = published_pages()
        for path in source.rglob("*.md"):
            path.write_text(expand_snippets(path.read_text()), encoding="utf-8")
        for path, relative in mapping.items():
            target = source / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(language_tabs(rewrite_links(path.read_text(), path, relative, mapping)), encoding="utf-8")

        examples = staging / "examples"
        examples.mkdir()
        for name in ("plain", "fields"):
            run("cargo", "run", "--quiet", "--locked", "--example", name, "--", str(examples / f"{name}-rust.docx"))
            run(sys.executable, str(ROOT / f"examples/{name}.py"), str(examples / f"{name}-python.docx"))
        if (examples / "plain-rust.docx").read_bytes() != (examples / "plain-python.docx").read_bytes():
            raise ValueError("Getting-started examples must produce identical DOCX bytes")

        downloads = source / "downloads"
        run(sys.executable, "tools/check_examples.py", "--output-dir", str(downloads))
        # A fresh target prevents accidentally publishing stale private/renamed-crate docs.
        rust_target = staging / "rust"
        run("cargo", "doc", "--quiet", "--locked", "--all-features", "--no-deps", "--target-dir", str(rust_target))
        shutil.copytree(rust_target / "doc", source / "api/rust")
        (source / "api/rust/index.html").write_text(
            '<!doctype html><html lang="en"><title>rocx Rust API</title>'
            '<a href="rocx/index.html">rocx Rust API reference</a></html>', encoding="utf-8")
        config = yaml.safe_load((ROOT / "mkdocs.yml").read_text())
        config["docs_dir"] = str(source)
        config["site_dir"] = str(SITE)
        for plugin in config["plugins"]:
            if isinstance(plugin, dict) and "mkdocstrings" in plugin:
                plugin["mkdocstrings"]["handlers"]["python"]["paths"] = [str(ROOT / "python")]
        config_file = staging / "mkdocs.yml"
        config_file.write_text(yaml.safe_dump(config, sort_keys=False), encoding="utf-8")
        run(sys.executable, "-m", "mkdocs", "build", "--strict", "--config-file", str(config_file))
        pages = check_links(SITE, urllib.parse.urlsplit(config["site_url"]).path)
        print(f"Built {pages} checked HTML pages at {SITE}; no deployment performed.")


if __name__ == "__main__":
    main()
