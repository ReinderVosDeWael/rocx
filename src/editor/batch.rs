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
    pub blocks: &'a [BlockEdit],
    pub inline_edits: &'a [InlineEdit],
    pub structural: &'a [TableStructureEdit],
    pub styles: &'a [StyleEdit],
    pub resources: &'a [ResourceEdit],
    pub controls: &'a [ControlEdit],
    pub fields: &'a [FieldEdit],
    pub paired_ranges: &'a [PairedRangeEdit],
}

pub(super) type Site = (usize, usize);

/// Planned against the unmodified tree; shared by field spans and paired
/// markers. Ordinary actions still pass their original boundary checks.
#[derive(Clone, Default)]
pub(super) struct StructuredSlot {
    pub before: Vec<Node>,
    pub after: Vec<Node>,
    pub replacement: Option<Vec<Node>>,
    pub field: bool,
}
pub(super) type Slots = BTreeMap<Site, StructuredSlot>;

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
