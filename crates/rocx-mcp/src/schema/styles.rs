use rmcp::schemars::{self, JsonSchema};
use rocx::{
    CharacterStyleDefinition, NumberingStyleDefinition, ParagraphStyleDefinition, StyleDefinition,
    TableSectionFormat, TableStyleDefinition,
};
use serde::Deserialize;

use super::{
    ParagraphAlignment,
    blocks::Nullable,
    payloads::{LineHeightInput, RunFormatInput, RunLanguageInput, TabStopInput},
};

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum StyleChange {
    Add {
        definition: StyleInput,
    },
    Replace {
        anchor: String,
        definition: StyleInput,
    },
    Patch {
        anchor: String,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    /// Set a native direct field using its stored string representation, or clear with null.
    Field {
        anchor: String,
        region: Option<Region>,
        field: String,
        value: Nullable<String>,
    },
    /// Replace a whole direct table region, or clear it with null.
    Region {
        anchor: String,
        region: Region,
        value: Nullable<Box<RegionFormat>>,
    },
    Remove {
        anchor: String,
    },
    /// Migrate provable stored uses to another original style of the same kind, then remove this style.
    Retarget {
        anchor: String,
        replacement: String,
    },
}

impl StyleChange {
    pub(crate) fn scope(
        &mut self,
        mut resolve: impl FnMut(&mut String) -> Result<(), String>,
    ) -> Result<(), String> {
        match self {
            Self::Add { .. } => Ok(()),
            Self::Replace { anchor, .. }
            | Self::Patch { anchor, .. }
            | Self::Field { anchor, .. }
            | Self::Region { anchor, .. }
            | Self::Remove { anchor } => resolve(anchor),
            Self::Retarget {
                anchor,
                replacement,
            } => {
                resolve(anchor)?;
                resolve(replacement)
            }
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum StyleInput {
    Character {
        name: String,
        base_style: Option<String>,
        linked_style: Option<String>,
        format: Option<Box<RunFormatInput>>,
    },
    Paragraph {
        name: String,
        base_style: Option<String>,
        linked_style: Option<String>,
        next_paragraph_style: Option<String>,
        format: Option<Box<ParagraphStyleFormat>>,
        text: Option<Box<TextFormat>>,
    },
    Table {
        name: String,
        base_style: Option<String>,
        #[serde(default)]
        regions: Vec<RegionInput>,
    },
    Numbering {
        name: String,
        numbering_id: u32,
    },
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextFormat {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    font: Option<String>,
    font_size: Option<f64>,
    color: Option<[u8; 3]>,
    language: Option<RunLanguageInput>,
    bold_complex_script: Option<bool>,
    italic_complex_script: Option<bool>,
    size_complex_script: Option<f64>,
    rtl: Option<bool>,
    complex_script: Option<bool>,
    font_east_asia: Option<String>,
    font_complex_script: Option<String>,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParagraphStyleFormat {
    bidi: Option<bool>,
    alignment: Option<ParagraphAlignment>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    line_spacing: Option<f64>,
    line_height: Option<LineHeightInput>,
    left_indent: Option<f64>,
    right_indent: Option<f64>,
    first_line_indent: Option<f64>,
    keep_together: Option<bool>,
    keep_with_next: Option<bool>,
    page_break_before: Option<bool>,
    widow_control: Option<bool>,
    suppress_line_numbers: Option<bool>,
    contextual_spacing: Option<bool>,
    mirror_indents: Option<bool>,
    snap_to_grid: Option<bool>,
    #[serde(default)]
    tab_stops: Vec<TabStopInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegionInput {
    region: Region,
    format: RegionFormat,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegionFormat {
    text: Option<TextFormat>,
    background: Option<[u8; 3]>,
    bidi: Option<bool>,
    alignment: Option<ParagraphAlignment>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    line_height: Option<LineHeightInput>,
    #[serde(default)]
    tab_stops: Vec<TabStopInput>,
}

impl From<RegionFormat> for TableSectionFormat {
    fn from(value: RegionFormat) -> Self {
        let text = value.text.unwrap_or_default();
        Self {
            background: value.background,
            bidi: value.bidi,
            alignment: value.alignment.map(Into::into),
            spacing_before: value.spacing_before,
            spacing_after: value.spacing_after,
            line_height: value.line_height.map(Into::into),
            tab_stops: value.tab_stops.into_iter().map(Into::into).collect(),
            bold: text.bold,
            italic: text.italic,
            underline: text.underline,
            font: text.font,
            font_size: text.font_size,
            color: text.color,
            language: text.language.map(Into::into),
            bold_complex_script: text.bold_complex_script,
            italic_complex_script: text.italic_complex_script,
            size_complex_script: text.size_complex_script,
            rtl: text.rtl,
            complex_script: text.complex_script,
            font_east_asia: text.font_east_asia,
            font_complex_script: text.font_complex_script,
        }
    }
}

macro_rules! regions {
    ($($variant:ident = $key:literal => $member:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Deserialize, JsonSchema)]
        pub(crate) enum Region {
            $(#[serde(rename = $key)] $variant),+
        }

        impl Region {
            pub(crate) fn key(self) -> &'static str {
                match self { $(Self::$variant => $key),+ }
            }

            fn slot(self, table: &mut TableStyleDefinition) -> &mut Option<TableSectionFormat> {
                match self { $(Self::$variant => &mut table.$member),+ }
            }
        }
    };
}

regions!(
    WholeTable = "wholeTable" => whole_table,
    FirstRow = "firstRow" => first_row,
    LastRow = "lastRow" => last_row,
    FirstCol = "firstCol" => first_column,
    LastCol = "lastCol" => last_column,
    Band1Horz = "band1Horz" => banding_1_row,
    Band2Horz = "band2Horz" => banding_2_row,
    Band1Vert = "band1Vert" => banding_1_column,
    Band2Vert = "band2Vert" => banding_2_column,
    NwCell = "nwCell" => top_left_cell,
    NeCell = "neCell" => top_right_cell,
    SwCell = "swCell" => bottom_left_cell,
    SeCell = "seCell" => bottom_right_cell,
);

impl StyleInput {
    pub(crate) fn resolve(self) -> Result<StyleDefinition, String> {
        Ok(match self {
            Self::Character {
                name,
                base_style,
                linked_style,
                format,
            } => CharacterStyleDefinition {
                name,
                base_style,
                linked_style,
                format: format.map(|f| (*f).into()).unwrap_or_default(),
            }
            .into(),
            Self::Paragraph {
                name,
                base_style,
                linked_style,
                next_paragraph_style,
                format,
                text,
            } => {
                let p = format.unwrap_or_default();
                let text = text.unwrap_or_default();
                ParagraphStyleDefinition {
                    name,
                    base_style,
                    linked_style,
                    next_paragraph_style,
                    bidi: p.bidi,
                    alignment: p.alignment.map(Into::into),
                    spacing_before: p.spacing_before,
                    spacing_after: p.spacing_after,
                    line_spacing: p.line_spacing,
                    line_height: p.line_height.map(Into::into),
                    left_indent: p.left_indent,
                    right_indent: p.right_indent,
                    first_line_indent: p.first_line_indent,
                    keep_together: p.keep_together,
                    keep_with_next: p.keep_with_next,
                    page_break_before: p.page_break_before,
                    widow_control: p.widow_control,
                    suppress_line_numbers: p.suppress_line_numbers,
                    contextual_spacing: p.contextual_spacing,
                    mirror_indents: p.mirror_indents,
                    snap_to_grid: p.snap_to_grid,
                    tab_stops: p.tab_stops.into_iter().map(Into::into).collect(),
                    bold: text.bold,
                    italic: text.italic,
                    underline: text.underline,
                    font: text.font,
                    font_size: text.font_size,
                    color: text.color,
                    language: text.language.map(Into::into),
                    bold_complex_script: text.bold_complex_script,
                    italic_complex_script: text.italic_complex_script,
                    size_complex_script: text.size_complex_script,
                    rtl: text.rtl,
                    complex_script: text.complex_script,
                    font_east_asia: text.font_east_asia,
                    font_complex_script: text.font_complex_script,
                }
                .into()
            }
            Self::Table {
                name,
                base_style,
                regions,
            } => {
                let mut table = TableStyleDefinition {
                    name,
                    base_style,
                    ..Default::default()
                };
                for item in regions {
                    let slot = item.region.slot(&mut table);
                    if slot.is_some() {
                        return Err(format!(
                            "duplicate table style region {}",
                            item.region.key()
                        ));
                    }
                    *slot = Some(item.format.into());
                }
                table.into()
            }
            Self::Numbering { name, numbering_id } => {
                NumberingStyleDefinition { name, numbering_id }.into()
            }
        })
    }
}
