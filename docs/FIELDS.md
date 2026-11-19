# Fields and cached results

R5 authors paragraph-local simple and complex WordprocessingML fields in both
frontends. **The library does not evaluate instructions, calculate pages, build
a TOC, fetch referenced documents, or refresh cached results.** A consuming
application may evaluate fields, and may ignore an update request. Schema/readback
checks do not prove display numbering, pagination or visual equivalence.

## Authoring

Rust uses `Field::new(instruction, children)` with optional
`.with_properties(FieldProperties { form, dirty, locked })`. Each argument is a
generic `Value<T>`: immediate, lazy, async, or a compatible `PropertyRef`. Children
are owned authoring programs, not cloneable futures. Share instructions/properties
or immutable `ResolvedInline::Field` content instead.

Python uses `Field(instruction, children=[], form="simple", dirty=None,
locked=None)`. Every field, including fields on `FieldInstruction`, uses the
unchanged generic deferred interpreter. Conditions skip the entire declaration.
There is no field-specific dependency resolver or evaluation engine.

| Instruction | Rust | Python |
| --- | --- | --- |
| Page number | `FieldInstruction::Page` | `FieldInstruction.page()` |
| Total pages | `FieldInstruction::NumPages` | `FieldInstruction.num_pages()` |
| Bookmark reference | `FieldInstruction::Reference(name)` | `FieldInstruction.reference(name)` |
| Heading-level 1–3 TOC | `FieldInstruction::TableOfContents` | `FieldInstruction.table_of_contents()` |
| Unparsed instruction | `FieldInstruction::raw(code)?` | `FieldInstruction.raw(code)` |

Typed REF uses the same `BookmarkName` validation and generated/imported name
catalog as internal hyperlinks. Missing or ambiguous names fail at compilation;
existing bookmarks are not renamed. References in nested cached results are
checked too. REF requests bookmark content, not a page-number calculation.
Use a raw instruction for switches or instruction kinds not represented here.

Raw instructions require nonempty, XML-valid text, but **field syntax and embedded
targets are not parsed or validated**. Whitespace, tabs, line endings, Unicode and
XML-special characters are preserved semantically in both forms. Raw instructions
may have behavior in consuming applications; only author trusted instructions.
The compiler never executes them or follows their external references.

## Cached content and encoding

- The default form is `Simple` / `"simple"`: `w:fldSimple` with `w:instr` and cached
  inline children. Complex form emits balanced run-level `begin`, `instrText`,
  `separate`, result and `end` nodes. Instruction text uses `xml:space="preserve"`.
- Cached display content is supplied by the caller and can be empty or contain
  formatted runs, tabs/breaks, images, hyperlinks and nested fields. Both forms
  project cached text, never instruction text, into the domain plain-text view.
- Nested fields are supported **in results**, not inside instruction programs.
  The type model exposes no standalone delimiters, so dangling, crossing and
  cross-story field markers cannot be authored. Cross-paragraph/multi-block fields
  and multi-paragraph TOC results are not yet exposed. The TOC helper is only an
  instruction plus inline cached placeholder/update contract.
- Explicit range markers inside cached results and fields inside hyperlinks are
  rejected library policies, not claims of XSD prohibitions. Existing leaf-comment
  conveniences remain subject to body-only comment rules. A surrounding paired
  bookmark/comment range can contain a complete field.
- `dirty` flags a stale cached result for a capable consumer to update; `locked`
  requests no recalculation. Neither is a library-side action. Absent, false and
  true remain distinct; flags serialize on `fldSimple` or the complex begin mark.
  Cached-run formatting is independent of these flags. No global update setting
  is inserted and no numbering/layout is calculated.
- Fields use their enclosing story and physical resource owner. Header/footer,
  note and structured-comment fields reuse existing contexts; note references or
  comment content inside results must still obey their normal context rules.
  Images are captured during resolution, and owner-local links use the existing
  relationship registry.

## Templates and evidence

Imported fields are retained, not converted into typed public domain objects.
Template text replacement within simple/complex field results, or across field
boundaries, fails with `unsupported_edit`. Complex-field protection persists
across paragraphs in the same logical story and resets per note/comment entry.
An unclosed imported field conservatively protects subsequent content in that
story; the runtime template loader is not a complete field-conformance validator.
Unrelated edits preserve the instructions and cached content.

Tests cover both encodings, nesting, exact instruction whitespace, flags, typed
targets, source conditions, concurrent producers, captured images, actual owners,
and imported multi-paragraph protection. Rust-generated DOCX readback uses
python-docx's loaded XML and `Run` API for exact cached text/formatting checks;
python-docx does not provide a complete high-level field model, and its
`Paragraph.text` omits simple-field results. Negative controls mutate cached text,
instructions and delimiters after successful loading. Official XSD checks are
supplemented with an independent strict field-balance checker per story, since
the base schema cannot pair complex markers. This checker intentionally rejects
unbalanced markers that consumers might otherwise ignore.

Runnable examples: [`fields.rs`](../examples/fields.rs) and
[`fields.py`](../examples/fields.py). Normative source: ECMA-376 P1 (2016) §17.16,
especially §§17.16.18–19 (`fldChar`, `fldSimple`), §17.16.23 (`instrText`), §17.16.5
(instructions), and Annex A.1 (base schemas).
