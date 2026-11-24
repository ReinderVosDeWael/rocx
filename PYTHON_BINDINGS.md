# Python: resolve and save a document

```python
import asyncio
from rocx import declarative as d

async def main():
    title = d.Paragraph(text=lambda: "Report")
    document = d.Document([d.Section(children=[title])])
    snapshot = await document.resolve()
    await snapshot.save("report.docx")
    data = await snapshot.to_bytes()

asyncio.run(main())
```

Use CPython 3.12+. Wheels include the native extension; source builds need Rust. For an editable python-docx document, install `rocx[interop]` and call `await snapshot.to_docx()`. Native bytes and file output do not require python-docx.

## Resolve once or save directly

- `await document.save(path_or_binary_stream)` and `await document.to_bytes()` resolve automatically. Paths are replaced only after successful compilation.
- `await document` and `await document.resolve()` return an immutable snapshot without changing the declaration. Render the snapshot to reuse its timestamp, inputs and image bytes.
- Pass keyword-only `options=` to any output method. `rocx.RenderOptions` controls the timestamp, profile and archive limits (part count, per-part bytes, total expanded bytes). Strict is unsupported. See the [options example](examples/resolution_options.py).
- Resolution uses asyncio. Independent fields run concurrently; native rendering is a synchronous CPU step that releases the GIL.

## Share values

`title.deferred("text")` (or `PropertyRef(title, "text")`) shares that field's result within a resolution pass, even if the source is outside the document. Conditions must be synchronous. Unknown fields and references to `condition` raise `AttributeError`. Excluded sources and cycles raise `DependencyError` (a `ValueError`) with rule `excluded_dependency` or `dependency_cycle`.

Use factories to rerun work across passes; bare coroutines are single-use. `Deferred(factory_or_awaitable)` instead caches a result or error across passes; the owner can call `cancel()`. See [deferred values](docs/DEFERRED-FIELDS.md) for nesting, cancellation and ownership.

## Formatting and errors

Run colors accept RGB tuples, `"auto"` or theme mappings such as `{"theme": "accent1", "tint": 128}`. Underline accepts booleans or `single`, `double`, `dotted`, `dash`, `wave`, `words`, `none`. See [formatting](docs/FORMATTING.md) and the [Python API](https://reindervosdewael.github.io/rocx/api/python/).

Classify native errors by exception type (`ValueError`, `KeyError`, `OSError`) and `rule`, not message text. `location` can identify a declaration field or OPC part; `cause` is optional. Producer exceptions retain their identity and `__cause__`. Some validations report the general `invalid_content` rule.
