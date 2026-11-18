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
        if let Some(size) = self.size {
            measurement(size, 2., true, "size")?;
        }
        Ok(())
    }
}
impl ParagraphFormat {
    pub fn validate(&self) -> Result<(), Diagnostic> {
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
            at(run.validate(), format!("children[{index}].format"))?;
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
                    for (cell_index, cell) in row.cells.iter().enumerate() {
                        let path = path
                            .field("rows")
                            .index(row_index)
                            .field("children")
                            .index(cell_index);
                        at(
                            TableCellProperties {
                                grid_span: Some(cell.grid_span),
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
