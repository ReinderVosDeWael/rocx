# Python API

Type hints show ordinary inputs. For factories, awaitables and property references, see [deferred values](../contracts/DEFERRED-FIELDS.md).

::: rocx.declarative

::: rocx.options

## Imported DOCX editor

`rocx.editor.ImportedDocument.open(data)` opens an immutable snapshot. For
example, on a fixture whose first body block is a plain paragraph starting
with “Draft”, replace that word and set the package's existing core title
in one transaction:

```python
from rocx.editor import ImportedDocument, ResourceEdit, TextInsertion, TextRangeEdit

doc = ImportedDocument.open(data)
first = doc.body[0]
preview = doc.preview(
    [],
    text_ranges=[TextRangeEdit(first.anchor, 0, 5, "Final")],
    text_insertions=[TextInsertion(first.anchor, 5, " copy")],
    resources=[ResourceEdit.core_title("Annual report")],
)
copy_bytes = preview.to_docx()
```

`[0, 5)` counts Unicode scalar values in the original paragraph. The
replacement keeps the first selected character's formatting. The insertion at
original position 5 follows the replacement, producing “Final copy”. Reopen
`copy_bytes` to obtain anchors for the new document; `doc` and its anchors
remain usable. See the [cross-binding editor reference](../compatibility.md#resolution-editing-and-output)
for the other batch fields and separate story and revision calls.
