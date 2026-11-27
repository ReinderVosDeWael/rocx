//! Narrow native adapters for the workspace MCP transport.

use super::{ImportedDocument, SectionStoryKind};
use crate::{NoteKind, Result};

/// Open an MCP snapshot whose changed Word parts must fit the native XML depth limit.
pub fn open_with_word_depth_check(
    bytes: &[u8],
    limits: crate::opc::ReadLimits,
) -> Result<ImportedDocument> {
    let mut document = ImportedDocument::open_with_limits(bytes, limits)?;
    document.package.check_word_depth = true;
    Ok(document)
}

/// Count logical story sections without inspecting unrelated layout properties.
pub fn story_section_count(document: &ImportedDocument) -> Result<usize> {
    document.story_section_count()
}

/// Resolve the main document's selected note part, independent of its entries.
/// This only locates the target; the native edit validates the owner and edit.
pub fn note_owner(document: &ImportedDocument, kind: NoteKind) -> Result<Option<String>> {
    let relation = match kind {
        NoteKind::Footnote => "footnotes",
        NoteKind::Endnote => "endnotes",
    };
    Ok(document
        .package
        .related(&document.main, relation)?
        .map(|(_, part)| part))
}

/// List native link candidates for the selected physical story kind.
pub fn linkable_story_parts(
    document: &ImportedDocument,
    kind: SectionStoryKind,
) -> Result<Vec<String>> {
    document.linkable_story_parts(kind, false)
}

/// List native link candidates, including parts already used by a section.
pub fn all_linkable_story_parts(
    document: &ImportedDocument,
    kind: SectionStoryKind,
) -> Result<Vec<String>> {
    document.linkable_story_parts(kind, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Component, ResolvedBlock, Table, TableCell, TableRow,
        editor::{BodyBlock, EditBatch, TableInsertion},
    };

    #[test]
    fn word_depth_guard_is_opt_in_for_native_table_previews() {
        let bytes = include_bytes!("../../tests/fixtures/imported-editor.docx");
        let mut nested = None;
        for _ in 0..84 {
            let blocks = nested
                .take()
                .map(|table| ResolvedBlock::Table(table).into())
                .into_iter()
                .collect();
            nested = futures_executor::block_on(
                Table::new(vec![
                    TableRow::new(vec![TableCell::new(blocks).into()]).into(),
                ])
                .resolve(),
            )
            .unwrap();
        }
        let table = nested.unwrap();

        let preview = |document: &ImportedDocument| {
            let target = document
                .body()
                .iter()
                .find_map(|block| match block {
                    BodyBlock::Paragraph(p) => Some(p.anchor.clone()),
                    _ => None,
                })
                .unwrap();
            document.preview_edits(EditBatch {
                tables: &[TableInsertion {
                    target,
                    table: table.clone(),
                }],
                ..EditBatch::default()
            })
        };
        let native = ImportedDocument::open(bytes).unwrap();
        let output = preview(&native).unwrap().to_docx().unwrap();
        let open_error = ImportedDocument::open(&output).unwrap_err();
        assert_eq!(open_error.to_string(), "XML depth exceeds read limit (256)");

        let mcp = open_with_word_depth_check(bytes, Default::default()).unwrap();
        let error = preview(&mcp).unwrap_err();
        assert_eq!(error.path(), Some("word/document.xml"));
        assert_eq!(error.rule(), open_error.rule());
    }
}
