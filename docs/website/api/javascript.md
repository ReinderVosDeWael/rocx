# JavaScript API reference

The `rocx-docx` ESM package exposes one function in Node.js and Chromium.
See [installation and supported environments](../javascript.md).

```typescript
generateDocx(
  declaration: DocumentDeclaration,
  options?: GenerateOptions,
): Promise<Uint8Array<ArrayBuffer>>
```

| Input | Shape |
| --- | --- |
| `DocumentDeclaration` | `blocks: readonly Block[]`; one section with default page layout |
| Paragraph block | `kind: "paragraph"`, `runs: readonly Run[]` |
| Table block | `kind: "table"`, `rows: readonly (readonly string[])[]`; a nonempty rectangular grid |
| Text run | `kind: "text"`, `text: string`, optional `bold` and `italic` booleans |
| Image run | `kind: "image"`, `bytes: Uint8Array` containing an encoded image, optional `altText: string` |
| `GenerateOptions` | Optional `timestamp`: UTC ISO time with seconds and optional milliseconds; defaults to the current time |

The result is a complete DOCX file in memory. Write the bytes to disk in Node
or download them as a Blob in a browser. Invalid declarations or rendering
errors reject the Promise. When available, `error.path` locates the problem
in the declaration, and `error.rule` gives a stable rocx diagnostic code.
These optional properties are described by the TypeScript `GenerateDocxError`
interface. The function does not fetch source images or upload documents.
It does not expose templates or the editor.

## Shared report declaration

This is the same declaration used by the Node and Chromium examples:

```javascript
{{snippet:examples/javascript/author.mjs:declaration}}
```

See the [runnable Node and browser examples](../javascript.md#runnable-report)
and the [complete TypeScript declarations](../javascript.md#typescript-declarations).
