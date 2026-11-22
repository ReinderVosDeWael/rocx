# Visual inspection catalog

The independently authored Rust and Python catalogs cover supported authoring
fields with visible labels. The documentation build regenerates both, checks
their API inventory and content assertions, and compares complete DOCX ZIP bytes
including intermediate templates. Normalized XML equality is not substituted.

- [Download the Rust catalog](downloads/all_features-rust.docx)
- [Download the Python catalog](downloads/all_features-python.docx)
- [Read the catalog and maintenance guide](catalog-guide.md)

Open the files in your document viewer. The same bytes do not imply identical
rendering across viewers. Rendering in Office is not automatically verified by
the documentation build.
