# Building the documentation website

The unified Rust/Python site uses MkDocs with Material and mkdocstrings-python.
Dependencies are pinned in the `docs` dependency group and locked in `uv.lock`.
Production Python dependencies are unchanged. JavaScript documentation is deferred
until bindings exist. Rust references retain rustdoc styling and separate search.

## Local setup and build

Use Rust 1.88 or newer and Python 3.12 or newer. In the managed development
environment:

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode uv sync --locked --group test --group docs
UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/build_docs.py
```

Outside the managed environment, omit the environment overrides and provision
your own environment. The build command itself does not install dependencies.
It runs the documented examples, checks exact introductory DOCX bytes, generates
both full visual catalogs with their existing byte/content/inventory checks,
builds Rust and Python references, runs MkDocs in strict mode, and checks local
HTML links, asset/download paths and anchors with the configured project subpath.
External links are not fetched. No deployment is performed.

The complete artifact is `target/docs-website/`. Preview it with:

```sh
python -m http.server --directory target/docs-website 8000
```

Open `http://localhost:8000/`. For a project-subpath preview, serve that directory
under the path specified by `site_url` in `mkdocs.yml` instead. Generated output
and temporary assembly files are disposable build artifacts; do not edit them.

## Maintaining pages and examples

- Edit site pages in `docs/website/` and navigation in `mkdocs.yml`.
- Public feature contracts stay in their existing `docs/*.md` files. The explicit
  allowlist in `tools/build_docs.py` copies them into the assembled site and
  rewrites links to source files back to GitHub. Private maintenance notes,
  dependency audits and historical reorganization records are not copied into
  the website.
- Extract guide snippets from executable repository examples instead of copying
  them. Add a unique `// docs:start label` / `// docs:end label` region in Rust,
  or the `#` equivalents
  in Python. Reference it with `{{snippet:examples/file.rs:label}}` on its own
  indented line inside a code fence. Keep Rust/Python tabs behaviorally aligned
  and document differences. Add newly documented executable programs to the
  builder's run list; extraction alone does not prove a new example was executed.
  Run standalone Markdown snippets manually when changing their API calls; the
  site build does not execute them.
- Keep the paired all-features catalogs and API inventories current for every
  feature. Their generated DOCX files are included as website downloads.
- Python reference pages inspect the source package; setup must build the native
  extension first. Rust output is generated with `cargo doc`.

Focused assembly/link-check tests run without MkDocs:

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode uv run --no-sync pytest -q tests/python_bindings/test_docs.py
```

## CI and deployment

`.github/workflows/docs.yml` builds and uploads a downloadable preview artifact
for pull requests, main-branch pushes and manual runs. Every push to `main`
automatically deploys the successfully built site to GitHub Pages. Manual runs
on `main` also redeploy; pull requests and manual runs on other branches never
deploy. Failed builds prevent deployment.

Repository setup:

1. Configure GitHub Pages to use GitHub Actions. Ensure the `github-pages`
   environment permits main-branch deployment without manual approval if fully
   automatic publication is required.
2. Confirm `site_url` matches the intended Pages project path.

No deployment input or repository-variable opt-in is needed. Main-branch runs
are not canceled while deploying; newer pull-request preview builds can cancel
older previews. The deploy job uses the Pages artifact and deployment actions
with job-scoped `pages:write` / `id-token:write` permissions. The workflow does
not itself configure repository Pages settings. No hosted CI or deployment
success is implied by a local build.
