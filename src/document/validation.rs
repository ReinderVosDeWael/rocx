//! Pure checks shared by both frontends and the compiler; no XML or I/O effects.
//! P1 (2016) §§17.3.2.38 (size), 17.3.1.33 (spacing), 17.4 (tables),
//! and 17.6 (sections). Coordinate caps remain application limits, not XSD ranges.
use super::*;

fn measurement(value: f64, factor: f64, nonnegative: bool, name: &str) -> Result<(), Diagnostic> {
    if nonnegative && value < 0. {
        return Err(Diagnostic::new(
            Rule::InvalidMeasurement,
            format!("{name} cannot be negative"),
        )
        .at(name));
    }
    Points::new(value)
        .and_then(|p| p.scaled(factor))
        .map(|_| ())
        .map_err(|d| d.at(name))
}

impl RunFormat {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(size) = self.size_complex_script {
            measurement(size, 2., true, "size_complex_script")?;
        }
        if let Some(size) = self.size {
            measurement(size, 2., true, "size")?;
        }
        Ok(())
    }
}
impl NumberingDefinition {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        let mut levels = std::collections::BTreeSet::new();
        for (index, level) in self.levels.iter().enumerate() {
            if level.level > 8 || !levels.insert(level.level) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering levels must be unique and in 0..=8",
                )
                .at(format!("levels[{index}].level")));
            }
        }
        let mut overridden = std::collections::BTreeSet::new();
        for (index, entry) in self.start_overrides.iter().enumerate() {
            if !levels.contains(&entry.level) || !overridden.insert(entry.level) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering start override must target a unique defined level",
                )
                .at(format!("start_overrides[{index}].level")));
            }
        }
        let mut replaced = std::collections::BTreeSet::new();
        for (index, entry) in self.level_overrides.iter().enumerate() {
            if !levels.contains(&entry.level) || !replaced.insert(entry.level) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering level override must target a unique defined level",
                )
                .at(format!("level_overrides[{index}].level")));
            }
        }
        Ok(())
    }
}
impl ParagraphFormat {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        let mut positions = std::collections::BTreeSet::new();
        for (index, stop) in self.tab_stops.iter().enumerate() {
            let position = Points::new(stop.position)
                .and_then(Points::twips)
                .map_err(|error| error.at(format!("tab_stops[{index}].position")))?
                .get();
            if !positions.insert(position) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "tab stops collide after rounding to twips",
                )
                .at(format!("tab_stops[{index}].position")));
            }
        }
        if self.line_spacing.is_some() && self.line_height.is_some() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "line_height and line_spacing are mutually exclusive",
            )
            .at("line_height"));
        }
        if let Some(height) = self.line_height {
            let (value, factor) = match height {
                LineHeight::Multiple(value) => (value, 240.),
                LineHeight::Exact(value) | LineHeight::AtLeast(value) => (value, 20.),
            };
            measurement(value, factor, true, "line_height")?;
        }
        if self.heading.is_some_and(|h| h > 9) {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "heading must be in 0..=9").at("heading"),
            );
        }
        for (name, value, factor, unsigned) in [
            ("spacing_before", self.spacing_before, 20., true),
            ("spacing_after", self.spacing_after, 20., true),
            ("line_spacing", self.line_spacing, 240., true),
            ("left_indent", self.left_indent, 20., false),
            ("right_indent", self.right_indent, 20., false),
            ("first_line_indent", self.first_line_indent, 20., false),
        ] {
            if let Some(value) = value {
                measurement(value, factor, unsigned, name)?;
            }
        }
        Ok(())
    }
}
impl SectionProperties {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(size) = &self.page_size {
            for (name, value) in [
                ("page_size.width", size.width),
                ("page_size.height", size.height),
            ] {
                if let Some(value) = value
                    && Emu::new(value).twips().get() <= 0
                {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "page size must be positive",
                    )
                    .at(name));
                }
            }
        }
        if self.columns.as_ref().and_then(|c| c.count) == Some(0) {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "column count must be positive")
                    .at("columns.count"),
            );
        }
        Ok(())
    }
}
impl ImageTransformation {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        for (name, value) in [("width", self.width), ("height", self.height)] {
            if let Some(value) = value {
                let emu = Points::new(value)
                    .and_then(Points::emu)
                    .map_err(|d| d.at(name))?
                    .get();
                if emu <= 0 || emu > i32::MAX as i64 {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "image dimensions must be positive and fit DrawingML coordinates",
                    )
                    .at(name));
                }
            }
        }
        Ok(())
    }
}
impl TableCellProperties {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.grid_span == Some(0) {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "grid_span must be positive").at("grid_span"),
            );
        }
        if let Some(margins) = &self.margins {
            at(margins.validate(), "margins")?;
        }
        Ok(())
    }
}
impl CellMargins {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        for (name, value) in [
            ("top", self.top),
            ("start", self.start),
            ("bottom", self.bottom),
            ("end", self.end),
        ] {
            if let Some(value) = value {
                measurement(value, 20., true, name)?;
            }
        }
        Ok(())
    }
}
impl TableRowProperties {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(height) = self.height {
            let (RowHeight::Exact(points) | RowHeight::AtLeast(points)) = height;
            measurement(points, 20., true, "height")?;
        }
        Ok(())
    }
}

fn at<T>(result: Result<T, Diagnostic>, path: impl std::fmt::Display) -> Result<T, Diagnostic> {
    result.map_err(|mut error| {
        error.location = Some(match error.location {
            Some(location) => format!("{path}.{location}"),
            None => path.to_string(),
        });
        error
    })
}
impl ResolvedInline {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Self::RangeStart {
            kind: RangeKind::StructuredComment(body),
            ..
        } = self
        {
            at(body.validate(), "content")?;
        }
        if let Some(format) = self.format() {
            format.validate()?;
        }
        Ok(())
    }
}
impl ResolvedParagraph {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        at(self.format.validate(), "format")?;
        for (index, run) in self.runs.iter().enumerate() {
            let path = if run.format().is_some() {
                format!("children[{index}].format")
            } else {
                format!("children[{index}]")
            };
            at(run.validate(), path)?;
        }
        Ok(())
    }
}
fn blocks(values: &[ResolvedBlock], path: &DeclarationPath) -> Result<(), Diagnostic> {
    for (index, block) in values.iter().enumerate() {
        let path = path.index(index);
        match block {
            ResolvedBlock::Paragraph(p) => at(p.validate(), &path)?,
            ResolvedBlock::Table(t) => {
                for (row_index, row) in t.rows.iter().enumerate() {
                    at(
                        row.properties.validate(),
                        path.field("rows").index(row_index).field("properties"),
                    )?;
                    for (cell_index, cell) in row.cells.iter().enumerate() {
                        let path = path
                            .field("rows")
                            .index(row_index)
                            .field("children")
                            .index(cell_index);
                        at(
                            TableCellProperties {
                                grid_span: Some(cell.grid_span),
                                margins: cell.margins.clone(),
                                ..Default::default()
                            }
                            .validate(),
                            &path,
                        )?;
                        blocks(&cell.blocks, &path.field("children"))?;
                    }
                }
            }
        }
    }
    Ok(())
}
impl ResolvedDocument {
    /// Validate supported domain invariants independently of either frontend or codec.
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(defaults) = &self.defaults {
            at(defaults.validate(), "defaults")?;
        }
        validate_ranges(self)?;
        for (index, definition) in self.numbering.iter().enumerate() {
            at(definition.validate(), format!("numbering[{index}]"))?;
        }
        let mut notes = std::collections::BTreeSet::new();
        for (i, note) in self.notes().iter().enumerate() {
            if !notes.insert(note.target()) {
                return Err(
                    Diagnostic::new(Rule::InvalidContent, "duplicate note definition")
                        .at(format!("notes[{i}].target")),
                );
            }
            at(note.validate(), format!("notes[{i}]"))?;
        }
        for (kind, runs) in story_inlines(self) {
            for (run, path) in runs {
                if matches!(run, ResolvedInline::NoteReference(_)) && kind != StoryKind::Body {
                    return Err(Diagnostic::new(
                        Rule::InvalidContent,
                        "note references are supported only in the body story",
                    )
                    .at(path));
                }
            }
        }
        for (index, section) in self.sections.iter().enumerate() {
            let path = DeclarationPath::root("sections").index(index);
            if let Some(properties) = &section.properties {
                at(properties.validate(), path.field("properties"))?;
            }
            blocks(&section.blocks, &path.field("children"))?;
            for (name, stories) in [("headers", &section.headers), ("footers", &section.footers)] {
                for (kind, story) in stories {
                    let kind = match kind {
                        HeaderFooterType::Default => "default",
                        HeaderFooterType::First => "first",
                        HeaderFooterType::Even => "even",
                    };
                    blocks(
                        &story.blocks,
                        &path.field(name).field(kind).field("children"),
                    )?;
                }
            }
        }
        Ok(())
    }
}
impl DocumentDefaults {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(run) = &self.run {
            if run.style.is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "document defaults cannot contain run style references",
                )
                .at("run.style"));
            }
            at(run.validate(), "run")?;
        }
        if let Some(paragraph) = &self.paragraph {
            if paragraph.style.is_some() || paragraph.heading.is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "document defaults cannot contain paragraph style references or headings",
                )
                .at("paragraph"));
            }
            at(paragraph.validate(), "paragraph")?;
        }
        Ok(())
    }
}

impl ResolvedNote {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        blocks(&self.blocks, &DeclarationPath::root("children"))
    }
}
impl ResolvedCommentBody {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        at(self.metadata.validate(), "metadata")?;
        blocks(&self.blocks, &DeclarationPath::root("children"))
    }
}
