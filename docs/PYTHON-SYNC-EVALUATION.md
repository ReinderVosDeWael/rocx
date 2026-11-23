# Python synchronous usage evaluation

Issue #8 first slice; evaluation only, not a new public API. The existing
`Document.resolve()`, `to_bytes()`, `to_docx()` and `save()` are async. A
synchronous application can own the event loop at its outermost boundary:

```python
import asyncio
from rocx import declarative as d

async def build():
    document = d.Document(
        [d.Section([d.Paragraph(text="Hello")])],
        timestamp="2026-11-21T00:00:00Z",
    )
    await document.save("report.docx")

if __name__ == "__main__":
    asyncio.run(build())
```

In a notebook with top-level await, or inside an async application, use
`await build()` instead; do not call `asyncio.run` on the already-running
loop. For bytes, use `data = await document.to_bytes()`. Notebook support
depends on the environment providing top-level await.

For an application that needs a synchronous function, an application-local
adapter can check `asyncio.get_running_loop()` before creating its coroutine,
use `asyncio.run(...)` when no loop is running, and otherwise raise a clear
error directing callers to the async function. See
[`tools/python_sync_evaluation.py`](../tools/python_sync_evaluation.py).
Checking first avoids creating an unawaited coroutine on the error path.
This is an illustrative boundary, not a supported rocx adapter.

## Evidence

Run from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 uv run --locked --group test python tools/python_sync_evaluation.py
```

Observed: `PASS: script, running-loop guard, async producer, exact bytes, DOCX readback`.
The probe renders a fresh document for each path, with a coroutine producer
that yields once, and a fixed timestamp. It compares the entire DOCX byte
strings from the synchronous script boundary and the awaited running-loop
analog, then reads the text with python-docx. It also verifies that the
adapter refuses to run inside an active loop. This is an asyncio analog, not
a test in a specific notebook frontend. Exact equality is demonstrated for
this deterministic input and environment, not promised for arbitrary
producers, templates, options, or versions.

## Tradeoffs and recommendation

- Prefer the existing async API plus these two boundary patterns. They require
  no library surface or extra dependency, and async producers run on the
  caller's loop in async contexts.
- `asyncio.run` creates and closes a loop each call and blocks the calling
  thread. It cannot be nested in a running loop, and producers or resources
  tied to a different loop may not work. Create fresh coroutine-valued
  producers for each resolution; coroutine objects cannot be awaited twice.
- A background-thread loop could make a blocking call from an active loop,
  but blocks that loop, complicates cancellation, cleanup, and exception
  propagation, and may break loop-affine producers. Nested-loop patching has
  similar interoperability risks. Neither is evaluated or recommended here.
- The probe does not establish performance, cancellation semantics, notebook
  frontend compatibility, or general thread safety. Rendering and file IO may
  still block during an async call. The patterns leave the existing async API,
  its cancellation behavior and deferred-resolution semantics unchanged; they
  do not introduce a second compiler or block a running loop with a sync bridge.

Do not ship a synchronous public API yet. Document the application boundary
patterns if maintainers choose to promote this evaluation; revisit an adapter
only with a defined loop-ownership, cancellation, error, and producer contract.
