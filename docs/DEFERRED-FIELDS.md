# Deferred fields and shared resolution scenarios

Inventory for the first R1 slice (2026-11-18). This describes authoring and
resolution, not a promise that every resolved value is legal OOXML. Conversion
and domain validation still apply after producers finish.

## Rust field inventory

`Value<T>` accepts immediate, lazy and async producers. `Property<T>` and
`PropertyRef<T>` convert into the same `Value<T>` where
`T: Clone + Send + Sync + 'static`. `Deferred<T>` explicitly caches a computation
without the named reference graph. No field-specific dependency engine exists.

| Declaration | Deferred fields | Currently ordinary fields |
| --- | --- | --- |
| `Document` | Each entry in `sections: Vec<Value<Section>>` | The sections vector itself; creator, title, subject, description, keywords, category, comments, version, comment_author, timestamp, styles, numbering |
| `Section` | children; each header/footer map value | properties; header/footer maps and keys |
| `Paragraph` | format (`ParagraphFormat` aggregate); content's text **or** inline children; comment_text/comment_author when present | numbering |
| `TextRun` | text; format (`RunFormat` aggregate); comment_text/comment_author when present | None beyond condition |
| `ImageRun` | data; transformation (`Option<ImageTransformation>`); alt_text (`Option<AltText>`) | type hint (`r#type`, not used to override inspected image headers) |
| `Header`, `Footer` | children | None beyond condition |
| `Table` | rows, including each row entry | column_widths, layout, style, borders |
| `TableRow` | children, including each cell entry | None beyond condition |
| `TableCell` | children | grid_span, vmerge, borders, background_color, vertical_alignment |
| `Break` | None | type |
| `Tab` | None | None beyond condition |

All component conditions are synchronous `Condition` values, not `Value<bool>`.
Documents have no condition. Domain structs such as `SectionProperties`, format,
style, numbering, border and image-option values do not run producers themselves.
Defer a supported aggregate, or produce a whole component through `Value`.

Rust owned declarations, including their futures and conditions, are not Clone.
Consequently a deferred list of **authoring children** can be produced once but
cannot itself be shared through `PropertyRef<BlockChildren>` or
`PropertyRef<InlineChildren>`. Share cloneable input data and construct owned
children for each consumer instead. This is a current reference-surface gap, not
evidence that arbitrary child-list reference parity has been achieved.

## Python field inventory

Every declared initialization field below, except `condition`, is resolved by
the generic interpreter. Fields accept immediate values, zero-argument factories,
awaitables, `DeferredProperty`/`PropertyRef`, or explicit `Deferred` computations.
This runtime contract is broader than several current type annotations. Mapping
values and sequence entries are recursively resolved; mapping keys are not.

| Declaration | Resolvable fields |
| --- | --- |
| `Document` | sections, creator, title, subject, description, keywords, category, comments, version, comment_author, styles, numbering, timestamp |
| `DocumentTemplate` | path, replacements, paragraph_index, scope (when resolved as a template input) |
| `Section` | children, properties, headers, footers |
| `SectionProperties` | page_size, page_orientation, page_margins, page_numbering, columns, vertical_align, title_page, type |
| `Paragraph` | text, children, comment_text, comment_author, numbering, heading, style, alignment, spacing_before, spacing_after, line_spacing, left_indent, right_indent, first_line_indent, keep_together, keep_with_next, page_break_before, widow_control |
| `TextRun` | text, comment_text, comment_author, bold, italic, underline, font, size, color, superscript, subscript, strike, all_caps, small_caps |
| `ImageRun` | data, type, transformation, alt_text |
| `Header`, `Footer` | children |
| `Table` | rows, column_widths, layout, style, borders |
| `TableRow` | children |
| `TableCell` | children, grid_span, vmerge, borders, background_color, vertical_alignment |
| `TableBorder`, `CellBorder` | side, sz, color, val |
| `Break` | type |
| `Tab` | None |
| `NumberingReference` | id, level |
| `NumberingDefinition` | id, levels |
| `NumberingLevel` | level, start, format, text |
| `ParagraphStyleDefinition` | name, base_style, next_paragraph_style, font, font_size, bold, italic, underline, color, alignment, spacing_before, spacing_after, line_spacing, left_indent, right_indent, first_line_indent, keep_together, keep_with_next, page_break_before, widow_control |
| `TableStyleDefinition` | name, base_style, whole_table, first_row, last_row, first_column, last_column, banding_1_row, banding_2_row, banding_1_column, banding_2_column, top_left_cell, top_right_cell, bottom_left_cell, bottom_right_cell |
| `TableSectionFormat` | font, font_size, bold, italic, underline, color, background, alignment, spacing_before, spacing_after |

`condition` is synchronous and evaluated once per source declaration per pass.
Computed helpers (`hex_color`, methods) and internal identities are not reference
targets. Constructor invariants still apply: a paragraph takes text or children,
not both, even if either is deferred. Python styles retain flat authoring fields;
Rust uses deferred format aggregates rather than duplicating each option.

## Lifetime and ownership contract

- Rust declarations are consumed once. New passes require new declarations;
  retained `Property`/`Deferred` handles cache results for their own lifetimes.
  A property owns its guard independently of a paragraph or image condition.
- Python declarations stay unchanged. Factories support fresh passes; raw
  coroutine objects remain single-use. Field references share work **per pass**,
  and check their source declaration's condition, even outside the rendered tree.
- Concurrent Python root passes do not share field-reference work; an explicit
  `Deferred` does share across passes on the same event loop. A `Deferred` is not
  promised to be portable across event loops while its task is pending.
- Nested component resolution joins the active pass in both languages. Explicit
  reference cycles within that graph fail. Arbitrary opaque futures and cycles
  involving independently owned computations/passes are not introspectable; no
  cross-root cycle-detection guarantee is made.
- Dropping/cancelling one waiter does not invalidate other shared consumers.
  Ending a pass drops Rust owned futures or cancels/drains Python pass tasks.
  Externally retained Rust handles can be polled again; explicit Python Deferred
  tasks remain independently owned and can be explicitly cancelled.
- Image bytes are captured in resolved snapshots. Rendering after source image
  mutation or deletion neither rereads assets nor reruns producers.

## Shared scenario matrix and evidence

Run `cargo test --test resolution_contract --test deferred --test compilation`
and `uv run --no-sync pytest tests/python_bindings/test_resolution_contract.py
tests/python_bindings/test_deferred.py`. Existing failure/cancellation tests in
`tests/resolution.rs` and `test_resolution.py` remain part of the full suites.

| Scenario | Rust evidence | Python evidence / difference |
| --- | --- | --- |
| Text source before/after consumer | `deferred::paragraph_dependencies_share_one_producer_in_any_declaration_order` | `test_deferred::test_paragraph_text_dependency_is_order_independent_and_saved` |
| Formatting references | `compilation::formatting_can_depend_on_another_named_async_property` | `test_deferred::test_dependencies_cover_formatting_comments_nested_children_and_metadata` |
| Image data, transformation, alt text; source last | `resolution_contract::image_fields_accept_references_before_the_source_and_snapshots_do_not_rerun_them` | Same scenario in `test_resolution_contract.py` |
| Concurrent image fields; hidden producers | `resolution_contract::image_fields_start_concurrently_and_hidden_images_do_not_poll_options` | Same scenario in `test_resolution_contract.py` |
| Children and metadata | Async child producers: `async_fields.rs`; direct shared child/metadata fields remain gaps | `test_deferred` covers shared header/cell children, image/section properties and title/creator |
| Diamond | `resolution_contract::diamond_references_share_the_base_producer` | `test_deferred::test_fanout_and_multistage_dependencies_share_one_producer` |
| Direct/indirect cycles | `resolution_contract::direct_and_indirect_reference_cycles_fail_without_waiting` | `test_deferred::test_dependency_cycles_fail_instead_of_waiting_forever` |
| Excluded source | `deferred::typed_property_references_detect_cycles_and_honor_excluded_sources` (property guard) | `test_deferred::test_condition_is_shared_and_excluded_dependency_does_not_run` (declaration guard) |
| Producer error | `deferred::shared_failures_preserve_original_error_and_run_once`; image-option field paths in `resolution_contract` | Original errors preserved by `test_deferred` and image-option tests |
| One waiter cancelled | `deferred::dropping_one_waiter_keeps_other_deferred_consumers_usable` | `test_resolution_contract::test_cancelling_one_field_waiter_keeps_the_other_waiter_usable` |
| Whole pass cancelled | `resolution_contract::dropping_a_pass_drops_owned_work_but_an_external_property_can_resume` | `test_resolution_contract::test_cancelling_a_pass_drains_owned_work_but_not_external_computations` |
| Fresh versus shared passes | `resolution_contract::fresh_programs_rerun_but_explicit_properties_remain_cached` | `test_deferred::test_standalone_references_use_new_passes_and_shared_computations_are_explicit` |
| Concurrent roots / nested passes | `resolution_contract` tests shared root handles and nested cycle detection | `test_resolution_contract` tests per-pass/explicit sharing and nested cycle detection |
| Snapshot after image mutation/deletion | `resolution_contract::image_snapshots_survive_source_mutation_and_deletion` | Same scenario in `test_resolution_contract.py` |

## Remaining R1 work

The matrix exposes gaps rather than declaring R1 complete. Resolve the Rust
document metadata/section/table deferred-field surface in small aggregate slices;
decide a safe contract for shared owned child declarations without cloning futures.
Then complete structured Python/native diagnostics, shared domain invariants and
checked units, snapshot accessors, and profile/read-budget/timestamp options.
Do not add async duplicate fields beside ordinary fields or promise parity that
the table above does not demonstrate.
