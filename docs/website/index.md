# rocx

Author DOCX documents in **Rust** and **Python**, using deferred values,
conditions and immutable resolved snapshots. Both frontends share a Rust domain
and OOXML compiler; their constructor syntax and resolution behavior differ.

- [Start with a runnable example](getting-started.md).
- Browse the feature guides for supported properties and limitations.
- [Download the paired visual catalogs](catalog.md) to inspect actual output.
- Read the [Rust API](api/rust/rocx/index.html) or [Python API](api/python.md).

The crate and Python distribution are both named `rocx`. The repository retains
the name `rv-docx-rs`. See [migration](contracts/MIGRATION.md) for older imports.

JavaScript bindings are not available. No JavaScript examples or API reference
are published here yet.

The library authors formatting and layout requests; it does not paginate, shape
text, evaluate fields or compute effective styles. DOCX schema validation and
exact Rust/Python byte parity do not establish visual equivalence in Office.
Consult [compatibility and limitations](compatibility.md) before adopting a feature.

Rust reference pages use native rustdoc styling and search. Python reference pages
use MkDocs. The first version does not unify search across both API references.
