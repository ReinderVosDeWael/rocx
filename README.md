# rocx

Create Word `.docx` files in **Rust**, **Python**, or **JavaScript**. Describe the document in
code; `rocx` writes the DOCX package. It does not render pages, paginate
documents, or calculate fields such as page numbers.

[Install and write your first DOCX](https://reindervosdewael.github.io/rocx/getting-started/) ·
[Feature guides](https://reindervosdewael.github.io/rocx/) · [Compatibility](COMPATIBILITY.md)

API: [Rust](https://reindervosdewael.github.io/rocx/api/rust/rocx/) ·
[Python](https://reindervosdewael.github.io/rocx/api/python/) ·
[JavaScript](https://reindervosdewael.github.io/rocx/javascript/) (Node/browser tarball, not on npm).

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

JavaScript (Node ESM):

```javascript
import { writeFile } from "node:fs/promises";
import { declarative as d, generateDocx } from "rocx-docx";

const document = d.Document([d.Section([
  d.Paragraph([d.TextRun("Hello, world!", { format: { bold: true } })]),
])]);
await writeFile("report.docx", await generateDocx(document));
```

## Project information

- [Create and edit DOCX files from an MCP client](docs/MCP.md)

- [Migration guide](docs/MIGRATION.md)
- [Changelog](CHANGELOG.md)
- [Visual catalog](https://reindervosdewael.github.io/rocx/catalog/)
- [Runnable branded-report example](https://reindervosdewael.github.io/rocx/branded-report/)
- [License](LICENSE)

- [Contributing](CONTRIBUTING.md)
