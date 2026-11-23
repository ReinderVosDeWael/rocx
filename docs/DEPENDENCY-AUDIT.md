# Dependency alert triage

## Dependency refresh (2026-11-23)

The refreshed lockfiles select ZIP 9.7.3 and PyO3 0.30.1, above the patched
versions below. ZIP 9.7.3 is the newest version compatible with Rust 1.88 and
uses only Flate2 deflate support; AES and archive extraction are not used.

The three documentation alerts open at the start of the refresh concerned
[MkDocs Material](https://github.com/advisories/GHSA-xvg9-69gf-fjrf) and
[PyMdown Extensions](https://github.com/advisories/GHSA-gm37-52c6-37mw)
([second advisory](https://github.com/advisories/GHSA-9xwg-3r6f-jcx2)).
The refreshed lockfile uses 9.8.2 and 13.1.2 respectively, above the reported
fixed versions. GitHub tracks alert closure after scanning the default branch.

## Historical triage (2026-11-17)

The following dispositions applied to ZIP 2.6.1 and PyO3 0.27.2. They record
source and feature inspection from before the dependency upgrades.

## Dispositions

| Advisory | Dependency / scope | Evidence and current disposition |
| --- | --- | --- |
| [GHSA-m665-v7pm-7grj](https://github.com/advisories/GHSA-m665-v7pm-7grj) (critical) | zip, production | Parser differential between central-directory and streaming readers. `src/opc/mod.rs::Package::load_with_limits` reads entries into bounded in-memory buffers through `ZipArchive::by_index`; it neither extracts archive paths to disk nor passes the original archive to a streaming extractor. `src/ooxml/defaults.rs` reads a bundled trusted archive through `by_name`. The reported overwrite chain is not used here. Retain for now; do not treat successful DOCX loading as sanitization for another ZIP reader. |
| [GHSA-q7v9-x93c-xqv3](https://github.com/advisories/GHSA-q7v9-x93c-xqv3) (high) | zip, production | Unbounded symlink-target allocation in `ZipArchive::extract`. No filesystem extraction API is called. OPC checks declared sizes and limits actual decompression with `Read::take` before retaining bytes. Retain for now; reassess before introducing extraction. |
| [GHSA-8hxf-qr8p-62x4](https://github.com/advisories/GHSA-8hxf-qr8p-62x4) (moderate) | zip, production | AES entry authentication bypass. `Cargo.toml` disables default zip features and enables only `deflate-flate2`/`flate2`; `cargo tree -e features -i zip` confirms AES is absent. No password/decryption API is called. Retain with AES disabled. |
| [GHSA-36hh-v3qg-5jq4](https://github.com/advisories/GHSA-36hh-v3qg-5jq4) (high) | PyO3, optional Python extension | Overflow in typed list/tuple iterators' `nth`/`nth_back`. Adapters use `PyAny::try_iter`, checked `get_item`, and extraction; no typed list/tuple iterator `nth`/`nth_back` call is present. The `nth` in `src/ooxml/mod.rs` is a standard Rust XML-children iterator, not a Python iterator. Retain for now; recheck typed iteration changes. |
| [GHSA-chgr-c6px-7xpp](https://github.com/advisories/GHSA-chgr-c6px-7xpp) (moderate) | PyO3, optional Python extension | Missing Sync bound on `PyCFunction::new_closure`. No closure-backed Python function is constructed; the extension exports a `#[pyfunction]` via `wrap_pyfunction!` in `src/adapters/python/mod.rs`. Retain for now; reassess new closure-backed callbacks. |

The reported patched versions are zip 9.0.0 for AES/symlink handling, zip 9.0.5
for the parser differential, and PyO3 0.29.0 for both PyO3 reports. Major/minor
API migrations need separate MSRV, packaging, parser and preservation testing;
there is no evidence-based reason to perform those migrations inside CI setup.
These dispositions are source/feature inspection, not exploit reproduction or
a general security audit. Revisit them when dependency features or call sites
change, and before a release.

None of these five alerts concerns `python-docx` (optional interop). This is not
evidence that it is free of other vulnerabilities. No production Python
dependencies were added; the clean-wheel probe checks that only the optional
interop requirement is present.
