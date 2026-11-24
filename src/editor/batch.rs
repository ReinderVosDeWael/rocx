use super::*;

/// An atomic batch against the document snapshot that issued its anchors.
/// Omitted slices are empty. Story and revision operations have separately
/// scoped preview methods.
#[derive(Default)]
pub struct EditBatch<'a> {
    pub alignments: &'a [ParagraphAlignmentEdit],
    pub tables: &'a [TableInsertion],
    pub header_images: &'a [HeaderImageEdit],
    pub run_text: &'a [RunTextEdit],
    pub paragraphs: &'a [ParagraphInsertion],
    pub cells: &'a [CellAlignmentEdit],
    pub run_removals: &'a [InlineAnchor],
    pub ranges: &'a [TextRangeEdit],
    pub text_insertions: &'a [TextInsertion],
    pub blocks: &'a [BlockEdit],
    pub inline_edits: &'a [InlineEdit],
    pub structural: &'a [TableStructureEdit],
    pub styles: &'a [StyleEdit],
    pub resources: &'a [ResourceEdit],
    pub controls: &'a [ControlEdit],
    pub wrappers: &'a [WrapperEdit],
}

// Compatibility entry points all take the original seven mixed-batch slices
// followed by their extension slice.
macro_rules! extended_preview {
    ($name:ident, $field:ident, $edit:ty, $doc:literal) => {
        #[doc = $doc]
        #[allow(clippy::too_many_arguments)]
        pub fn $name(
            &self,
            alignments: &[ParagraphAlignmentEdit],
            tables: &[TableInsertion],
            header_images: &[HeaderImageEdit],
            run_text: &[RunTextEdit],
            paragraphs: &[ParagraphInsertion],
            cells: &[CellAlignmentEdit],
            run_removals: &[InlineAnchor],
            $field: &[$edit],
        ) -> Result<EditPreview> {
            self.preview_edits(EditBatch {
                alignments,
                tables,
                header_images,
                run_text,
                paragraphs,
                cells,
                run_removals,
                $field,
                ..EditBatch::default()
            })
        }
    };
}

impl ImportedDocument {
    /// Stage a standalone main-story operation against a private package.
    pub(super) fn stage_body(
        &self,
        stage: impl FnOnce(&mut Node) -> Result<bool>,
    ) -> Result<EditPreview> {
        let mut package = self.package.clone();
        let mut root = package.word(&self.main)?;
        if stage(root.child_mut("w:body").ok_or_else(missing_body)?)? {
            package.put_word(&self.main, root)?;
        }
        Ok(EditPreview {
            output: PackagePlan::new(package)?,
            block_changes: vec![],
            inline_changes: vec![],
            changes: vec![],
            insertions: vec![],
            paragraph_insertions: vec![],
            header_changes: vec![],
            run_text_changes: vec![],
            run_removals: vec![],
            cell_alignment_changes: vec![],
        })
    }

    /// Preview a batch consisting only of block operations.
    pub fn preview_blocks(&self, blocks: &[BlockEdit]) -> Result<EditPreview> {
        self.preview_edits(EditBatch {
            blocks,
            ..EditBatch::default()
        })
    }

    extended_preview!(
        preview_with_blocks,
        blocks,
        BlockEdit,
        "Stage paragraph and section edits in a mixed batch."
    );
    extended_preview!(
        preview_with_inlines,
        inline_edits,
        InlineEdit,
        "Stage whole-inline edits in a mixed batch."
    );
    extended_preview!(
        preview_with_table_structure,
        structural,
        TableStructureEdit,
        "Stage structural table edits in a mixed batch."
    );
    extended_preview!(
        preview_with_styles,
        styles,
        StyleEdit,
        "Stage style definitions and content edits together."
    );
    extended_preview!(
        preview_with_resources,
        resources,
        ResourceEdit,
        "Stage document-wide properties in a mixed batch."
    );
    extended_preview!(
        preview_with_controls,
        controls,
        ControlEdit,
        "Stage content-control edits in a mixed batch."
    );
}
