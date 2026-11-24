//! Read and edit an existing Transitional DOCX without rebuilding it.
//!
//! Inspect direct body blocks and section headers and footers, patch body paragraph
//! alignment, insert declarative tables, and append an image to an existing header.
//! The package, not the typed inspection view, is the source of output.

use crate::ooxml::wordprocessing::run_child::RunChild;
use crate::ooxml::{WordParts, open_word_source};
use crate::opc::{Package, PackagePlan, ReadLimits};
use crate::xml::{Content, Node};
use crate::{
    Alignment, BreakType, Diagnostic, Error, ResolvedParagraph, ResolvedTable, Result, Rule,
    SectionType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

mod batch;
pub use batch::EditBatch;
mod blocks;
pub use blocks::BlockEdit;
mod controls;
mod entries;
mod header;
mod insertion;
pub(crate) use insertion::BoundaryState;
mod ranges;
pub use ranges::TextRangeEdit;
mod numbering;
mod resource_fields;
pub use numbering::{ImportedNumberingInstance, NumberingEdit};
mod resources;
mod revisions;
pub use revisions::TrackedText;
mod style_edits;
mod tables;
mod text;
pub use controls::{ControlAnchor, ControlContext, ControlEdit, ImportedControl};
pub use entries::{EntryKind, ImportedEntry};
pub use header::{
    HeaderAnchor, HeaderEditMode, HeaderImageChange, HeaderImageEdit, HeaderImagePreview,
    ImportedHeader, ImportedHeaderBlock, ImportedStory, ImportedStoryBlock, SectionStoryKind,
};
pub use resource_fields::{AbstractStart, ResourceEdit};
pub use resources::{ImportedStyle, ImportedStyles, StyleKind};
pub use style_edits::{StyleAnchor, StyleEdit};
pub use tables::{CellAlignmentChange, CellAlignmentEdit, TableOperation, TableStructureEdit};
pub use text::{InlineAction, InlineEdit, RunRemovalChange, RunTextChange, RunTextEdit};

/// A target valid only for the snapshot that issued it.
#[derive(Clone, Debug)]
pub struct ParagraphAnchor {
    snapshot: Arc<()>,
    body_index: usize,
}

/// Direct body children in document order, excluding the final section properties.
#[derive(Debug)]
#[non_exhaustive]
pub enum BodyBlock {
    Paragraph(ImportedParagraph),
    Table(ImportedTable),
    /// Direct block content control; its contents remain opaque.
    ContentControl(ImportedContentControl),
    /// An unsupported element, identified by expanded XML name.
    Opaque(String),
}

/// Read-only direct body control properties. Values are source XML strings;
/// absent elements or missing `w:val` attributes produce `None`.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedContentControl {
    pub tag: Option<String>,
    pub alias: Option<String>,
    pub lock: Option<String>,
}

fn read_content_control(node: &Node) -> ImportedContentControl {
    let props = node.child("w:sdtPr");
    let value = |name| {
        props
            .and_then(|p| p.child(name))
            .and_then(|p| p.get("w:val"))
            .map(str::to_owned)
    };
    ImportedContentControl {
        tag: value("w:tag"),
        alias: value("w:alias"),
        lock: value("w:lock"),
    }
}

/// Snapshot-scoped identity for a direct body table.
#[derive(Clone, Debug)]
pub struct TableAnchor {
    _snapshot: Arc<()>,
    _body_index: usize,
}

/// Physical direct rows and cells; grid spans and merged cells are not expanded.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedTable {
    pub anchor: TableAnchor,
    pub children: Vec<ImportedTableChild>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ImportedTableChild {
    Row(ImportedTableRow),
    Opaque(String),
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedTableRow {
    pub children: Vec<ImportedRowChild>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ImportedRowChild {
    Cell(ImportedTableCell),
    Opaque(String),
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedTableCell {
    pub blocks: Vec<ImportedCellBlock>,
    inline_structure: Vec<Option<Vec<ImportedInlineNode>>>,
}

impl ImportedTableCell {
    /// Ordered runs, hyperlinks, and opaque inline nodes in a direct paragraph.
    ///
    /// `block_index` indexes `blocks`, including opaque blocks. Returns `None`
    /// for opaque or absent blocks, and an empty slice for an empty paragraph.
    pub fn inline_structure(&self, block_index: usize) -> Option<&[ImportedInlineNode]> {
        self.inline_structure.get(block_index)?.as_deref()
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ImportedCellBlock {
    /// Text projection with the same inline rules as body paragraphs.
    Paragraph(Vec<ImportedInline>),
    /// Nested tables and unsupported blocks remain opaque in this slice.
    Opaque(String),
}

fn read_table(node: &Node, snapshot: &Arc<()>, body_index: usize) -> ImportedTable {
    ImportedTable {
        anchor: TableAnchor {
            _snapshot: snapshot.clone(),
            _body_index: body_index,
        },
        children: node
            .elements()
            .filter(|n| !matches!(n.name.as_str(), "w:tblPr" | "w:tblGrid"))
            .enumerate()
            .map(|(row_index, n)| {
                if n.name != "w:tr" {
                    return ImportedTableChild::Opaque(n.name.to_string());
                }
                ImportedTableChild::Row(ImportedTableRow {
                    children: n
                        .elements()
                        .filter(|n| n.name != "w:trPr")
                        .enumerate()
                        .map(|(cell_index, n)| {
                            if n.name != "w:tc" {
                                return ImportedRowChild::Opaque(n.name.to_string());
                            }
                            let (blocks, inline_structure) = n
                                .elements()
                                .filter(|n| n.name != "w:tcPr")
                                .enumerate()
                                .map(|(block_index, n)| {
                                    if n.name != "w:p" {
                                        return (
                                            ImportedCellBlock::Opaque(n.name.to_string()),
                                            None,
                                        );
                                    }
                                    let (items, structure) = read_paragraph_inlines(
                                        n,
                                        snapshot,
                                        InlineLocation::TableCell {
                                            body_index,
                                            row_index,
                                            cell_index,
                                            block_index,
                                        },
                                    );
                                    (ImportedCellBlock::Paragraph(items), Some(structure))
                                })
                                .unzip();
                            ImportedRowChild::Cell(ImportedTableCell {
                                blocks,
                                inline_structure,
                            })
                        })
                        .collect(),
                })
            })
            .collect(),
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedParagraph {
    pub anchor: ParagraphAnchor,
    /// Text from ordinary runs and hyperlinks; not a rendered-text view.
    pub text: String,
    /// Recognized direct alignment, not inherited/effective alignment.
    pub alignment: Option<Alignment>,
    /// The source value, including an explicit alignment outside rocx's model.
    pub alignment_xml: Option<String>,
    /// Some inline content was omitted from the text projection.
    pub has_opaque_content: bool,
    /// Ordered content from direct runs and hyperlink runs. This view flattens
    /// run and hyperlink boundaries and is not an inline edit target.
    pub inline_items: Vec<ImportedInline>,
    /// Ordered direct inline nodes, retaining run and hyperlink boundaries.
    pub inline_structure: Vec<ImportedInlineNode>,
}

/// Read-only projected run content and opaque element names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportedInline {
    Text(String),
    Tab,
    /// `Line` denotes the `w:cr` form; an untyped `w:br` is `TextWrapping`.
    Break(BreakType),
    /// Source element name, with unsupported content left in the package.
    Opaque(String),
}

/// Address of a run, hyperlink, or revision in a direct body, body table-cell,
/// header, footer, or entry paragraph. Supported direct body revision wrappers
/// may be reviewed; anchors inside them are read-only.
///
/// Resolve it with [`ImportedDocument::inline_at`]. Reopening even identical
/// bytes creates a different snapshot. Within a snapshot, section aliases of
/// one physical story part yield equal anchors for the same inline node.
#[derive(Clone, Debug)]
pub struct InlineAnchor {
    snapshot: Arc<()>,
    location: InlineLocation,
    path: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InlineLocation {
    Entry {
        index: usize,
        block_index: usize,
    },
    Body(usize),
    TableCell {
        body_index: usize,
        row_index: usize,
        cell_index: usize,
        block_index: usize,
    },
    Story {
        part: Arc<str>,
        block_index: usize,
    },
}

impl PartialEq for InlineAnchor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.snapshot, &other.snapshot)
            && self.location == other.location
            && self.path == other.path
    }
}

impl Eq for InlineAnchor {}

/// One node in a paragraph's structured inline view.
#[derive(Debug)]
#[non_exhaustive]
pub enum ImportedInlineNode {
    Run(ImportedRun),
    Hyperlink(ImportedHyperlink),
    Revision(ImportedRevision),
    /// Unsupported element, retained in place without inspecting its contents.
    Opaque(String),
}

/// Inline insertion, deletion or move wrapper with literal source metadata.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedRevision {
    pub anchor: InlineAnchor,
    /// `insertion`, `deletion`, `moveSource` or `moveDestination`.
    pub kind: &'static str,
    pub id: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub children: Vec<ImportedInlineNode>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedRun {
    pub anchor: InlineAnchor,
    /// The run's text, tabs, breaks, and opaque children; excludes run properties.
    pub items: Vec<ImportedInline>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedHyperlink {
    pub anchor: InlineAnchor,
    /// Explicit `r:id`, local to the part containing the paragraph (main
    /// document, header, or footer); not a resolved URL.
    pub relationship_id: Option<String>,
    /// Explicit `w:anchor`; the bookmark reference is not resolved.
    pub bookmark: Option<String>,
    pub children: Vec<ImportedInlineNode>,
}

/// A direct body section identified by its half-open range in `body()`.
/// This is an inspection value, not an edit target.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedSection {
    pub body_start: usize,
    pub body_end: usize,
    /// Recognized explicit section type. No default is inferred.
    pub section_type: Option<SectionType>,
    /// The explicit source value, including unknown values.
    pub section_type_xml: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ParagraphAlignmentEdit {
    pub target: ParagraphAnchor,
    pub alignment: Alignment,
}

/// Insert a resolved declarative table immediately after a direct body paragraph.
#[derive(Clone, Debug)]
pub struct TableInsertion {
    pub target: ParagraphAnchor,
    pub table: ResolvedTable,
}

/// Insert a resolved declarative paragraph immediately after a direct body paragraph.
#[derive(Clone, Debug)]
pub struct ParagraphInsertion {
    pub target: ParagraphAnchor,
    pub paragraph: ResolvedParagraph,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct TableInsertionChange {
    /// Zero-based index of the anchor in the original body.
    pub after_body_index: usize,
    pub rows: usize,
    pub columns: usize,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct AlignmentChange {
    /// Zero-based index in `ImportedDocument::body()`.
    pub body_index: usize,
    /// The previous direct XML value, or `None` if there was no direct value.
    pub before: Option<String>,
    pub after: Alignment,
}

/// A validated edit batch and its independently writable output.
#[derive(Debug)]
pub struct EditPreview {
    block_changes: Vec<usize>,
    run_text_changes: Vec<RunTextChange>,
    inline_changes: Vec<(usize, usize)>,
    run_removals: Vec<RunRemovalChange>,
    cell_alignment_changes: Vec<CellAlignmentChange>,
    changes: Vec<AlignmentChange>,
    insertions: Vec<TableInsertionChange>,
    paragraph_insertions: Vec<usize>,
    header_changes: Vec<HeaderImageChange>,
    output: PackagePlan,
}

impl EditPreview {
    /// Start indices of applied block operations in original body order.
    pub fn block_changes(&self) -> &[usize] {
        &self.block_changes
    }
    /// Changed physical cells in original document order.
    pub fn cell_alignment_changes(&self) -> &[CellAlignmentChange] {
        &self.cell_alignment_changes
    }
    /// Changed (body index, direct inline index) pairs in original document order.
    pub fn inline_changes(&self) -> &[(usize, usize)] {
        &self.inline_changes
    }
    /// Removed direct body runs, in original document order.
    pub fn run_removals(&self) -> &[RunRemovalChange] {
        &self.run_removals
    }
    /// Changed direct body runs, in original document order.
    pub fn run_text_changes(&self) -> &[RunTextChange] {
        &self.run_text_changes
    }
    /// Actual changes in document order. No-op edits are omitted.
    pub fn changes(&self) -> &[AlignmentChange] {
        &self.changes
    }

    /// Table insertions in source document order.
    pub fn insertions(&self) -> &[TableInsertionChange] {
        &self.insertions
    }

    /// Original body indices after which paragraphs were inserted.
    pub fn paragraph_insertions(&self) -> &[usize] {
        &self.paragraph_insertions
    }

    /// Header image changes in target section order.
    pub fn header_changes(&self) -> &[HeaderImageChange] {
        &self.header_changes
    }

    pub fn to_docx(&self) -> Result<Vec<u8>> {
        self.output.to_bytes()
    }
}

/// An immutable source snapshot. Opening it does not rewrite the package.
#[derive(Debug)]
pub struct ImportedDocument {
    package: Package,
    main: String,
    body: Vec<BodyBlock>,
    controls: Vec<ImportedControl>,
    snapshot: Arc<()>,
    default_header_inspection: OnceLock<header::StoryInspection>,
    story_inspection: OnceLock<header::StoryInspection>,
    entries: OnceLock<Vec<ImportedEntry>>,
}

impl ImportedDocument {
    pub fn open(bytes: &[u8]) -> Result<Self> {
        Self::open_with_limits(bytes, ReadLimits::default())
    }

    pub fn open_with_limits(bytes: &[u8], limits: ReadLimits) -> Result<Self> {
        let (package, main, root) = open_word_source(bytes, limits, false)?;
        let snapshot = Arc::new(());
        let body = root
            .child("w:body")
            .ok_or_else(missing_body)?
            .elements()
            .filter(|node| node.name != "w:sectPr")
            .enumerate()
            .map(|(body_index, node)| match node.name.as_str() {
                "w:p" => {
                    let anchor = ParagraphAnchor {
                        snapshot: snapshot.clone(),
                        body_index,
                    };
                    let (inline_items, inline_structure) =
                        read_paragraph_inlines(node, &snapshot, InlineLocation::Body(body_index));
                    let mut text = String::new();
                    let mut has_opaque_content = false;
                    for item in &inline_items {
                        match item {
                            ImportedInline::Text(value) => text.push_str(value),
                            ImportedInline::Tab => text.push('\t'),
                            ImportedInline::Break(BreakType::Line | BreakType::TextWrapping) => {
                                text.push('\n')
                            }
                            _ => has_opaque_content = true,
                        }
                    }
                    let alignment_xml = node
                        .child("w:pPr")
                        .and_then(|props| props.child("w:jc"))
                        .and_then(|jc| jc.get("w:val"));
                    BodyBlock::Paragraph(ImportedParagraph {
                        anchor,
                        text,
                        alignment: alignment_xml.and_then(read_alignment),
                        alignment_xml: alignment_xml.map(str::to_owned),
                        has_opaque_content,
                        inline_items,
                        inline_structure,
                    })
                }
                "w:tbl" => BodyBlock::Table(read_table(node, &snapshot, body_index)),
                "w:sdt" => BodyBlock::ContentControl(read_content_control(node)),
                name => BodyBlock::Opaque(name.to_owned()),
            })
            .collect();
        let controls = controls::inspect(root.child("w:body").ok_or_else(missing_body)?, &snapshot);
        Ok(Self {
            controls,
            package,
            main,
            body,
            snapshot,
            default_header_inspection: OnceLock::new(),
            story_inspection: OnceLock::new(),
            entries: OnceLock::new(),
        })
    }

    pub fn body(&self) -> &[BodyBlock] {
        &self.body
    }

    /// Read a body, body table-cell, header, or footer run or hyperlink from this snapshot.
    /// An address from another snapshot is rejected, even at the same position.
    pub fn inline_at(&self, target: &InlineAnchor) -> Result<&ImportedInlineNode> {
        self.validate_snapshot(&target.snapshot)?;
        let invalid = || unsupported("inline target not found");
        let mut nodes = match &target.location {
            InlineLocation::Entry { index, block_index } => self
                .entry_stories()?
                .get(*index)
                .and_then(|entry| entry.inline_structure(*block_index))
                .ok_or_else(invalid)?,
            InlineLocation::Body(index) => {
                let Some(BodyBlock::Paragraph(paragraph)) = self.body.get(*index) else {
                    return Err(invalid());
                };
                paragraph.inline_structure.as_slice()
            }
            InlineLocation::TableCell {
                body_index,
                row_index,
                cell_index,
                block_index,
            } => {
                let Some(BodyBlock::Table(table)) = self.body.get(*body_index) else {
                    return Err(invalid());
                };
                let Some(ImportedTableChild::Row(row)) = table.children.get(*row_index) else {
                    return Err(invalid());
                };
                let Some(ImportedRowChild::Cell(cell)) = row.children.get(*cell_index) else {
                    return Err(invalid());
                };
                cell.inline_structure(*block_index).ok_or_else(invalid)?
            }
            InlineLocation::Story { part, block_index } => [
                self.story_inspection.get(),
                self.default_header_inspection.get(),
            ]
            .into_iter()
            .flatten()
            .find_map(|inspection| inspection.inline_structure(part, *block_index))
            .ok_or_else(invalid)?,
        };
        let (last, parents) = target.path.split_last().ok_or_else(invalid)?;
        for &index in parents {
            nodes = match nodes.get(index) {
                Some(ImportedInlineNode::Hyperlink(link)) => &link.children,
                Some(ImportedInlineNode::Revision(revision)) => &revision.children,
                _ => return Err(invalid()),
            };
        }
        nodes.get(*last).ok_or_else(invalid)
    }

    /// Validate and stage the whole batch; neither the snapshot nor the source is modified.
    pub fn preview(&self, edits: &[ParagraphAlignmentEdit]) -> Result<EditPreview> {
        self.preview_batch(edits, &[])
    }

    /// All targets refer to the original body. A paragraph may receive one
    /// alignment edit and one table insertion in the same batch.
    pub fn preview_batch(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
    ) -> Result<EditPreview> {
        self.preview_transaction(edits, tables, &[])
    }

    /// Stage body and header edits in one independently writable preview.
    /// All targets refer to this original snapshot. One alignment and one table
    /// insertion may target the same paragraph; each section header slot may
    /// have one edit. A shared edit excludes other edits of the same physical source part.
    /// Local forks use the original part even when another section forks it in
    /// the same batch. No edits are published if any operation fails.
    pub fn preview_transaction(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
    ) -> Result<EditPreview> {
        self.preview_with_text(edits, tables, header_images, &[])
    }

    /// Stage text replacements together with paragraph, table and header edits.
    /// Text targets must be direct, single-text body runs outside protected ranges.
    pub fn preview_with_text(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
    ) -> Result<EditPreview> {
        self.preview_with_run_removals(edits, tables, header_images, run_text, &[])
    }

    /// Stage removal of direct plain-text body runs together with other edits.
    pub fn preview_with_run_removals(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
        run_removals: &[InlineAnchor],
    ) -> Result<EditPreview> {
        self.preview_mixed(
            edits,
            tables,
            header_images,
            run_text,
            &[],
            &[],
            run_removals,
        )
    }

    /// Stage paragraph insertions alongside text, alignment, table and header edits.
    pub fn preview_with_paragraphs(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
        paragraphs: &[ParagraphInsertion],
    ) -> Result<EditPreview> {
        self.preview_mixed(edits, tables, header_images, run_text, paragraphs, &[], &[])
    }

    /// Stage cell alignment alongside text, alignment, table and header edits.
    pub fn preview_with_cells(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
        cells: &[CellAlignmentEdit],
    ) -> Result<EditPreview> {
        self.preview_mixed(edits, tables, header_images, run_text, &[], cells, &[])
    }

    /// Stage all edit kinds atomically against their original snapshot addresses.
    #[allow(clippy::too_many_arguments)]
    pub fn preview_mixed(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
        paragraphs: &[ParagraphInsertion],
        cells: &[CellAlignmentEdit],
        run_removals: &[InlineAnchor],
    ) -> Result<EditPreview> {
        self.preview_with_ranges(
            edits,
            tables,
            header_images,
            run_text,
            paragraphs,
            cells,
            run_removals,
            &[],
        )
    }

    /// Stage a mixed transaction, including scalar ranges in plain body paragraphs.
    #[allow(clippy::too_many_arguments)]
    pub fn preview_with_ranges(
        &self,
        edits: &[ParagraphAlignmentEdit],
        tables: &[TableInsertion],
        header_images: &[HeaderImageEdit],
        run_text: &[RunTextEdit],
        paragraphs: &[ParagraphInsertion],
        cells: &[CellAlignmentEdit],
        run_removals: &[InlineAnchor],
        ranges: &[TextRangeEdit],
    ) -> Result<EditPreview> {
        self.preview_edits(EditBatch {
            alignments: edits,
            tables,
            header_images,
            run_text,
            paragraphs,
            cells,
            run_removals,
            ranges,
            ..EditBatch::default()
        })
    }

    /// Stage a transaction against this snapshot. Every address and reported
    /// change uses its original coordinates, even when siblings are spliced.
    pub fn preview_edits(&self, batch: EditBatch<'_>) -> Result<EditPreview> {
        let EditBatch {
            alignments: edits,
            tables,
            header_images,
            run_text,
            paragraphs,
            cells,
            run_removals,
            ranges,
            blocks,
            inline_edits,
            structural,
            styles,
            resources,
            controls,
        } = batch;
        for snapshot in edits
            .iter()
            .map(|edit| &edit.target.snapshot)
            .chain(tables.iter().map(|insertion| &insertion.target.snapshot))
            .chain(header_images.iter().map(HeaderImageEdit::target_snapshot))
            .chain(run_text.iter().map(|edit| &edit.target.snapshot))
            .chain(paragraphs.iter().map(|edit| &edit.target.snapshot))
            .chain(cells.iter().map(|edit| &edit.target._snapshot))
            .chain(ranges.iter().map(|r| &r.target.snapshot))
            .chain(inline_edits.iter().map(|edit| &edit.target.snapshot))
            .chain(controls.iter().map(|edit| &edit.target.snapshot))
            .chain(run_removals.iter().map(|target| &target.snapshot))
            .chain(structural.iter().map(|edit| &edit.target._snapshot))
        {
            self.validate_snapshot(snapshot)?;
        }
        let mut by_index = BTreeMap::new();
        let number_refs = numbering::paragraph_targets(self, resources)?;
        for edit in edits {
            if by_index
                .insert(edit.target.body_index, edit.alignment)
                .is_some()
            {
                return Err(unsupported("multiple alignment edits target one paragraph"));
            }
        }
        let mut table_targets = BTreeMap::new();
        for insertion in tables {
            if table_targets
                .insert(insertion.target.body_index, &insertion.table)
                .is_some()
            {
                return Err(unsupported(
                    "multiple table insertions target one paragraph",
                ));
            }
        }
        let mut package = self.package.clone();
        // Check unused definitions against the original snapshot, and make
        // newly added definitions available to every payload codec.
        self.stage_styles(&mut package, styles)?;
        let destructive_styles = styles
            .iter()
            .any(|s| matches!(s, StyleEdit::Replace { .. } | StyleEdit::Remove(_)));
        let mut paragraph_targets = BTreeMap::new();
        for insertion in paragraphs {
            if paragraph_targets
                .insert(insertion.target.body_index, &insertion.paragraph)
                .is_some()
            {
                return Err(unsupported(
                    "multiple paragraph insertions target one paragraph",
                ));
            }
            if table_targets.contains_key(&insertion.target.body_index) {
                return Err(unsupported("multiple insertions target one paragraph"));
            }
        }
        let mut paragraph_insertions = Vec::new();
        let mut changes = Vec::new();
        let mut insertions = Vec::new();
        let mut header_changes = Vec::new();
        let mut run_text_changes = Vec::new();
        let mut inline_changes = Vec::new();
        let mut removed_runs = Vec::new();
        let mut cell_alignment_changes = Vec::new();
        let mut block_changes = Vec::new();
        if !by_index.is_empty()
            || !number_refs.is_empty()
            || !table_targets.is_empty()
            || !header_images.is_empty()
            || !ranges.is_empty()
            || !run_text.is_empty()
            || !inline_edits.is_empty()
            || !run_removals.is_empty()
            || !paragraph_targets.is_empty()
            || !cells.is_empty()
            || !blocks.is_empty()
            || !controls.is_empty()
            || !structural.is_empty()
        {
            let mut root = package.word(&self.main)?;
            let headers = self.plan_header_images(&root, header_images)?;
            let occupied = by_index
                .keys()
                .chain(number_refs.keys())
                .chain(table_targets.keys())
                .chain(paragraph_targets.keys())
                .copied()
                .chain(cells.iter().map(|e| e.target._body_index))
                .chain(structural.iter().map(|e| e.target._body_index))
                .chain(ranges.iter().map(|e| e.target.body_index))
                .chain(
                    run_text
                        .iter()
                        .map(|e| &e.target)
                        .chain(inline_edits.iter().map(|e| &e.target))
                        .chain(run_removals)
                        .filter_map(|a| match a.location {
                            InlineLocation::Body(i) => Some(i),
                            _ => None,
                        }),
                )
                .collect();
            let inserted_after = table_targets
                .keys()
                .chain(paragraph_targets.keys())
                .copied()
                .collect();
            let body = root.child_mut("w:body").ok_or_else(missing_body)?;
            let sites = if table_targets.is_empty() {
                Vec::new()
            } else {
                insertion::table_sites(body, &table_targets)?
            };
            let paragraph_sites = if paragraph_targets.is_empty() {
                Vec::new()
            } else {
                insertion::paragraph_sites(
                    body,
                    &paragraph_targets.keys().copied().collect::<BTreeSet<_>>(),
                )?
            };
            let mut expanded = ranges::expand(body, ranges)?;
            expanded.extend_from_slice(run_text);
            // Plan destructive cross-family overlaps before rendering payloads.
            let mut addressed = BTreeSet::new();
            for anchor in expanded
                .iter()
                .map(|edit| &edit.target)
                .chain(run_removals)
                .chain(inline_edits.iter().map(|edit| &edit.target))
            {
                if let (InlineLocation::Body(body), [index]) =
                    (&anchor.location, anchor.path.as_slice())
                    && !addressed.insert((*body, *index))
                {
                    return Err(unsupported("duplicate or conflicting run target"));
                }
            }
            // Control paths include raw XML child slots: resolve them before
            // any property insertion or run, row, cell or block splice.
            let controls_changed = self.apply_controls(body, controls)?;
            let mut block_plan =
                blocks::Plan::build(self, &mut root, blocks, &occupied, &inserted_after)?;
            let body = root.child_mut("w:body").ok_or_else(missing_body)?;
            let (staged, actions) = text::prepare(package, &self.main, inline_edits)?;
            package = tables::structure(body, staged, &self.main, structural, cells)?;
            (run_text_changes, removed_runs) =
                text::apply(body, &expanded, run_removals, actions, &mut inline_changes)?;
            cell_alignment_changes = tables::apply(body, cells)?;
            let numbering_changed = numbering::patch_paragraphs(body, &number_refs)?;
            for (body_index, paragraph) in body
                .elements_mut()
                .filter(|node| node.name != "w:sectPr")
                .enumerate()
            {
                let Some(after) = by_index.remove(&body_index) else {
                    continue;
                };
                if paragraph.name != "w:p" {
                    return Err(unsupported("target is not a body paragraph"));
                }
                if let Some(before) = patch_alignment(paragraph, after)? {
                    changes.push(AlignmentChange {
                        body_index,
                        before,
                        after,
                    });
                }
            }
            if !by_index.is_empty() {
                return Err(unsupported("target paragraph not found"));
            }
            // Header reference edits use original body slots, before splicing tables.
            let (staged, staged_headers) = headers.stage(package, &self.main, body)?;
            package = staged;
            header_changes = staged_headers;
            let mut pending = Vec::new();
            if !sites.is_empty() {
                let mut payloads = Vec::with_capacity(sites.len());
                let mut slots = Vec::with_capacity(sites.len());
                for (body_index, slot, width) in sites {
                    let table = table_targets[&body_index];
                    insertions.push(TableInsertionChange {
                        after_body_index: body_index,
                        rows: table.rows().len(),
                        columns: table.columns(),
                    });
                    slots.push(slot);
                    payloads.push((table.clone(), width));
                }
                let (staged, nodes) =
                    crate::ooxml::encode_editor_tables(package, &self.main, payloads)?;
                package = staged;
                pending.extend(slots.into_iter().zip(nodes));
            }
            if !paragraph_sites.is_empty() {
                let mut payloads = Vec::new();
                let mut slots = Vec::new();
                for (index, slot) in paragraph_sites {
                    paragraph_insertions.push(index);
                    slots.push(slot);
                    payloads.push(paragraph_targets[&index].clone());
                }
                let (staged, nodes) =
                    crate::ooxml::encode_editor_paragraphs(package, &self.main, payloads)?;
                package = staged;
                pending.extend(slots.into_iter().zip(nodes));
            }
            package = block_plan.stage(package, &self.main, body, pending)?;
            block_changes = block_plan.changes;
            if controls_changed
                || numbering_changed
                || !changes.is_empty()
                || !block_changes.is_empty()
                || !run_text_changes.is_empty()
                || !inline_changes.is_empty()
                || !removed_runs.is_empty()
                || !cell_alignment_changes.is_empty()
                || !structural.is_empty()
                || !insertions.is_empty()
                || !paragraph_insertions.is_empty()
                || header_changes
                    .iter()
                    .any(|change| change.mode == HeaderEditMode::SectionLocal)
            {
                package.put_word(&self.main, root)?;
            }
            if destructive_styles {
                // Revalidate against the staged content, including implicit
                // style references emitted by payload codecs. The plan always
                // derives the catalog itself from the immutable source.
                self.stage_styles(&mut package, styles)?;
            }
        }
        // A defaults patch shares the styles part. Read the staged version.
        numbering::apply(&mut package, &self.main, resources)?;
        resource_fields::apply(&mut package, &self.main, resources)?;
        Ok(EditPreview {
            block_changes,
            run_text_changes,
            inline_changes,
            run_removals: removed_runs,
            cell_alignment_changes,
            changes,
            insertions,
            paragraph_insertions,
            header_changes,
            output: PackagePlan::new(package)?,
        })
    }

    fn validate_snapshot(&self, snapshot: &Arc<()>) -> Result<()> {
        if !Arc::ptr_eq(&self.snapshot, snapshot) {
            return Err(unsupported(
                "target belongs to a different document snapshot",
            ));
        }
        Ok(())
    }
}

const ALTERNATE: &str =
    "{http://schemas.openxmlformats.org/markup-compatibility/2006}AlternateContent";

fn missing_body() -> Error {
    Error::Validation("document has no body".into())
}

fn unsupported(message: &str) -> Error {
    Diagnostic::new(Rule::UnsupportedEdit, message).into()
}

fn read_alignment(value: &str) -> Option<Alignment> {
    use Alignment::*;
    [
        Left,
        Center,
        Right,
        Justify,
        Distribute,
        JustifyMedium,
        JustifyHigh,
        JustifyLow,
        ThaiJustify,
    ]
    .into_iter()
    .find(|alignment| alignment.xml() == value)
}

fn read_paragraph_inlines(
    node: &Node,
    snapshot: &Arc<()>,
    location: InlineLocation,
) -> (Vec<ImportedInline>, Vec<ImportedInlineNode>) {
    let mut flat = Vec::new();
    let mut address = InlineAnchor {
        snapshot: snapshot.clone(),
        location,
        path: Vec::new(),
    };
    let structure = read_inline_nodes(
        node.elements().filter(|child| child.name != "w:pPr"),
        &mut address,
        &mut flat,
        None,
    );
    (flat, structure)
}

fn read_inline_nodes<'a>(
    nodes: impl Iterator<Item = &'a Node>,
    address: &mut InlineAnchor,
    flat: &mut Vec<ImportedInline>,
    revision: Option<&str>,
) -> Vec<ImportedInlineNode> {
    nodes
        .enumerate()
        .map(|(index, node)| {
            address.path.push(index);
            let item = match node.name.as_str() {
                "w:r" => {
                    let mut items = Vec::new();
                    read_run(
                        node,
                        &mut items,
                        matches!(revision, Some("deletion" | "moveSource")),
                    );
                    flat.extend(items.iter().cloned());
                    ImportedInlineNode::Run(ImportedRun {
                        anchor: address.clone(),
                        items,
                    })
                }
                "w:hyperlink" => ImportedInlineNode::Hyperlink(ImportedHyperlink {
                    anchor: address.clone(),
                    relationship_id: node.get("r:id").map(str::to_owned),
                    bookmark: node.get("w:anchor").map(str::to_owned),
                    children: read_inline_nodes(node.elements(), address, flat, revision),
                }),
                "w:ins" | "w:del" | "w:moveFrom" | "w:moveTo"
                    if revision.is_none()
                        && (matches!(node.name.as_str(), "w:ins" | "w:del")
                            || node.get("w:author").is_some()) =>
                {
                    let kind = match node.name.as_str() {
                        "w:ins" => "insertion",
                        "w:del" => "deletion",
                        "w:moveFrom" => "moveSource",
                        _ => "moveDestination",
                    };
                    flat.push(ImportedInline::Opaque(node.name.to_string()));
                    ImportedInlineNode::Revision(ImportedRevision {
                        anchor: address.clone(),
                        kind,
                        id: node.get("w:id").map(str::to_owned),
                        author: node.get("w:author").map(str::to_owned),
                        date: node.get("w:date").map(str::to_owned),
                        children: read_inline_nodes(
                            node.elements(),
                            address,
                            &mut Vec::new(),
                            Some(kind),
                        ),
                    })
                }
                name => {
                    flat.push(ImportedInline::Opaque(name.to_owned()));
                    ImportedInlineNode::Opaque(name.to_owned())
                }
            };
            address.path.pop();
            item
        })
        .collect()
}

fn read_run(node: &Node, items: &mut Vec<ImportedInline>, deletion: bool) {
    for item in node.elements() {
        if deletion && item.name == "w:delText" {
            let text = item
                .children
                .iter()
                .filter_map(|child| match child {
                    crate::xml::Content::Text(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            items.push(ImportedInline::Text(text));
            if item.elements().next().is_some() {
                items.push(ImportedInline::Opaque(item.name.as_str().to_owned()));
            }
            continue;
        }
        match RunChild::classify(item) {
            RunChild::Properties => (),
            RunChild::Text(text) => {
                items.push(ImportedInline::Text(text));
                if item.elements().next().is_some() {
                    items.push(ImportedInline::Opaque(item.name.as_str().to_owned()));
                }
            }
            RunChild::Tab => items.push(ImportedInline::Tab),
            RunChild::Break(kind) => items.push(ImportedInline::Break(kind)),
            RunChild::CommentReference | RunChild::Opaque => {
                items.push(ImportedInline::Opaque(item.name.as_str().to_owned()));
            }
        }
    }
}

/// Outer `None` means no write; inner `None` means no previous value.
fn patch_alignment(paragraph: &mut Node, after: Alignment) -> Result<Option<Option<String>>> {
    patch_property(paragraph, "w:jc", after.xml())
}

fn patch_property(
    paragraph: &mut Node,
    property: &str,
    value: &str,
) -> Result<Option<Option<String>>> {
    if paragraph.elements().filter(|n| n.name == "w:pPr").count() > 1
        || paragraph.child(ALTERNATE).is_some()
        || paragraph.child("w:pPr").is_some_and(|props| {
            props.elements().filter(|n| n.name == property).count() > 1
                || props.child(ALTERNATE).is_some()
        })
    {
        return Err(unsupported(if property == "w:jc" {
            "paragraph alignment is structurally ambiguous"
        } else {
            "paragraph property is structurally ambiguous"
        }));
    }

    if paragraph.child("w:pPr").is_none() {
        paragraph
            .children
            .insert(0, Content::Element(Node::new("w:pPr")));
    }
    let props = paragraph.child_mut("w:pPr").unwrap();
    if property == "w:keepNext"
        && let Some(current) = props.child(property)
        && matches!(
            (current.get("w:val"), value),
            (None | Some("true" | "on" | "1"), "1") | (Some("false" | "off" | "0"), "0")
        )
    {
        return Ok(None);
    }
    let jc = props.ensure(property);
    if jc.get("w:val") == Some(value) {
        return Ok(None);
    }
    let before = jc.get("w:val").map(str::to_owned);
    jc.set("w:val", value);
    Ok(Some(before))
}
