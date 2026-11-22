# Runnable recipes

## Fields with cached results

Fields store instructions and cached placeholders. The consuming application,
not rocx, calculates page numbers, references and tables of contents.

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
