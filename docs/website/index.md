# rocx

Create Word `.docx` files with **Rust**, **Python**, or **JavaScript**.
Describe a `Document` containing `Section` and `Paragraph` components in
each language. The [JavaScript package](javascript.md) runs the same Rust
engine in Node.js and Chromium via WebAssembly.

- [Start with a runnable example](getting-started.md).
- [Build a branded report from data and an editable template](branded-report.md).
- Browse the feature guides for supported properties and limitations.
- [Download the paired visual catalogs](catalog.md) to inspect actual output.
- Read the [Rust API](api/rust/rocx/index.html), [Python API](api/python.md),
  or [JavaScript API](api/javascript.md).

`rocx` does not lay out or paginate pages. A compatible viewer may update fields
such as page numbers. Documents may look different in Word and
other viewers. See [compatibility and limitations](compatibility.md).

See the [migration guide](contracts/MIGRATION.md) if you are upgrading from an
earlier package name or API.
