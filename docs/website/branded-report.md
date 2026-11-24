# Branded service report

Fill `{{title}}` in an unsigned [Word template](branded_report_template.docx) and insert a table from [tab-separated data](https://github.com/ReinderVosDeWael/rocx/blob/main/examples/branded_report_data.tsv). The [finished report](branded_report_output.docx) keeps the Northstar header and adds a shaded, bold table heading.

From the repository root, run either example:

```sh
cargo run --locked --example branded_report -- report.docx
uv run python examples/branded_report.py report.docx
```

Adapt the [Rust](https://github.com/ReinderVosDeWael/rocx/blob/main/examples/branded_report.rs) or [Python](https://github.com/ReinderVosDeWael/rocx/blob/main/examples/branded_report.py) program for your own template. See [template replacement](recipes.md#edit-an-existing-document) for supported targets and [compatibility](compatibility.md#resolution-editing-and-output) for what survives an edit.
