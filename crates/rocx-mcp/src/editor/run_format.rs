use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow::{self, Break, Continue};
use std::sync::{Arc, OnceLock};

use rocx::editor::{
    BodyBlock, EntryKind, ImportedDocument, ImportedEntry, ImportedInline, ImportedInlineNode,
    ImportedRowChild, ImportedRun, ImportedStory, ImportedTableCell, ImportedTableChild,
    InlineEdit,
};
use rocx::{Color, RunFormat, ThemeColor, Underline};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text, read_diagnostic};
use crate::{diagnostics::ScopedEditError, schema::run_format::RunFormatChange};

const OWNER: &str = "format-owner:";
const RUN: &str = "format-run:";

#[derive(Clone, Copy)]
enum Owner<'a> {
    Body,
    Story(usize, &'a str),
    Entry(usize, &'a str),
}

impl Owner<'_> {
    fn value(self) -> Value {
        match self {
            Self::Body => json!({"anchor": format!("{OWNER}body")}),
            Self::Story(i, part) => {
                json!({"anchor": format!("{OWNER}story:{i}"), "part_name": part})
            }
            Self::Entry(i, part) => {
                json!({"anchor": format!("{OWNER}entry:{i}"), "part_name": part})
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Paragraph<'a> {
    Body(usize),
    Cell(usize, usize, usize, usize),
    Story(usize, usize, &'a ImportedStory),
    Entry(usize, usize, &'a ImportedEntry),
}

impl Paragraph<'_> {
    fn value(
        self,
        owner: Owner<'_>,
        inline: usize,
        child: Option<usize>,
        run: &ImportedRun,
    ) -> Value {
        let (mut source, local, candidate) = match self {
            Self::Body(b) => (
                json!({"kind": "body", "body_index": b}),
                format!("body:{b}"),
                true,
            ),
            Self::Cell(b, r, c, p) => (
                json!({
                    "kind": "body-cell", "body_index": b, "row_index": r, "cell_index": c,
                    "block_index": p,
                }),
                format!("cell:{b}:{r}:{c}:{p}"),
                true,
            ),
            Self::Story(i, b, story) => (
                json!({"kind": story.kind.as_str(), "block_index": b}),
                format!("story:{i}:{b}"),
                true,
            ),
            Self::Entry(i, b, entry) => (
                json!({
                    "kind": entry.kind.as_str(), "block_index": b,
                    "id": entry.id, "note_type": entry.note_type,
                }),
                format!("entry:{i}:{b}"),
                ordinary_entry(entry),
            ),
        };
        let (text, opaque) = projected_run_text(&run.items);
        let editable = candidate && format_candidate(run);
        source["owner"] = owner.value();
        source["inline_index"] = json!(inline);
        if let Some(child) = child {
            source["child_index"] = json!(child);
        }
        source["text"] = json!(text);
        source["format"] = format_value(&run.format);
        source["hidden"] = json!(run.hidden);
        source["has_opaque_formatting"] = json!(run.has_opaque_formatting);
        source["has_opaque_content"] = json!(opaque);
        source["read_only"] = json!(!editable);
        if editable {
            let mut anchor = format!("{RUN}{local}:{inline}");
            if let Some(child) = child {
                anchor.push_str(&format!(":{child}"));
            }
            source["anchor"] = json!(anchor);
        }
        source
    }
}

fn ordinary_entry(entry: &ImportedEntry) -> bool {
    if entry.kind == EntryKind::Comment {
        return true;
    }
    let negative = entry
        .id
        .trim_matches([' ', '\t', '\r', '\n'])
        .strip_prefix('-')
        .is_some_and(|digits| {
            digits.bytes().all(|b| b.is_ascii_digit()) && !digits.trim_start_matches('0').is_empty()
        });
    matches!(entry.note_type.as_deref(), None | Some("normal")) && !negative
}

pub(super) fn format_candidate(run: &ImportedRun) -> bool {
    // Shape hint only: native checks raw XML, the selected properties, protection
    // settings, tracked ancestors, and field/range boundaries in the whole story.
    run.property_revision.is_none()
        && !run.items.is_empty()
        && run.items.iter().all(|item| match item {
            ImportedInline::Text(_) | ImportedInline::Tab | ImportedInline::Break(_) => true,
            ImportedInline::Opaque(tag) => matches!(
                tag.as_str(),
                "w:noBreakHyphen"
                    | "w:softHyphen"
                    | "w:sym"
                    | "w:lastRenderedPageBreak"
                    | "w:ptab"
                    | "w:drawing"
                    | "w:br"
            ),
            _ => false,
        })
}

struct Page {
    start: usize,
    limit: usize,
    position: usize,
    rows: Vec<Value>,
}

impl Page {
    fn push(&mut self, value: impl FnOnce() -> Value) -> ControlFlow<()> {
        if self.position >= self.start {
            if self.rows.len() == self.limit {
                return Break(());
            }
            self.rows.push(value());
        }
        self.position += 1;
        Continue(())
    }

    fn paragraph(
        &mut self,
        source: Paragraph<'_>,
        owner: Owner<'_>,
        nodes: &[ImportedInlineNode],
    ) -> ControlFlow<()> {
        for (i, node) in nodes.iter().enumerate() {
            match node {
                ImportedInlineNode::Run(run) => self.push(|| source.value(owner, i, None, run))?,
                ImportedInlineNode::Hyperlink(link) => {
                    for (j, child) in link.children.iter().enumerate() {
                        if let ImportedInlineNode::Run(run) = child {
                            self.push(|| source.value(owner, i, Some(j), run))?;
                        }
                    }
                }
                _ => {}
            }
        }
        Continue(())
    }

    fn unavailable(&mut self, catalog: &str, error: rocx::Error) -> ControlFlow<()> {
        self.push(|| {
            json!({
                "kind": "unavailable",
                "catalog": catalog,
                "error": error.to_string(),
                "diagnostic": read_diagnostic(error),
            })
        })
    }

    fn collect(&mut self, doc: &ImportedDocument) -> ControlFlow<()> {
        for (b, block) in doc.body().iter().enumerate() {
            match block {
                BodyBlock::Paragraph(p) => {
                    self.paragraph(Paragraph::Body(b), Owner::Body, &p.inline_structure)?;
                }
                BodyBlock::Table(table) => {
                    for (r, row) in table.children.iter().enumerate() {
                        let ImportedTableChild::Row(row) = row else {
                            continue;
                        };
                        for (c, cell) in row.children.iter().enumerate() {
                            let ImportedRowChild::Cell(cell) = cell else {
                                continue;
                            };
                            for p in 0..cell.blocks.len() {
                                if let Some(nodes) = cell.inline_structure(p) {
                                    self.paragraph(
                                        Paragraph::Cell(b, r, c, p),
                                        Owner::Body,
                                        nodes,
                                    )?;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        match doc.stories() {
            Ok(stories) => {
                let mut physical = BTreeSet::new();
                for (i, story) in stories.iter().enumerate() {
                    if !physical.insert(story.part_name.as_str()) {
                        continue;
                    }
                    let owner = Owner::Story(i, &story.part_name);
                    for b in 0..story.blocks.len() {
                        if let Some(nodes) = story.inline_structure(b) {
                            self.paragraph(Paragraph::Story(i, b, story), owner, nodes)?;
                        }
                    }
                }
            }
            Err(error) => self.unavailable("stories", error)?,
        }
        match doc.entry_stories() {
            Ok(entries) => {
                let mut physical = BTreeMap::new();
                for (i, entry) in entries.iter().enumerate() {
                    let first = *physical.entry(entry.part_name.as_str()).or_insert(i);
                    let owner = Owner::Entry(first, &entry.part_name);
                    for b in 0..entry.blocks().len() {
                        if let Some(nodes) = entry.inline_structure(b) {
                            self.paragraph(Paragraph::Entry(i, b, entry), owner, nodes)?;
                        }
                    }
                }
            }
            Err(error) => self.unavailable("entries", error)?,
        }
        Continue(())
    }
}

struct Sources<'a> {
    doc: &'a ImportedDocument,
    stories: OnceLock<Result<Vec<ImportedStory>, Arc<rocx::Error>>>,
}

impl Sources<'_> {
    fn stories(&self) -> rocx::Result<&[ImportedStory]> {
        self.stories
            .get_or_init(|| self.doc.stories().map_err(Arc::new))
            .as_deref()
            .map_err(|error| rocx::Error::Shared(Arc::clone(error)))
    }

    fn owner(&self, local: &str) -> Result<Option<&str>, ScopedEditError> {
        let invalid =
            || ScopedEditError::WrongAnchorType("expected a format-runs physical owner".into());
        let value = local.strip_prefix(OWNER).ok_or_else(invalid)?;
        if value == "body" {
            return Ok(None);
        }
        let (kind, index) = value.split_once(':').ok_or_else(invalid)?;
        let index = index.parse::<usize>().map_err(|_| invalid())?;
        let part = match kind {
            "story" => self.stories()?.get(index).map(|s| s.part_name.as_str()),
            "entry" => self
                .doc
                .entry_stories()?
                .get(index)
                .map(|e| e.part_name.as_str()),
            _ => return Err(invalid()),
        };
        part.map(Some)
            .ok_or_else(|| "unknown run formatting owner".into())
    }

    fn run(&self, local: &str) -> Result<&ImportedRun, ScopedEditError> {
        let wrong = || ScopedEditError::WrongAnchorType("expected a format-runs run anchor".into());
        let (kind, coords) = local
            .strip_prefix(RUN)
            .and_then(|s| s.split_once(':'))
            .ok_or_else(wrong)?;
        let invalid = "unknown run formatting anchor";
        let mut coords = coords.split(':');
        let next = |coords: &mut std::str::Split<'_, char>| {
            coords.next().and_then(|s| s.parse().ok()).ok_or(invalid)
        };
        let source = next(&mut coords)?;
        let nodes = match kind {
            "body" => match self.doc.body().get(source) {
                Some(BodyBlock::Paragraph(p)) => Some(p.inline_structure.as_slice()),
                _ => None,
            },
            "cell" => {
                let (r, c, b) = (next(&mut coords)?, next(&mut coords)?, next(&mut coords)?);
                body_cell(self.doc, source, r, c).and_then(|cell| cell.inline_structure(b))
            }
            "story" => {
                let b = next(&mut coords)?;
                self.stories()?
                    .get(source)
                    .and_then(|story| story.inline_structure(b))
            }
            "entry" => {
                let b = next(&mut coords)?;
                self.doc
                    .entry_stories()?
                    .get(source)
                    .filter(|entry| ordinary_entry(entry))
                    .and_then(|entry| entry.inline_structure(b))
            }
            _ => return Err(wrong()),
        }
        .ok_or(invalid)?;
        let node = nodes.get(next(&mut coords)?).ok_or(invalid)?;
        let child = coords
            .next()
            .map(str::parse::<usize>)
            .transpose()
            .map_err(|_| invalid)?;
        if coords.next().is_some() {
            return Err(invalid.into());
        }
        match (node, child) {
            (ImportedInlineNode::Run(run), None) if format_candidate(run) => Ok(run),
            (ImportedInlineNode::Hyperlink(link), Some(child)) => match link.children.get(child) {
                Some(ImportedInlineNode::Run(run)) if format_candidate(run) => Ok(run),
                _ => Err(invalid.into()),
            },
            _ => Err(invalid.into()),
        }
    }
}

fn body_cell(doc: &ImportedDocument, b: usize, r: usize, c: usize) -> Option<&ImportedTableCell> {
    let BodyBlock::Table(table) = doc.body().get(b)? else {
        return None;
    };
    let ImportedTableChild::Row(row) = table.children.get(r)? else {
        return None;
    };
    let ImportedRowChild::Cell(cell) = row.children.get(c)? else {
        return None;
    };
    Some(cell)
}

impl Snapshot {
    pub fn format_run_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("format-run page limit must be positive".into());
        }
        let mut page = Page {
            start,
            limit,
            position: 0,
            rows: Vec::new(),
        };
        let more = page.collect(&self.document).is_break();
        Ok((page.rows, more.then_some(page.position)))
    }

    // Session resolves only handles issued for the requesting snapshot.
    pub fn edit_run_format(
        &self,
        local_owner: &str,
        edits: Vec<RunFormatChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let sources = Sources {
            doc: &self.document,
            stories: OnceLock::new(),
        };
        let owner = sources.owner(local_owner)?;
        let edits = edits
            .into_iter()
            .map(|change| {
                Ok(InlineEdit {
                    target: sources.run(&change.anchor)?.anchor.clone(),
                    action: change.edit.into(),
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        self.document
            .edit_run_format(owner, &edits)
            .map_err(Into::into)
    }
}

fn theme_name(value: ThemeColor) -> &'static str {
    match value {
        ThemeColor::Dark1 => "dark1",
        ThemeColor::Light1 => "light1",
        ThemeColor::Dark2 => "dark2",
        ThemeColor::Light2 => "light2",
        ThemeColor::Accent1 => "accent1",
        ThemeColor::Accent2 => "accent2",
        ThemeColor::Accent3 => "accent3",
        ThemeColor::Accent4 => "accent4",
        ThemeColor::Accent5 => "accent5",
        ThemeColor::Accent6 => "accent6",
        ThemeColor::Hyperlink => "hyperlink",
        ThemeColor::FollowedHyperlink => "followedHyperlink",
        ThemeColor::Background1 => "background1",
        ThemeColor::Text1 => "text1",
        ThemeColor::Background2 => "background2",
        ThemeColor::Text2 => "text2",
    }
}

fn underline_name(value: Underline) -> &'static str {
    match value {
        Underline::None => "none",
        Underline::Single => "single",
        Underline::Double => "double",
        Underline::Dotted => "dotted",
        Underline::Dash => "dash",
        Underline::Wave => "wave",
        Underline::Words => "words",
    }
}

pub(super) fn format_value(format: &RunFormat) -> Value {
    let color = match format.color {
        Some(Color::Auto) => json!("auto"),
        Some(Color::Rgb(rgb)) => json!(rgb),
        Some(Color::Theme { name, tint, shade }) => {
            json!({"theme": theme_name(name), "tint": tint, "shade": shade})
        }
        None => Value::Null,
    };
    let language = format.language.as_ref().map(|language| {
        json!({
            "latin": language.latin, "east_asia": language.east_asia, "bidi": language.bidi,
        })
    });
    json!({
        "style": format.style, "bold": format.bold, "italic": format.italic,
        "underline": format.underline.map(underline_name), "font": format.font, "size": format.size,
        "color": color, "superscript": format.superscript, "subscript": format.subscript,
        "strike": format.strike, "all_caps": format.all_caps, "small_caps": format.small_caps,
        "snap_to_grid": format.snap_to_grid, "language": language,
        "bold_complex_script": format.bold_complex_script,
        "italic_complex_script": format.italic_complex_script,
        "size_complex_script": format.size_complex_script, "rtl": format.rtl,
        "complex_script": format.complex_script, "font_east_asia": format.font_east_asia,
        "font_complex_script": format.font_complex_script,
    })
}
