# Getting started

## Install and write a document

Choose a language to write `report.docx`.

=== "Rust"

    In a Cargo project with Rust 1.88 or newer:

    ```sh
    cargo add rocx
    ```

    `resolve()` is async. If your app already uses Tokio or another async runtime, call `write_report().await` there. For a standalone executable with a synchronous `main`, run `cargo add futures-executor` for the `block_on` call below.

    Save as `src/main.rs` and run `cargo run`:

    ```rust
    use rocx::{Document, Error, Paragraph, Section};

    async fn write_report() -> Result<(), Error> {
        let section = Section::new(vec![Paragraph::new("Hello, world!").into()]);
        Document::new(vec![section.into()]).resolve().await?.save("report.docx")
    }

    fn main() -> Result<(), Error> {
        futures_executor::block_on(write_report())
    }
    ```

=== "Python"

    In a CPython 3.12 or newer environment:

    ```sh
    python -m pip install rocx
    ```

    A compatible prebuilt wheel does not require a Rust toolchain; building
    the extension from source does. Save this as `report.py` and run
    `python report.py`:

    ```python
    import asyncio
    from rocx import declarative as d

    async def main():
        document = d.Document([d.Section(children=[d.Paragraph(text="Hello, world!")])])
        await document.save("report.docx")

    asyncio.run(main())
    ```

=== "JavaScript"

    Install the [JavaScript development tarball](javascript.md) in your
    Node.js project. Save this as `report.mjs` and run `node report.mjs`:

    ```javascript
    import { writeFile } from "node:fs/promises";
    import { declarative as d, generateDocx } from "rocx-docx";

    const document = d.Document([d.Section([
      d.Paragraph("Hello, world!"),
    ])]);
    await writeFile("report.docx", await generateDocx(document));
    ```

Open `report.docx` in your document viewer. Next, try [filling a template](recipes.md#edit-an-existing-document) or browse the [visual catalog](catalog.md).
