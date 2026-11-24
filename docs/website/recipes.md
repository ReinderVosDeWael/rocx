# Runnable recipes

## Fields with cached results

Fields store instructions and cached placeholders. A compatible viewer may update
page numbers, references and tables of contents; rocx does not calculate them.

From the checkout:

```sh
cargo run --locked --example fields -- fields.docx
uv run python examples/fields.py fields.docx
```

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

Using the [quickstart setup](getting-started.md), replace `{{name}}` in an unsigned `template.docx` and save a new `filled.docx`. The placeholder may span adjacent runs.

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

Choose the body scope for body paragraphs only. Broader scopes include headers and footers, or related comments, footnotes and endnotes. Ordinary replacement can span runs inside a hyperlink but cannot cross structured boundaries. For protected content, use the [revision](contracts/REVISIONS.md) or [tagged control](contracts/CONTENT_CONTROLS.md#editing-template-controls-by-tag) APIs. See [what survives an edit](compatibility.md#resolution-editing-and-output).
