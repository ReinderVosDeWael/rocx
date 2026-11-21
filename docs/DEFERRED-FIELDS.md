# Deferred fields and shared resolution scenarios

Inventory including R6a–R6v formatting/table/style/list properties (2026-11-20). This describes authoring and
resolution, not a promise that every resolved value is legal OOXML. Conversion
and domain validation still apply after producers finish.

## Rust field inventory

`Value<T>` accepts immediate, lazy and async producers. `Property<T>` and
`PropertyRef<T>` convert into the same `Value<T>` where
`T: Clone + Send + Sync + 'static`. `Deferred<T>` explicitly caches a computation
without the named reference graph. No field-specific dependency engine exists.

| Declaration | Deferred fields | Currently ordinary fields |
| --- | --- | --- |
| `Document` | metadata (`Metadata` aggregate); comment_author, timestamp, styles, numbering, notes, defaults (`Option<DocumentDefaults>` aggregate); each entry in `sections: Vec<Value<Section>>` | The structural sections vector itself |
| `Section` | properties (`Option<SectionProperties>`); children; each header/footer map value | Header/footer maps and keys |
| `Paragraph` | format (`ParagraphFormat` aggregate); numbering; content's text **or** inline children; comment_text/comment_author when present | None beyond condition |
| `Hyperlink` | target (`HyperlinkTarget`); children; properties (`HyperlinkProperties`: tooltip/history) | None beyond condition; nested links intentionally unsupported |
| `Field` | instruction (`FieldInstruction`); cached children; properties (`FieldProperties`: form/dirty/locked) | None beyond condition |
| `RangeStart`, `RangeEnd` | id (`RangeId`); start kind/payload (`RangeKind`) | None beyond condition |
| `Note` | target (`NoteTarget`: kind/id); children | None beyond condition |
| `NoteReference` | target (`NoteTarget`) | None beyond condition |
| `Comment` | children; metadata (`CommentMetadata`: author/date/initials) | None beyond condition |
| `TextRun` | text; format (`RunFormat` aggregate); comment_text/comment_author when present | None beyond condition |
| `ImageRun` | data; transformation (`Option<ImageTransformation>`); alt_text (`Option<AltText>`) | type hint (`r#type`, not used to override inspected image headers) |
| `Header`, `Footer` | children | None beyond condition |
| `Table` | properties (`TableProperties`: column_widths, layout, style, borders, look, alignment); rows, including each row entry | None beyond condition |
| `TableRow` | children, including each cell entry; properties (`TableRowProperties`: height/repeat_header/cant_split) | None beyond condition |
| `TableCell` | properties (`TableCellProperties`: grid_span, vmerge, borders, background_color, vertical_alignment, margins, text_direction); children | None beyond condition |
| `Break` | None | type |
| `Tab` | None | None beyond condition |

All component conditions are synchronous `Condition` values, not `Value<bool>`.
Documents have no condition. Domain structs such as `SectionProperties`, format,
style, numbering, border and image-option values do not run producers themselves.
Defer a supported aggregate, or produce a whole component through `Value`.
`ParagraphFormat.line_height` and style `line_height` use the same aggregate
approach in Rust; Python's line-height property uses generic per-field references.
`tab_stops` uses those same format/styles aggregates and Python field references;
individual `TabStop` values introduce no special evaluation machinery.
Character style definitions reuse Rust `RunFormat` within the deferred styles
collection. Direct `RunFormat.style` and Python `TextRun.style` use the same
generic format/field reference mechanisms as the other formatting properties.
`Document.notes` is a deferred collection of owned `Value<Note>` declarations;
like authoring child lists, its futures are not cloneable. Share cloneable target
keys or immutable child blocks rather than cloning definition programs.

Rust owned declarations, including their futures and conditions, are not Clone.
Consequently a deferred list of **authoring children** can be produced once but
cannot itself be shared through `PropertyRef<BlockChildren>` or
`PropertyRef<InlineChildren>`. This ownership distinction is intentional: share
cloneable input data to build new declarations, or share **resolved** content:

```rust
use rocx::{BlockChildren, Property, ResolvedBlock, Section, Value};
fn shared_section(source: &Property<Vec<ResolvedBlock>>) -> Section {
    Section::default().with_children(Value::from(source.reference()).map(|blocks| {
        blocks.into_iter().map(Into::into).collect::<BlockChildren>()
    }))
}
```

`Block::Resolved`/`Inline::Resolved` carry immutable snapshots through the same
field machinery, without re-running original guards, futures or asset reads.
Use `Vec<ResolvedInline>` for inline children. The same block conversion works
for section, header, footer and cell children. Structural collections of owned
sections/header entries/rows/cells remain authoring programs, not cloneable
reference results; create their shells around shared resolved blocks as needed.
Break type and the ignored image type hint are intentionally static discriminants;
produce a whole Break/ImageRun declaration to choose those asynchronously.

## Python field inventory

Every declared initialization field below, except `condition`, is resolved by
the generic interpreter. Fields accept immediate values, zero-argument factories,
awaitables, `DeferredProperty`/`PropertyRef`, or explicit `Deferred` computations.
This runtime contract is broader than several current type annotations. Mapping
values and sequence entries are recursively resolved; mapping keys are not.

| Declaration | Resolvable fields |
| --- | --- |
| `Document` | sections, creator, title, subject, description, keywords, category, comments, version, comment_author, styles, numbering, timestamp, notes, defaults |
| `DocumentDefaults` | run, paragraph |
| `RunDefaults` | TextRun formatting fields except text/comments/style |
| `ParagraphDefaults` | Paragraph formatting fields except heading/style; all lengths use points |
| `DocumentTemplate` | path, replacements, paragraph_index, scope (when resolved as a template input) |
| `Section` | children, properties, headers, footers |
| `SectionProperties` | page_size, page_orientation, page_margins, page_numbering, columns, vertical_align, title_page, type, bidi, rtl_gutter |
| `Paragraph` | text, children, comment_text, comment_author, numbering, heading, style, alignment, bidi, spacing_before, spacing_after, line_spacing, line_height, tab_stops, left_indent, right_indent, first_line_indent, keep_together, keep_with_next, page_break_before, widow_control |
| `Hyperlink` | target, children, tooltip, history |
| `Field`, `FieldInstruction` | instruction/children/form/dirty/locked on fields; kind/name/code on instructions |
| `RangeStart`, `RangeEnd`, `BookmarkTarget` | id/kind/name/text/author/content on starts; id on ends; name on targets |
| `Note`, `NoteReference`, `Comment` | kind/id/children on notes; kind/id on references; children/author/date/initials on comments |
| `TextRun` | text, comment_text, comment_author, style, bold, italic, underline, font, font_east_asia, font_complex_script, size, size_complex_script, color, superscript, subscript, strike, all_caps, small_caps, language, bold_complex_script, italic_complex_script, rtl, complex_script |
| `ImageRun` | data, type, transformation, alt_text |
| `Header`, `Footer` | children |
| `Table` | rows, column_widths, layout, style, borders, look, alignment |
| `TableLook` | first_row, last_row, first_column, last_column, horizontal_banding, vertical_banding |
| `TableRow` | children, height, repeat_header, cant_split |
| `TableCell` | children, grid_span, vmerge, borders, background_color, vertical_alignment, margins, text_direction |
| `CellMargins` | top, start, bottom, end |
| `RunLanguage` | latin, east_asia, bidi |
| `LineHeight` | rule, value |
| `TabStop` | position, alignment, leader |
| `TableBorder`, `CellBorder` | side, sz, color, val |
| `Break` | type |
| `Tab` | None |
| `NumberingReference` | id, level |
| `NumberingDefinition` | id, levels, start_overrides, level_overrides |
| `NumberingStartOverride` | level, start |
| `NumberingLevel` | level, start, format, text, is_legal, suffix, restart, alignment |
| `ParagraphStyleDefinition` | name, base_style, next_paragraph_style, linked_style, font, font_size, bold, italic, underline, color, alignment, spacing_before, spacing_after, line_spacing, line_height, tab_stops, left_indent, right_indent, first_line_indent, keep_together, keep_with_next, page_break_before, widow_control; script/language/font fields as in TextRun |
| `CharacterStyleDefinition` | name, base_style, linked_style; TextRun formatting fields except text/comments/style |
| `TableStyleDefinition` | name, base_style, whole_table, first_row, last_row, first_column, last_column, banding_1_row, banding_2_row, banding_1_column, banding_2_column, top_left_cell, top_right_cell, bottom_left_cell, bottom_right_cell |
| `TableSectionFormat` | font, font_size, bold, italic, underline, color, background, alignment, spacing_before, spacing_after, line_height, tab_stops, bidi, language, bold_complex_script, italic_complex_script, size_complex_script, rtl, complex_script, font_east_asia, font_complex_script |

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
| Children and metadata | `r1::immutable_resolved_children_can_be_shared_without_cloning_futures`; `r1::metadata_section_table_and_cell_properties_use_the_same_reference_mechanism` | `test_deferred` shared header/cell children; `test_r1` metadata/section/table/cell references |
| Diamond | `resolution_contract::diamond_references_share_the_base_producer` | `test_deferred::test_fanout_and_multistage_dependencies_share_one_producer` |
| Direct/indirect cycles | `resolution_contract::direct_and_indirect_reference_cycles_fail_without_waiting` | `test_deferred::test_dependency_cycles_fail_instead_of_waiting_forever` |
| Excluded source | `deferred::typed_property_references_detect_cycles_and_honor_excluded_sources` (property guard) | `test_deferred::test_condition_is_shared_and_excluded_dependency_does_not_run` (declaration guard) |
| Producer error | `deferred::shared_failures_preserve_original_error_and_run_once`; image-option field paths in `resolution_contract` | Original errors preserved by `test_deferred` and image-option tests |
| One waiter cancelled | `deferred::dropping_one_waiter_keeps_other_deferred_consumers_usable` | `test_resolution_contract::test_cancelling_one_field_waiter_keeps_the_other_waiter_usable` |
| Whole pass cancelled | `resolution_contract::dropping_a_pass_drops_owned_work_but_an_external_property_can_resume` | `test_resolution_contract::test_cancelling_a_pass_drains_owned_work_but_not_external_computations` |
| Fresh versus shared passes | `resolution_contract::fresh_programs_rerun_but_explicit_properties_remain_cached` | `test_deferred::test_standalone_references_use_new_passes_and_shared_computations_are_explicit` |
| Concurrent roots / nested passes | `resolution_contract` tests shared root handles and nested cycle detection | `test_resolution_contract` tests per-pass/explicit sharing and nested cycle detection |
| Snapshot after image mutation/deletion | `resolution_contract::image_snapshots_survive_source_mutation_and_deletion` | Same scenario in `test_resolution_contract.py` |

## Diagnostics, validation and rendering policy

R1 adds stable `rule`, `location` and `cause` attributes to native errors while
retaining standard exception categories. Rule codes come from Rust `Rule::code`,
not display text. Domain traversal uses `DeclarationPath`; package identities
continue to use OPC part/target types. Native conversion errors without a more
specific path use `document`, `template` or `options`. Some legacy compiler
validations retain the broad `invalid_content` rule; R1 is not an exhaustive
classification of every legacy error. A location can be absent for failures
without known declaration/part context. `cause` is an optional display string;
native cause chains also populate Python `__cause__`.

Python producer exceptions are not wrapped or replaced: their original identity,
category and `__cause__` survive, with rule/location/cause context attached.
Producer locations identify the component type and field; native domain paths
identify structural indexes. No promise of a typed Python import/edit model is
made. Dependency errors retain `dependency_cycle` and `excluded_dependency`.

Run/paragraph formats, image transformations, section size/column counts and cell
spans have pure validators shared by the frontends/compiler. `Points` rejects
nonfinite/overflowing conversions; `Emu::twips` uses integer rounding. Image
coordinate and table column caps are application limits, not assertions of the
full OOXML numeric range. XML content, imported inherited geometry, style lookup
and relationship validation still require their existing compilation contexts.
Rust snapshots expose read-only accessors for metadata/timestamp, styles,
numbering, section properties/stories, table/cell properties and image alt text.

Python `rocx.RenderOptions` and `ReadLimits` map directly to Rust's render
options. Pass `options=` to bytes, save or optional interop output on declarations
or snapshots. Profile, timestamp override and archive read budgets are explicit;
rendering does not change snapshots or read source images again. Strict remains
unsupported. Templates are read only at the application boundary. Raw XML parser
budgets remain fixed implementation limits, distinct from selectable archive
budgets. See [MIGRATION.md](MIGRATION.md) for signatures and timing changes.
