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
impl NumberingLevel {
    fn validate_formats(&self) -> Result<(), Diagnostic> {
        if let Some(format) = &self.label_format {
            if format.style.is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering label formats cannot contain run style references",
                )
                .at("label_format.style"));
            }
            at(format.validate(), "label_format")?;
        }
        if let Some(format) = &self.paragraph_format {
            if format.style.is_some() || format.heading.is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering level paragraph formats cannot contain style references or headings",
                )
                .at("paragraph_format"));
            }
            at(format.validate(), "paragraph_format")?;
        }
        Ok(())
    }
}
impl NumberingDefinition {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.inherit_style.is_some()
            && !(self.levels.is_empty()
                && self.level_overrides.is_empty()
                && self.start_overrides.is_empty())
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "inherited numbering definitions cannot contain levels or overrides",
            )
            .at("inherit_style"));
        }
        let mut levels = std::collections::BTreeSet::new();
        for (index, level) in self.levels.iter().enumerate() {
            if level.level > 8 || !levels.insert(level.level) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "numbering levels must be unique and in 0..=8",
                )
                .at(format!("levels[{index}].level")));
            }
            at(level.validate_formats(), format!("levels[{index}]"))?;
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
            at(
                entry.validate_formats(),
                format!("level_overrides[{index}]"),
            )?;
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
        if let Some(margins) = &self.page_margins {
            for (field, value) in [("header", margins.header), ("footer", margins.footer)] {
                if value.is_some_and(|value| value < 0) {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "header and footer distances must be nonnegative",
                    )
                    .at(format!("page_margins.{field}")));
                }
            }
        }
        if let Some(borders) = &self.page_borders {
            for (side, border) in [
                ("top", &borders.top),
                ("left", &borders.left),
                ("bottom", &borders.bottom),
                ("right", &borders.right),
            ] {
                let Some(border) = border else { continue };
                if border.size.is_some_and(|size| !(2..=96).contains(&size)) {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "page border size must be in 2..=96 eighth-points",
                    )
                    .at(format!("page_borders.{side}.size")));
                }
                if border.space.is_some_and(|space| space > 31) {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "page border space must be in 0..=31 points",
                    )
                    .at(format!("page_borders.{side}.space")));
                }
            }
        }
        if let Some(line) = &self.line_numbering {
            if line.clear
                && (line.count_by.is_some()
                    || line.start.is_some()
                    || line.distance.is_some()
                    || line.restart.is_some())
            {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "clearing line numbering cannot be combined with explicit settings",
                )
                .at("line_numbering.clear"));
            }
            if line
                .count_by
                .is_some_and(|value| !(1..=100).contains(&value))
            {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "line numbering count_by must be in 1..=100",
                )
                .at("line_numbering.count_by"));
            }
            if line.start.is_some_and(|value| value > 32767) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "line numbering start must be in 0..=32767",
                )
                .at("line_numbering.start"));
            }
        }
        if self.document_grid.as_ref().and_then(|grid| grid.line_pitch) == Some(0) {
            return Err(Diagnostic::new(
                Rule::InvalidMeasurement,
                "grid line pitch must be positive",
            )
            .at("document_grid.line_pitch"));
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
impl ImageCrop {
    pub(crate) fn wire_values(&self) -> Result<[u32; 4], Diagnostic> {
        let mut values = [0; 4];
        for (value, (name, inset)) in values.iter_mut().zip([
            ("left", self.left),
            ("top", self.top),
            ("right", self.right),
            ("bottom", self.bottom),
        ]) {
            if !(0. ..=100.).contains(&inset) {
                return Err(Diagnostic::new(
                    Rule::InvalidMeasurement,
                    "image crop percentages must be in 0..=100",
                )
                .at(format!("crop.{name}")));
            }
            *value = (inset * 1000.).round_ties_even() as u32;
        }
        if values[0] + values[2] >= 100000 || values[1] + values[3] >= 100000 {
            return Err(Diagnostic::new(
                Rule::InvalidMeasurement,
                "image crop must leave a positive source rectangle",
            )
            .at("crop"));
        }
        Ok(values)
    }
}
impl ImageAnchor {
    pub(crate) fn wrap_distances_emu(&self) -> Result<[Option<u32>; 4], Diagnostic> {
        let distances = [
            self.distance_top,
            self.distance_bottom,
            self.distance_left,
            self.distance_right,
        ];
        let polygon_wrap = matches!(self.wrap, ImageWrap::Tight | ImageWrap::Through);
        if polygon_wrap {
            let points = &self.wrap_polygon;
            let area = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(&(x1, y1), &(x2, y2))| {
                    i128::from(x1) * i128::from(y2) - i128::from(x2) * i128::from(y1)
                })
                .sum::<i128>();
            if points.len() < 3 || area == 0 {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "picture wrap polygon requires at least three vertices and nonzero signed area",
                )
                .at("anchor.wrap_polygon"));
            }
        } else if !self.wrap_polygon.is_empty() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "picture wrap polygon requires tight or through wrapping",
            )
            .at("anchor.wrap_polygon"));
        }
        if (self.wrap_side.is_some() && !polygon_wrap && self.wrap != ImageWrap::Square)
            || (polygon_wrap && (self.distance_top.is_some() || self.distance_bottom.is_some()))
            || (self.wrap == ImageWrap::None && distances.iter().any(Option::is_some))
            || (self.wrap == ImageWrap::TopAndBottom
                && (self.distance_left.is_some() || self.distance_right.is_some()))
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "wrap sides and distances must match the wrap mode",
            )
            .at("anchor.wrap"));
        }
        let mut result = [None; 4];
        for ((output, value), name) in result.iter_mut().zip(distances).zip([
            "anchor.distance_top",
            "anchor.distance_bottom",
            "anchor.distance_left",
            "anchor.distance_right",
        ]) {
            if let Some(value) = value {
                measurement(value, 12700., true, name)?;
                *output = Some(
                    u32::try_from(Points::new(value)?.emu()?.get()).map_err(|_| {
                        Diagnostic::new(
                            Rule::InvalidMeasurement,
                            "wrap distance exceeds the DrawingML range",
                        )
                        .at(name)
                    })?,
                );
            }
        }
        Ok(result)
    }

    pub(crate) fn offsets_emu(&self) -> Result<[i32; 2], Diagnostic> {
        if matches!(
            self.horizontal_reference,
            ImageAnchorReference::Paragraph
                | ImageAnchorReference::Line
                | ImageAnchorReference::TopMargin
                | ImageAnchorReference::BottomMargin
        ) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "invalid horizontal picture reference frame",
            )
            .at("anchor.horizontal_reference"));
        }
        if matches!(
            self.vertical_reference,
            ImageAnchorReference::Column
                | ImageAnchorReference::Character
                | ImageAnchorReference::LeftMargin
                | ImageAnchorReference::RightMargin
        ) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "invalid vertical picture reference frame",
            )
            .at("anchor.vertical_reference"));
        }
        if (self.horizontal_alignment.is_some() && self.x != 0.)
            || (self.vertical_alignment.is_some() && self.y != 0.)
        {
            return Err(Diagnostic::new(Rule::InvalidContent,
                "picture anchor alignment cannot be combined with a nonzero offset on the same axis")
                .at("anchor"));
        }
        let offset = |value, name| {
            Points::new(value)
                .and_then(Points::emu)
                .and_then(|emu| {
                    i32::try_from(emu.get()).map_err(|_| {
                        Diagnostic::new(
                            Rule::InvalidMeasurement,
                            "picture anchor offset must fit a signed DrawingML coordinate",
                        )
                    })
                })
                .map_err(|error| error.at(name))
        };
        Ok([offset(self.x, "anchor.x")?, offset(self.y, "anchor.y")?])
    }
}
impl ImageTransformation {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if let Some(anchor) = &self.anchor {
            anchor.offsets_emu()?;
            anchor.wrap_distances_emu()?;
        }
        if self
            .rotation
            .is_some_and(|value| !(-360. ..=360.).contains(&value))
        {
            return Err(Diagnostic::new(
                Rule::InvalidMeasurement,
                "image rotation must be in -360..=360 degrees",
            )
            .at("rotation"));
        }
        if let Some(crop) = &self.crop {
            crop.wire_values()?;
        }
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
        if let Some(control) = &self.content_control {
            control.validate_table_context()?;
        }

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
        if let Some(control) = &self.content_control {
            control.validate_table_context()?;
        }

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
            ResolvedBlock::ContentControl(control) => {
                blocks(control.children(), &path.field("children"))?;
            }
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
                                content_control: cell.content_control.clone(),
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
        let mut abstracts = std::collections::BTreeMap::new();
        for (index, definition) in self.numbering.iter().enumerate() {
            at(definition.validate(), format!("numbering[{index}]"))?;
            if let Some(id) = definition.abstract_id
                && let Some(previous) =
                    abstracts.insert(id, (&definition.levels, &definition.inherit_style))
                && previous != (&definition.levels, &definition.inherit_style)
            {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "shared abstract numbering IDs require identical levels and inherited style",
                )
                .at(format!("numbering[{index}].levels")));
            }
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
        let stories = story_inlines(self);
        for (kind, runs) in &stories {
            for (run, path) in runs {
                if let Some(revision) = run.revision() {
                    at(revision.validate(), format!("{path}.revision"))?;
                }
                if let ResolvedInline::Shape(shape) = run {
                    at(shape.validate(), path)?;
                    if *kind != StoryKind::Body {
                        return Err(Diagnostic::new(
                            Rule::InvalidContent,
                            "DrawingML shapes are supported only in body paragraphs",
                        )
                        .at(path));
                    }
                }
                if matches!(run, ResolvedInline::NoteReference(_)) && *kind != StoryKind::Body {
                    return Err(Diagnostic::new(
                        Rule::InvalidContent,
                        "note references are supported only in the body story",
                    )
                    .at(path));
                }
            }
        }
        super::revision::validate_moves(&stories)?;
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
