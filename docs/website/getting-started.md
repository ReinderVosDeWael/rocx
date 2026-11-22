# Getting started

## Install

=== "Rust"

    Add `rocx` to your Cargo dependencies. For this repository checkout, use a path
    dependency, or run the examples directly with `cargo run --locked --example plain`.
    Rust 1.88 or newer is required. Choose your own executor to poll resolution;
    `futures::executor::block_on` below is only the example's executor.

=== "Python"

    Python 3.12 or newer is required. From a repository checkout, provision the
    locked native development build with `uv sync --locked --group test`, then
    run `uv run --no-sync python examples/plain.py report.docx`.
    Native bytes/save methods do not require python-docx. The optional `interop`
    extra provides editable python-docx conveniences.

These instructions use a source checkout; see the manifests for the current
version and [release guide](contracts/RELEASING.md) for publication status and
platform artifacts. Do not assume a repository version is already on a registry.

## Write a document

These programs are extracted from executable examples at build time. Both take
an optional output filename. A fixed timestamp makes this tutorial reproducible.

=== "Rust"

    ```rust
    {{snippet:examples/plain.rs:quickstart}}
    ```

=== "Python"

    ```python
    {{snippet:examples/plain.py:quickstart}}
    ```

Rust resolution consumes the declaration and owned producers. Python resolution
leaves the declaration unchanged and returns a deeply immutable snapshot; raw
coroutine objects are still single-use. Snapshots can be rendered repeatedly.
Neither API requires the other language's authoring interface.
