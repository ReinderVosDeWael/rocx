# Getting started

## Install and write a document

Use these install commands once rocx is available on your registry. Otherwise,
[run the repository examples](#run-the-repository-examples).

=== "Rust"

    In an existing Cargo project with Rust 1.88 or newer, add rocx and a
    futures executor:

    ```sh
    cargo add rocx
    cargo add futures-executor
    ```

    Put this in `src/main.rs` and run `cargo run`:

    ```rust
    use rocx::{Document, Error, Paragraph, Section, TextRun};

    fn main() -> Result<(), Error> {
        futures_executor::block_on(async {
            let document = Document::new(vec![
                Section::new(vec![
                    Paragraph::from_children(vec![TextRun::new("Hello, world!").into()]).into(),
                ]).into(),
            ]);
            document.resolve().await?.save("report.docx")
        })
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
    from rocx.declarative import Document, Paragraph, Section, TextRun

    async def main():
        document = Document([Section(children=[Paragraph(children=[
            TextRun(text="Hello, world!"),
        ])])])
        await document.save("report.docx")

    asyncio.run(main())
    ```

Rust resolution consumes its declaration. Python resolution leaves the
declaration unchanged and returns an immutable snapshot. See the
[deferred values guide](contracts/DEFERRED-FIELDS.md) for details.

## Run the repository examples

Clone the repository and change into it:

```sh
git clone https://github.com/ReinderVosDeWael/rv-docx-rs.git
cd rv-docx-rs
```

=== "Rust"

    Install Rust 1.88 or newer, then run:

    ```sh
    cargo run --locked --example plain -- report.docx
    ```

=== "Python"

    Install Python 3.12 or newer, Rust 1.88 or newer, and
    [uv](https://docs.astral.sh/uv/). Rust builds the Python extension from
    this checkout. Then run:

    ```sh
    uv sync --locked
    uv run --no-sync python examples/plain.py report.docx
    ```

The checkout examples accept an optional output filename; without one, they
write `examples/plain.docx`.
