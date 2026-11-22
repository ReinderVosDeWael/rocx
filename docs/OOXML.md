# OOXML references and implementation coverage

This is a specification map for contributors, not a declaration of full OOXML
conformance. The standards and explicit product choices determine the domain
model and behavior.
See [the module guide](MODULES.md) for implemented boundaries and
[the original review](ARCHITECTURE.md) for historical context.

## Source editions and authority

Use the [official ECMA-376 publication page][ECMA] and these specific editions:

| Reference | Official download | Applies to |
| --- | --- | --- |
| **P1**: ECMA-376 Part 1, fifth edition, December 2016 | [Part 1 ZIP][P1] | Fundamentals, WordprocessingML, DrawingML, shared vocabularies, schemas |
| **P2**: ECMA-376 Part 2, fifth edition, December 2021 | [Part 2 ZIP][P2] | Open Packaging Conventions (OPC) |
| **P3**: ECMA-376 Part 3, fifth edition, December 2015 | [Part 3 ZIP][P3] | Markup Compatibility and Extensibility (MCE) |
| **P4**: ECMA-376 Part 4, fifth edition, December 2016 | [Part 4 ZIP][P4] | Transitional migration features, including legacy vocabularies |

Clause numbers below were checked against the PDFs inside those official ZIPs.
They are **clause numbers, not PDF page numbers**. Part 2 was reorganized in its
2021 edition: packaging references copied from older editions can point to the
wrong clause. Follow the named edition and title, not just a bare section number.
The official downloads include supplementary schema material; do not substitute
an SDK type definition or a tutorial for the normative content model.

For Microsoft-specific behavior, consult [MS-OI29500] for implementation notes
on ISO/IEC 29500, [MS-OE376] for ECMA implementation notes, and [MS-DOCX] for Word
extensions. These have different roles: an implementation note describes Office's
behavior; an extension specification defines additional markup; neither makes an
extension part of the base standard. Record the revision of an extension document
when implementing it. The base specification links are edition-pinned; Microsoft's
landing pages can advance to newer revisions.

## Conformance and support profiles

P1 §2 describes Strict conformance; P4 §2 describes Transitional conformance.
Today's engine emits the Transitional-family namespace URIs from
[src/xml/mod.rs](../src/xml/mod.rs) and uses a python-docx blank template. That identifies
its current serialization vocabulary; it does **not** certify complete
Transitional conformance. The explicit profile currently supports Transitional and rejects Strict.
The engine retains opaque MCE markup; independent developer validation supports
base-schema checks after MCE preprocessing, not arbitrary extension interpretation.

`RenderOptions` carries the output profile, timestamp override and read budgets.
New extension support must additionally record its target capability/version.
Distinguish generation, interpretation, safe editing, and preservation of each
feature. Strict support will require changes to namespaces, relationship types,
lexical values and permitted markup, not just replacement of one URI. Recognizing
or retaining an unknown part is not equivalent to understanding its feature.

## Packaging and part identity

| Concern | Specification | Current code and implications |
| --- | --- | --- |
| Part identity/names | P2 §6.2.2, Part names | [opc](../src/opc/mod.rs): validates typed part-name values and rejects equivalent duplicate names. |
| Content/media types | P2 §6.2.3; §7.3.7, Mapping the Media Types stream | `Package::content_type` writes overrides. The registry models defaults/overrides and checks coverage for package parts. |
| Relative target resolution | P2 §6.4, Resolving relative references | `Package::related` uses centralized validated resolution and relativization from actual owners. |
| Relationships and target modes | P2 §6.5, Relationships | `Package::{relation,related}`. IDs belong to an owner; internal parts and external targets have distinct representations. |
| Main document part discovery | P1 §11.3.10, Main Document Part; P2 §6.5 | The package relationship identifies the main part. Discovery follows the package relationship; relocated main parts are tested. |
| ZIP mapping | P2 §7.3; normative Annex B | `Package::{load,finish}`. Archive reader limits are additional application policy. |
| Core properties | P2 §8; P1 §15.2.12.1 | `ooxml/properties.rs` handles metadata above generic OPC. |
| Signatures | P2 §10 | No signature generation or re-signing implementation. Editing signed content needs an explicit signature-handling policy. |

A relationship target is relative to its source according to OPC rules; a
filename pattern such as `word/header*.xml` is not authoritative story discovery.
Likewise, ID allocation rules differ for relationships, comments, drawings,
bookmarks, and numbering. Shared allocation infrastructure must retain each
domain's scope and constraints.

## WordprocessingML domain map

“Current generation” below describes the typed authoring surface. Template loading
is not a general public typed document reader. Untouched parts are retained as
payload bytes, but edited XML is not guaranteed to preserve every unknown feature.

| Domain / relevant elements | Specification | Current generation | Current implementation |
| --- | --- | --- | --- |
| Main story, `document`, `body` | P1 §17.2; §§17.2.2–17.2.3 | Body content through sections | [document content](../src/document/content.rs), [OOXML compiler](../src/ooxml/mod.rs) |
| Paragraph and run content, `p`, `r`, `t` | P1 §§17.3.1.22, 17.3.2.25, 17.3.3.31 | Text, tabs, line/page/column breaks and rich external hyperlinks; explicit inline variants | `Paragraph`, `TextRun`, `ooxml::wordprocessing::text::text_run` |
| Paragraph/run properties, `pPr`, `rPr` | P1 §§17.3.1.26, 17.3.2.28 | Selected direct formatting; see units below | [format.rs](../src/ooxml/wordprocessing/format.rs) |
| Tables and grid/merges | P1 §17.4; §§17.4.17 (`gridSpan`), 17.4.84 (`vMerge`) | Nested tables, widths, spans/merges, borders, shading; not the whole table vocabulary | [table.rs](../src/document/table.rs), `Compiler::table` |
| Sections, final/non-final `sectPr` | P1 §17.6; §§17.6.17–17.6.18 | Page geometry, columns and selected section settings | `section_properties`, `ooxml::compile` |
| Styles | P1 §17.7; §11.3.12, Style Definitions Part | Selected paragraph/table definitions and references; not complete style semantics | [styles.rs](../src/ooxml/wordprocessing/styles.rs), [domain values](../src/document/values.rs) |
| Fonts | P1 §17.8; §11.3.5, Font Table Part | Font name and selected run properties; template font table retained | [format.rs](../src/ooxml/wordprocessing/format.rs), default assets |
| Numbering | P1 §17.9; §11.3.11, Numbering Definitions Part | Typed definitions/levels/references in both Rust and Python | [styles.rs](../src/ooxml/wordprocessing/styles.rs), native Python conversion |
| Headers and footers | P1 §17.10; §§11.3.6, 11.3.9 | Default/first/even stories; linked template content is appended | [authoring components](../src/authoring/components.rs), `Compiler::header_footer` |
| Footnotes and endnotes | P1 §17.11; §§11.3.4, 11.3.7 | Ordinary definitions/references, rich block bodies, type-distinguished specials and settings references; no layout/numbering calculation | [notes](../src/ooxml/notes.rs), explicit logical stories and relationship-discovered owners; [contract](NOTES.md) |
| Glossary/building blocks | P1 §17.12; §11.3.8 | No typed generation yet | Future story/import support |
| Comments and anchors | P1 §17.13.4; §§17.13.4.3–17.13.4.5; §11.3.2 | Text conveniences or structured paragraphs/tables/images/links on body ranges; explicit author/date/initials; not modern threaded comments | `ResolvedComment`/`ResolvedCommentBody`, shared range codec; extensions also require [MS-DOCX] |
| Bookmarks | P1 §17.13.6; §§17.13.6.1–17.13.6.2 | Paired typed markers, overlap/cross-paragraph spans, document-wide names/IDs and imported collision checks | [range domain](../src/document/range.rs), [range codec](../src/ooxml/ranges.rs); [contract](RANGES.md) |
| Tracked revisions | P1 §17.13.5 | No typed generation yet | Future revision/range model |
| Content controls/custom markup | P1 §17.5; §§17.5.2.29–17.5.2.32 (`sdt` in different contexts) | No typed generation yet | Future context-aware block/inline/control model |
| Hyperlinks, `hyperlink`, `r:id`, `anchor`, `tooltip`, `history` | P1 §17.16.22; normative Annex A.1 `CT_Hyperlink`/`EG_PContent`; P2 §§6.4–6.5 | External URI/IRI and internal bookmark links; rich display runs; no nesting | [domain](../src/document/hyperlink.rs), [codec](../src/ooxml/wordprocessing/paragraph.rs); internal anchors have no external relationship |
| Fields, `fldChar` | P1 §17.16; §§17.16.18–19, 17.16.23 | Simple/complex paragraph-local fields and nested cached results, PAGE/NUMPAGES/REF/TOC helpers and raw instructions; no evaluation | [field codec](../src/ooxml/wordprocessing/fields.rs), [contract](FIELDS.md); imported field edits rejected |
| Mail merge | P1 §17.14 | No typed generation yet | Future fields/data-source integration |
| Document settings | P1 §17.15; §11.3.3 | Defaults plus selected changes, including even/odd headers | `ooxml/defaults.rs`, section/story codecs |

Schema element names alone do not determine their context: for example, `sdt`
and `sectPr` have multiple definitions. Reference the applicable parent/content
model. Do not reuse one serializer solely because two elements have the same
local name.

`CT_Hyperlink` contains `EG_PContent`, so the schema permits nested hyperlinks.
Rejecting nesting is an explicit library limitation, not an XSD assertion.
Supported display children serialize as `w:r` containing text, tabs, breaks or
drawings. Formatting belongs to those runs; no automatic Hyperlink style is
applied. Comment conveniences retain their existing body-only restriction,
including when used on a hyperlink's display runs. Text projection includes link
display text but not target/tooltips or image alt text. Paragraphs cache a private
immutable projection; links derive theirs from children without another cache.

Targets retain Unicode and original percent escapes/query/fragment syntax. The
external relationship is registered against the actual owning part, with IDs and
deduplication scoped to that owner. No target is resolved over the network.
Imported links remain opaque XML rather than a typed import model. Structural
template edits can replace across display runs inside one link, but not across its
boundary, and do not rewrite targets. Rust/Python tests, relocated-owner fixtures,
independent Python readback with target mutation controls, and official-schema
validation cover generation; they do not establish browser/Word navigation safety.

## Formatting and units

| Value / rule | Specification | Current implementation and direction |
| --- | --- | --- |
| Paragraph/run property sequence and choices | P1 §§17.3.1.26, 17.3.2.28 and normative Annex A.1 | `ooxml/schema.rs` supplies selected ordering policy for generated/changed properties. Ordering alone is not cardinality/choice validation. |
| Paragraph spacing and line spacing | P1 §17.3.1.33 (`spacing`) | Before/after points to twips; `LineHeight` supports multiples in 240ths and exact/minimum point heights in twips on paragraphs/styles. Legacy multiplier unchanged; [coverage](FORMATTING.md). |
| Custom tab stops | P1 §§17.3.1.37–38, 17.18.84–85 | Signed point positions to twips; typed alignments/leaders/clears on paragraphs/styles, additive inheritance left to consumer; [coverage](FORMATTING.md). |
| Row height/pagination | P1 §§17.4.6, 17.4.49, 17.4.80 | Exact/minimum point heights to twips; optional header/cannot-split flags; no pagination calculation; [row contract](TABLES.md). |
| Cell margins/text flow | P1 §§17.4.68, 17.4.72, 17.18.93 | Point margins to twips and checked text direction; [cell contract](TABLES.md). |
| Language/complex-script runs | P1 §§17.3.2.2, .7, .17, .20, .26, .30, .39; §17.7.3 | Language/font slots, independent complex-script flags/size and RTL requests; no shaping/effective-style calculation; [coverage](FORMATTING.md). |
| Character styles and run references | P1 §§17.7, 17.3.2.29 | Shared run properties and checked character IDs; forward bases, no effective-style calculation; [contract](STYLES.md). |
| Concrete numbering starts | P1 §§17.9.8, 17.9.26 | Typed defined-level start overrides; abstract start unchanged, no label evaluation; [coverage](NUMBERING.md). |
| Paragraph bidi | P1 §17.3.1.6 | Optional direct/style paragraph direction, distinct from run RTL/cell text flow; no string reversal or computed layout. |
| Section bidi | P1 §17.6.1 | Optional section page direction, preserving omitted/inherited settings; independent of text direction, no page-layout calculation. |
| Conditional table paragraph formatting | P1 §§17.7.6.1, 17.7.6.6 | Typed line heights/tabs and optional bidi in table-style regions, no effective conditional formatting calculation. |
| Conditional table run scripts | P1 §§17.7.6.2, 17.3.2 | Language, literal script fonts and complex-script/RTL requests, no shaping or effective conditional formatting. |
| Document defaults | P1 §§17.7.5.1–5 | Explicit run/paragraph defaults merged into template styles; no effective-style computation. |
| Linked styles | P1 §17.7.4.6 | Explicit opposite-kind paragraph/character links, forward references; no automatic backlinks or application-selection behavior. |
| Section gutter placement | P1 §17.6.16 | Optional rtlGutter request, independently of page/text direction; no gutter-size or binding/layout calculation. |
| Concrete numbering level replacements | P1 §§17.9.7–8, 17.9.26 | Supported NumberingLevel subset under lvlOverride/lvl, combined with distinct startOverride; no abstract mutation or counter evaluation. |
| Table-look activation | P1 §17.4.55 | Explicit first/last row/column and row/column banding requests, consistent val mask; no effective conditional-style computation. |
| Inline table alignment | P1 §17.4.28 | Optional tblPr/jc left/center/right/start/end; no width/grid changes or floating positioning. |
| Legal numbering display | P1 §17.9.4 | Optional isLgl on abstract/replacement levels; explicit false/true, no label calculation or numFmt mutation. |
| Numbering suffix | P1 §17.9.28 | Optional suff on abstract/replacement levels: tab/space/nothing; no separator insertion or tab layout. |
| Numbering level restart | P1 §17.9.10 | Optional lvlRestart on abstract/replacement levels; zero/one-based requests, no effective counter calculation. |
| Text content controls | P1 §17.5.2 and Annex A.1 | Separate inline-run, block, row and cell models with alias, tag, lock and checked optional IDs, plain-text and drop-down modes. |
| Template control editing | P1 §17.5.2 and Annex A.1 | Tag-scoped editing of one eligible imported plain-text control while retaining its run properties. |
| Picture geometry and anchors | P1 §§20.1–20.4 and Annex A.4 | Crop, rotation and flipping, optional page/margin-relative floating placement, none, square, top-and-bottom, tight or through wrapping, edge distances, side selection and placement flags. |
| Inline shapes | P1 §§20.1, 20.3 | Rectangle and ellipse geometry in a standard DrawingML locked canvas, with checked size, RGB fill, outline, plain multiline text and optional uniform text size/bold/italic/Latin typeface/RGB/alignment requests. |
| Standalone inline text boxes | P1 §20.3, CT_GvmlTextShape | Direct locked-canvas text shapes with independent transforms and shared plain-text styling; no geometric fill/outline or rich WordprocessingML text story. |
| Floating shapes and text boxes | P1 §20.4, Annex A.4 `CT_Anchor` | Shared picture-anchor placement around a locked canvas in body paragraphs; no new relationships, computed wrapping, or visual layout claim. |
| Tracked text revisions | P1 §17.13.5 and Annex A.1 | Text-run insertion, deletion and paired moves with author and optional UTC timestamp, preserving imported revision IDs. |
| Track future revisions | P1 §17.15.1 | Optional trackRevisions flag in the document settings. |
| Accept and reject text revisions | P1 §17.13.5 | Scoped template transformation for direct single text-run insertions, deletions and paired moves. |
| Note numbering defaults | P1 §§17.11.4, 17.11.12, 17.11.19–20 | Independent settings-part note placement, formats, starts and restart requests with special-entry preservation. |
| Document page layout defaults | P1 §§17.15.1.50, 17.15.1.57 | Optional mirrorMargins and gutterAtTop requests in the settings part. |
| Header and footer distances | P1 §17.6.11 | Optional independent nonnegative pgMar header and footer distances. |
| Page borders | P1 §17.6.10 | Optional single and dashed physical page edges with checked measurements, RGB color and placement options. |
| Section document grid | P1 §17.6.5 | Optional docGrid mode, linePitch and signed charSpace. |
| Grid snapping | P1 Annex A.1 CT_PPrBase and CT_RPr | Optional snapToGrid for paragraph and run formats. |
| Contextual spacing and mirrored indents | P1 Annex A.1 CT_PPrBase | Optional contextualSpacing and mirrorIndents flags on paragraph properties. |
| Section line numbering | P1 §17.6.8 | Optional lnNumType settings with count, start, distance, restart mode and explicit removal. |
| Paragraph line number suppression | P1 Annex A.1 CT_PPrBase | Optional suppressLineNumbers on paragraph properties. |
| Shared abstract numbering | P1 §§17.9.1–2 | Multiple concrete instances can reference one authored abstract ID with identical ordered levels. Imported IDs are reserved. |
| Numbering label run formatting | P1 Annex A.1 CT_Lvl | Optional direct lvl/rPr on abstract and replacement levels. |
| Numbering level paragraph properties | P1 Annex A.1 CT_Lvl | Optional lvl/pPr on abstract and replacement levels, with indents and tabs in points. |
| Numbering level paragraph style links | P1 Annex A.1 CT_Lvl | Optional lvl/pStyle references a registered paragraph style on abstract and replacement levels. |
| Authored numbering styles | P1 Annex A.1 CT_AbstractNum and CT_Style | Numbering style references an authored concrete definition whose abstract definition contains styleLink. |
| Inherited numbering styles | P1 Annex A.1 CT_AbstractNum | Level-free numStyleLink targets an authored numbering style. |
| Numbering-label alignment | P1 §17.9.7 | Optional lvlJc: left/center/right/start/end on abstract/replacement levels; independent of paragraph alignment. |
| Half-point font sizes | P1 §17.3.2.38 (`sz`); §17.18.42 (`ST_HpsMeasure`) | `format::run` multiplies point sizes by two. Keep the authoring unit separate from the wire unit. |
| Twip measurements | P1 §22.9.2.14 (`ST_TwipsMeasure`); §17.18.81 (`ST_SignedTwipsMeasure`) | Numeric conveniences retain documented units; checked domain value objects centralize conversion. |
| Universal measurements | P1 §22.9.2.15 (`ST_UniversalMeasure`) | Distinguish lexical representations and selected output profile. |
| On/off values and inheritance | P1 §22.9.2.7 (`ST_OnOff`), individual property definitions and §17.7 | Do not conflate absent, true, false, and style toggle semantics. |
| Font/script selection | P1 §17.3.2.26 (`rFonts`); §17.8 | Current font mapping covers part of the vocabulary; broader script/font modeling is unsupported. |

The [current public units](../COMPATIBILITY.md#units-and-defaults) are API choices,
not a requirement that future constructors accept those same units. A numeric
guard in the implementation is not necessarily an OOXML limit: document
application limits separately from schema bounds.

## DrawingML and shared vocabularies

| Feature | Specification | Coverage / boundary |
| --- | --- | --- |
| Drawing framework and themes | P1 §20.1; §14.2.7, Theme Part | Current theme is primarily supplied by the bundled template. |
| Pictures | P1 §20.2; §15.2.14, Image Part | Native PNG/JPEG/GIF/BMP/TIFF header inspection and inline picture emission; Rust/Python defer data, transformation and alt text through the generic resolver; not a general DrawingML model. |
| Wordprocessing drawings | P1 §20.4; §§20.4.2.8 (`inline`), 20.4.2.3 (`anchor`), 20.4.2.5 (`docPr`), 20.4.2.7 (`extent`) | Current inline sizing/accessibility fields; floating anchors and broader drawing properties are not generated. |
| Charts | P1 §21.2; §14.2.1, Chart Part | No typed generation yet; introduces dependent parts and possibly embedded data. |
| Chart drawings and diagrams | P1 §§21.3–21.4 | No typed generation yet; reuse asset and relationship infrastructure. |
| Math | P1 §22.1 | No typed generation yet; requires rich inline/block content integration. |
| Extended/custom properties | P1 §§22.2–22.3; §§15.2.12.2–15.2.12.3 | Not exposed as a complete typed property surface. |
| Legacy VML | P4 §19 | Not generated; imported documents require an explicit preservation policy. |

Image file headers, codec formats, and pixel validation are governed by their
respective image specifications. OOXML defines the embedding/drawing side; citing
DrawingML does not validate a PNG/JPEG/TIFF decoder. Keep those responsibilities
separate.

## XML and compatibility

- [XML 1.0 §2.2][XML10] defines legal XML characters; §2.3 covers names and
  references. `xml::validate_text` owns the shared character check below the Word codecs.
- [Namespaces in XML 1.0][XMLNS], especially §§2 and 6, defines expanded names
  and namespace scoping. Prefix spelling is not element identity. Attribute
  values containing QName/prefix lists need vocabulary-aware handling.
- P3 §7 defines MCE markup, including `Ignorable`, `ProcessContent`,
  `MustUnderstand`, `AlternateContent`, `Choice`, and `Fallback`; §9 defines the
  reference processing model. Unknown-content retention and MCE interpretation
  are different capabilities. The current engine does not implement a complete
  MCE processor.
- `Package` retains untouched part payload bytes; [XML module](../src/xml/mod.rs)
  normalizes parsed/rewritten trees while preserving expanded names and opaque
  prefix bindings. The confirmed shadowing regression is fixed and tested.
  Do not describe edited XML as byte-lossless.

## Documentation and contribution policy

For each new feature or non-obvious serialization rule, document:

1. **Domain meaning and invariants:** how it relates to stories, ranges, styles,
   or resources; what belongs in the pure domain and what is an application effect.
2. **Mapping and source:** owning part(s), relationship types, relevant elements,
   and `ECMA-376 Part N (edition), §clause — title`, linked to the source above.
   Place a short reference near the codec and a fuller entry in this map.
3. **Support profile:** Strict/Transitional applicability, extension namespace and
   revision if needed, and any intentional application-specific behavior.
4. **Operations:** generation, interpretation, safe editing, and opaque retention
   capabilities, with explicit limits. References alone are not conformance proof.
5. **Evidence:** domain tests, exact XML/content-model assertions, cross-part
   validation, independent consumer checks, and preservation fixtures. Record
   producer/version/provenance for fixtures and expected divergences.

Example contributor note (format, not a new implementation):

```text
Feature: Inline picture
Source: ECMA-376 Part 1 (2016), §20.4.2.8 — inline;
        §20.2 — DrawingML - Picture; §15.2.14 — Image Part.
Packaging: ECMA-376 Part 2 (2021), §6.5 — Relationships.
Domain: Drawing::InlinePicture references AssetId; dimensions are Emu values.
Profile: record the emitted namespace family and supported attributes.
Evidence: image relationship resolves from the owning story part, payload and
          extent match expected values, imported opaque content is retained.
```

Optional developer tooling such as an Open XML SDK validator should pin its
version and target profile. It is not a runtime dependency. Do not add the full
standard PDFs to the repository; keep links and precise clause citations. Any
future generated schema tables should record their source edition, generation
command, licensing/provenance, and a reproducibility check.

[ECMA]: https://ecma-international.org/publications-and-standards/standards/ecma-376/
[P1]: https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip
[P2]: https://ecma-international.org/wp-content/uploads/ECMA-376-2_5th_edition_december_2021.zip
[P3]: https://ecma-international.org/wp-content/uploads/ECMA-376-3_5th_edition_december_2015.zip
[P4]: https://ecma-international.org/wp-content/uploads/ECMA-376-4_5th_edition_december_2016.zip
[MS-OI29500]: https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/
[MS-OE376]: https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/
[MS-DOCX]: https://learn.microsoft.com/en-us/openspecs/office_standards/ms-docx/b839fe1f-e1ca-4fa6-8c26-5954d0abbccd
[XML10]: https://www.w3.org/TR/REC-xml/#charsets
[XMLNS]: https://www.w3.org/TR/REC-xml-names/


## Executable validation and current architecture

The [module guide](MODULES.md) records the implemented functional/domain boundary.
[tests/README.md](../tests/README.md) describes the independent validator and its
coverage limits. `tools/validate_ooxml.py` pins official archive digests, resolves
W3C XML attributes locally, validates supported base WordprocessingML/DrawingML
parts, and rejects broken internal relationship targets/uses. Mutation tests
verify that invalid property order and missing targets are detected.

Independent validation found an out-of-order settings insertion and a missing
required `CT_Zoom` percent attribute inherited from the blank template. Both were
corrected; the asset change is documented in [assets](../assets/README.md). This
is evidence for using standards checks alongside consumer readback, not a claim
that all OOXML/Word extensions or layout semantics are now implemented.
