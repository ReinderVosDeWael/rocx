# Runnable recipes

## Fields with cached results

Fields store instructions and cached placeholders. A compatible viewer may update
page numbers, references and tables of contents; rocx does not calculate them.

The tabs below are extracted from the runnable field examples. In the checkout,
run `cargo run --locked --example fields -- fields.docx` or
`uv run --no-sync python examples/fields.py fields.docx`.

=== "Rust"

    ```rust
    {{snippet:examples/fields.rs:fields}}
    ```

=== "Python"

    ```python
    {{snippet:examples/fields.py:fields}}
    ```

See [the field contract](contracts/FIELDS.md) for supported instructions,
container restrictions and consumer update behavior. Other executable recipes
cover [ranges](contracts/RANGES.md), [notes](contracts/NOTES.md), and
[deferred values](contracts/DEFERRED-FIELDS.md).

## Edit an existing document

Start with an unsigned `template.docx` containing `{{name}}` in a body paragraph.
The placeholder may be split across adjacent runs, for example `{{na` and
`me}}`. These programs save the result to a new `filled.docx` without appending
new document content. Use the installation and runtime setup from
[getting started](getting-started.md).

=== "Rust"

    ```rust
    use rocx::{Document, Error};
    use rocx::application::DocumentTemplate;
    use rocx::template::EditScope;

    fn main() -> Result<(), Error> {
        futures_executor::block_on(async {
            let mut template = DocumentTemplate::new("template.docx");
            template.edits.scope = EditScope::Body;
            template.edits.replacements.push(("{{name}}".into(), "Ada".into()));
            Document::new(vec![]).resolve().await?
                .save_with_template("filled.docx", &template)
        })
    }
    ```

=== "Python"

    ```python
    import asyncio
    from rocx.declarative import Document, DocumentTemplate

    async def main():
        template = DocumentTemplate(
            "template.docx", replacements={"{{name}}": "Ada"}, scope="body"
        )
        await Document([]).save("filled.docx", template)

    asyncio.run(main())
    ```

This recipe explicitly edits the body. Other scopes include headers and
footers, or supported stories including related comments, footnotes and
endnotes; unknown parts remain opaque. Replacement can span runs and work
inside hyperlinks, but not cross structured inline boundaries or alter field,
revision or content-control contents. Inserted text takes the first matched
run's position; comment and bookmark markers remain in place. See the separate
[revision](contracts/REVISIONS.md) and
[content-control](contracts/CONTENT_CONTROLS.md) contracts for their editing APIs.

Untouched part payloads retain their bytes, but edited XML and the ZIP container
are reserialized. Templates declaring an OPC digital-signature origin are
rejected, even without edits; use an unsigned copy. See
[compatibility](compatibility.md) for namespace and other preservation limits.
