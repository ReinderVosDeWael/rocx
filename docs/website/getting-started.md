# Getting started

## Try the examples

These steps use a source checkout. Clone the repository and change into it:

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
    [uv](https://docs.astral.sh/uv/). Rust is needed to build the Python
    extension from this source checkout. Then run:

    ```sh
    uv sync --locked
    uv run --no-sync python examples/plain.py report.docx
    ```

## Write a document

The examples below create a simple document. Each accepts an optional output
filename; without one, it writes `examples/plain.docx`.

=== "Rust"

    ```rust
    {{snippet:examples/plain.rs:quickstart}}
    ```

=== "Python"

    ```python
    {{snippet:examples/plain.py:quickstart}}
    ```

Rust resolution consumes its declaration. Python resolution leaves the
declaration unchanged and returns an immutable snapshot. See the
[deferred values guide](contracts/DEFERRED-FIELDS.md) for details.
