# Documentation audit

The docs explain what rocx writes and what it leaves to the document viewer.
The biggest gaps are getting started without a source checkout and finding
one complete template-editing example. The biggest opportunity to cut text
is to stop repeating feature inventories across guides and runbooks.

This review compares repository Markdown only. It does not verify runtime
behavior, registry availability, or the generated website.

## Fix before release

| Where | Recommended change |
| --- | --- |
| [Getting started](website/getting-started.md) | Lead with installing rocx into an existing Cargo project or Python environment once registry availability is confirmed. Give each language one complete create-and-save example, including how to run the Rust async entry point. Distinguish a supported Python wheel (no Rust toolchain) from a source build (Rust required). Keep the clone-and-run instructions under “Run the repository examples.” |
| [Recipes](website/recipes.md) and [Compatibility](../COMPATIBILITY.md#resolution-editing-and-output) | Add one paired template recipe: start with an unsigned `.docx`, replace a placeholder split across runs, and save to a new path. Put supported story scope, protected content, and what is preserved versus reserialized next to the recipe. Link to the separate [revision](REVISIONS.md) and [content-control](CONTENT_CONTROLS.md) editing contracts rather than repeating them. |
| [Migration](MIGRATION.md) | The page is titled “Migrating to 0.2” but includes Rust `Metadata` literal changes for features listed in the [0.3.0 changelog](../CHANGELOG.md). Separate the legacy rename/redesign from the `0.2 → 0.3` upgrade. Check the release baseline before assigning the `ShapeProperties` literal advice to either section. |
| [OOXML map](OOXML.md#packaging-and-part-identity) | The signatures row says an explicit policy is needed, but the end of the same page already documents rejection. Put the rejection in the row and link to Compatibility for the user-facing policy. In the [XML and compatibility section](OOXML.md#xml-and-compatibility), link to the exact `MustUnderstand` guard and say explicitly that the editor does not select `AlternateContent` branches. |
| [Website home](website/index.md) and [field recipe](website/recipes.md#fields-with-cached-results) | Replace “calculated by the document viewer” with “a compatible viewer may update fields.” The [field contract](FIELDS.md) already explains that the viewer can ignore update requests. |
| [Release checklist](RELEASING.md#release-checklist) and [verification](../tests/README.md#independent-official-schema-checks) | The release checklist requires schemas, but the main schema-test command allows skips. Show the copyable release command with `ROCX_REQUIRE_SCHEMAS=1`. Add the Python runtime version to the checklist’s version-update step, and make a successful manual hosted preflight an explicit step before pushing the publishing tag. |

## Shorten and consolidate

- **[Python bindings](../PYTHON_BINDINGS.md):** Replace the long authoring tour
  with links to feature contracts and the generated Python reference. Keep
  Python-specific guidance: async ownership, errors, output methods and interop.
  Explain that the extension is required for native bytes/save, while
  `python-docx` is optional; the opening currently calls the extension optional
  without saying what works without it.
- **[Formatting](FORMATTING.md):** Add a short topic index for text, page layout
  and drawings. Promote charts and equations out of the page-direction heading
  hierarchy. Keep units, omission versus explicit false, supported contexts and
  examples; move test-by-test evidence out of user-facing sections.
- **[Visual catalog](../examples/ALL_FEATURES.md):** Fold sample descriptions
  into the existing topic table. Repair the sentence split between “a single”
  and “abstract definition,” and move the section and picture samples out of
  the maintenance section. Keep the exact-byte requirement and viewer caveats.
- **[Verification](../tests/README.md):** Keep the runnable commands, setup,
  troubleshooting and what each check proves. Replace the per-file test ledger
  and trailing feature updates with a brief index by verification goal. Link
  to the catalog guide for its exact-byte command instead of maintaining two
  copies.
- **[Plan](PLAN.md):** Keep the active release checklist, pending decisions and
  deferred backlog at the top. Archive or remove superseded handoffs, repeated
  pass counts and scratch paths. The closing “First assignment for the next
  agent” still directs new feature work despite the freeze at the top. Move it
  into dated history so it cannot be mistaken for current authorization.

Keep Compatibility as the short support summary, feature guides as the
authoring reference, and the OOXML map as the contributor specification
reference. Prefer links over repeating a rule in all three.
