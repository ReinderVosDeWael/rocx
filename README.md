# rocx

Create Word `.docx` files in **Rust** or **Python**. Describe the document in
code; `rocx` writes the DOCX package. It does not render pages, paginate
documents, or calculate fields such as page numbers.

Start here:

- [Getting started](https://reindervosdewael.github.io/rocx/getting-started/)
- [Feature guides](https://reindervosdewael.github.io/rocx/)
- [Rust API reference](https://reindervosdewael.github.io/rocx/api/rust/rocx/)
- [Python API reference](https://reindervosdewael.github.io/rocx/api/python/)
- [JavaScript authoring for Node and browsers](https://github.com/ReinderVosDeWael/rocx/tree/main/javascript) (Wasm tarball; not on the npm registry)
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
- [Visual catalog](https://reindervosdewael.github.io/rocx/catalog/)
- [Runnable branded-report example](https://reindervosdewael.github.io/rocx/branded-report/)
- [License](LICENSE)

## Contributing

Install [prek](https://prek.j178.dev/) and enable its Git hook once per clone:

```sh
uv tool install prek
prek install
```

With Cargo and `rustfmt` installed, prek runs `cargo fmt --check` before each
commit, temporarily setting aside unstaged edits to tracked files. If it
fails, run `cargo fmt` and stage the intended formatting changes before
retrying. GitHub Actions checks formatting even if prek is not installed.
