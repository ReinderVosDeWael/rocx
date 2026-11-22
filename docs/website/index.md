# rocx

Create Word `.docx` files with **Rust** or **Python**. Describe the content in
code and let `rocx` build the document. The two APIs have different syntax and
async behavior.

- [Start with a runnable example](getting-started.md).
- Browse the feature guides for supported properties and limitations.
- [Download the paired visual catalogs](catalog.md) to inspect actual output.
- Read the [Rust API](api/rust/rocx/index.html) or [Python API](api/python.md).

`rocx` does not lay out or paginate pages, and fields such as page numbers are
calculated by the document viewer. Documents may look different in Word and
other viewers. See [compatibility and limitations](compatibility.md).

See the [migration guide](contracts/MIGRATION.md) if you are upgrading from an
earlier package name or API.
