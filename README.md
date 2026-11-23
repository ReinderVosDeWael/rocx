# rocx

Create Word `.docx` files in **Rust** or **Python**. Describe the document in
code; `rocx` writes the DOCX package. It does not render pages, paginate
documents, or calculate fields such as page numbers.

Start here:

- [Getting started](https://reindervosdewael.github.io/rv-docx-rs/getting-started/)
- [Feature guides](https://reindervosdewael.github.io/rv-docx-rs/)
- [Rust API reference](https://reindervosdewael.github.io/rv-docx-rs/api/rust/rocx/)
- [Python API reference](https://reindervosdewael.github.io/rv-docx-rs/api/python/)
- [Compatibility and limitations](COMPATIBILITY.md)

## Example

Rust:

```rust
use rocx::{Document, Error, Paragraph, RunFormat, Section, TextRun};

async fn write_report() -> Result<(), Error> {
    let title = TextRun::new("Hello, world!")
        .with_format(RunFormat::default().with_bold(true));
    let document = Document::new(vec![
        Section::new(vec![Paragraph::from_children(vec![title.into()]).into()]).into(),
    ]);
    document.resolve().await?.save("report.docx")
}
```

Python:

```python
import asyncio
from rocx.declarative import Document, Paragraph, Section, TextRun

async def main():
    document = Document([Section(children=[Paragraph(children=[
        TextRun(text="Hello, world!", bold=True),
    ])])])
    await document.save("report.docx")

asyncio.run(main())
```

Both APIs support text, formatting, tables, images, links, templates, and more.
Their constructors and async behavior differ; the guides describe those
differences and the limits of each feature.

## Project information

- [Migration guide](docs/MIGRATION.md)
- [Changelog](CHANGELOG.md)
- [Visual catalog](https://reindervosdewael.github.io/rv-docx-rs/catalog/)
- [License](LICENSE)

## Contributing

Enable the repository's pre-commit hook once per clone:

```sh
git config --local core.hooksPath .githooks
```

With Cargo and `rustfmt` installed, the hook runs `cargo fmt --check` against
the staged snapshot. If it fails, run `cargo fmt` and stage the intended
formatting changes before retrying. GitHub Actions also checks formatting
whether or not the hook is installed.
