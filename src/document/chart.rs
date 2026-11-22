//! Data and dimensions for a single workbook-free column series.
use super::{Diagnostic, Rule, invalid_xml_character, measure::drawing_dimension};

/// One inline column chart. Category and value positions correspond one to one.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnChartProperties {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
    /// Ordered, nonempty collection of category labels.
    pub categories: Vec<String>,
    /// One finite number per category.
    pub values: Vec<f64>,
}
setters!(ColumnChartProperties {
    with_width => width: value<f64>,
    with_height => height: value<f64>,
    with_categories => categories: value<Vec<String>>,
    with_values => values: value<Vec<f64>>,
});

impl ColumnChartProperties {
    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        self.dimensions_emu()?;
        if self.categories.is_empty() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "column chart requires at least one category",
            )
            .at("categories"));
        }
        if u32::try_from(self.categories.len()).is_err() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "column chart category count exceeds the DrawingML range",
            )
            .at("categories"));
        }
        if self.categories.len() != self.values.len() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "column chart requires one value per category",
            )
            .at("values"));
        }
        for (index, label) in self.categories.iter().enumerate() {
            if let Some((offset, character)) = invalid_xml_character(label) {
                return Err(Diagnostic::new(
                    Rule::InvalidXml,
                    format!(
                        "invalid XML 1.0 character U+{:04X} at byte offset {offset}",
                        character as u32
                    ),
                )
                .at(format!("categories[{index}]")));
            }
        }
        for (index, value) in self.values.iter().enumerate() {
            if !value.is_finite() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "column chart values must be finite",
                )
                .at(format!("values[{index}]")));
            }
        }
        Ok(())
    }

    pub(crate) fn dimensions_emu(&self) -> Result<(i64, i64), Diagnostic> {
        Ok((
            drawing_dimension(self.width, "width", "chart")?,
            drawing_dimension(self.height, "height", "chart")?,
        ))
    }
}
