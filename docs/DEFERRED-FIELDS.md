# Deferred values

Use a producer when a field depends on work that should happen at resolution. Use a reference when several fields need the **same result**.

```python
from rocx import declarative as d

title = d.Paragraph(text=lambda: "Quarterly report")
subtitle = d.Paragraph(text=title.deferred("text"))
document = d.Document([d.Section(children=[title, subtitle])])
```

## Python field inventory

Declared constructor fields except `condition` accept literals, zero-argument factories, awaitables, field references or explicit `Deferred` computations. This includes nested formatting records, sequence entries and mapping **values**, not keys. Type hints show ordinary inputs. Methods and computed properties are not reference targets. A paragraph takes text **or** children, even when deferred.

`condition` is synchronous and evaluated once per source declaration per pass. A false condition skips producers. Referencing an excluded source fails, even if the source is outside the rendered tree.

## Rust field inventory

`Value<T>` accepts immediate, lazy and async inputs. Share a `Property<T>` or `PropertyRef<T>` when `T: Clone + Send + Sync + 'static`. Use `Deferred<T>` to cache a computation independently of the named reference graph.

| Declaration | What to defer |
| --- | --- |
| Document | Each section entry, metadata, author, timestamp, styles, numbering, notes or defaults; the sections vector itself is static. |
| Section, header, footer | Children, section properties and each header/footer map value; keys are static. |
| Paragraph, text run | Text or inline children, format, paragraph numbering, optional comment text/author. |
| Hyperlink, field | Target/instruction, children, properties. |
| Range, note, comment | Marker ID and start payload, note/reference target, note/comment children, comment metadata. |
| Table, row, cell | Properties and children, including individual row/cell entries. |
| Image | Data, transformation and alt text. Image type hints do not override encoded headers. |

Defer a format/options **record**, not its individual members. For static fields such as break type, produce the whole component. Conditions are synchronous; documents have none. Authoring children own their futures and cannot be cloned. Share cloneable inputs or immutable resolved content instead:

```rust
use rocx::{BlockChildren, Property, ResolvedBlock, Section, Value};
fn shared_section(source: &Property<Vec<ResolvedBlock>>) -> Section {
    Section::default().with_children(Value::from(source.reference()).map(|blocks: Vec<ResolvedBlock>| {
        blocks.into_iter().map(Into::into).collect::<BlockChildren>()
    }))
}
```

Use `Vec<ResolvedInline>` for inline content. Resolved blocks work in sections, headers, footers and cells.

## Lifetime and ownership contract

| | Rust | Python |
| --- | --- | --- |
| Resolve again | Resolution consumes the declaration. Build a fresh one. | Declaration stays unchanged. Use factories for fresh work; raw coroutines are single-use. |
| Share work | `Property` and `Deferred` cache for their handle's lifetime. A property's guard is independent of a component's condition. | Field references share per pass. Explicit `Deferred` shares across passes on the same event loop; a pending task is not portable across loops. |
| Cancel | Dropping a pass drops owned futures. Retained handles can be polled again. | Cancelling a pass drains its tasks. Independently owned `Deferred` tasks can be cancelled explicitly. |

Dropping one waiter does not invalidate other consumers. Nested resolution joins the active pass; explicit reference cycles there fail with `dependency_cycle`. Avoid cycles across independent root passes or opaque futures: they may wait indefinitely.

## Rendering and errors

Render an immutable snapshot repeatedly without rereading images or rerunning producers. Python `options=` on bytes, save and optional interop output accepts `rocx.RenderOptions` (timestamp override, profile and archive `ReadLimits`). Overrides do not change the snapshot. Strict output is unsupported; archive limits do not change XML parser limits.

Python errors expose `rule`, `location` and `cause` when available. Use rule codes such as `excluded_dependency`, not message text. Producer exceptions keep their identity and `__cause__`; their locations name fields, whereas domain errors may identify a structural index or package part. Some validation needs the template, styles or relationships and can fail at render time.
