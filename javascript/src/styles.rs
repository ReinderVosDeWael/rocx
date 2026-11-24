//! A07–A08: declarations and projections of Rust style and numbering values.
use crate::{
    boundary::{self as b, Failure, Operation, Pass, Record},
    host::Wire,
    program::{self, Hooks},
};
use js_sys::Array;
use rocx::{
    CharacterStyleDefinition, Document, NumberingAlignment, NumberingDefinition, NumberingLevel,
    NumberingReference, NumberingStartOverride, NumberingStyleDefinition, NumberingSuffix,
    ParagraphStyleDefinition, ResolvedDocument, ResolvedParagraph, Result, StyleDefinition,
    StyleId, TableSectionFormat, TableStyleDefinition,
};
use wasm_bindgen::JsValue;

mod format;
use format::*;

pub(crate) const HOOKS: Hooks = Hooks {
    document_fields: &["styles", "numbering"],
    document,
    document_view,
    paragraph_view,
    ..Hooks::EMPTY
};

fn document(document: &mut Document, record: &Record, pass: &Pass) -> Result<()> {
    if let Some(styles) = program::optional(record, pass, "styles", styles)? {
        document.styles = styles;
    }
    if let Some(numbering) = program::optional(record, pass, "numbering", numbering)? {
        document.numbering = numbering;
    }
    Ok(())
}

fn document_view(document: &ResolvedDocument, output: &JsValue) {
    b::set(output, "styles", &list_view(document.styles(), style_view));
    b::set(
        output,
        "numbering",
        &list_view(document.numbering(), numbering_definition_view),
    );
}

fn paragraph_view(paragraph: &ResolvedParagraph, output: &JsValue) {
    if let Some(reference) = paragraph.numbering() {
        b::set(output, "numbering", &numbering_reference_view(reference));
    }
}

fn list<T>(input: &Wire, pass: &Pass, path: &str, decode: program::Decode<T>) -> Result<Vec<T>> {
    b::list(input, path)?
        .iter()
        .enumerate()
        .map(|(index, wire)| decode(wire, pass, &format!("{path}[{index}]")))
        .collect()
}

fn list_view<T>(values: &[T], view: fn(&T) -> JsValue) -> JsValue {
    values.iter().map(view).collect::<Array>().into()
}

fn styles(input: &Wire, pass: &Pass, path: &str) -> Result<Vec<StyleDefinition>> {
    list(input, pass, path, style_definition)
}

fn numbering(input: &Wire, pass: &Pass, path: &str) -> Result<Vec<NumberingDefinition>> {
    list(input, pass, path, numbering_definition)
}

pub(crate) fn style_definition(input: &Wire, pass: &Pass, path: &str) -> Result<StyleDefinition> {
    let kind_path = b::child_path(path, "kind");
    match b::string(&b::member(&input.js(), "kind", path)?, &kind_path)?.as_str() {
        "paragraph" => paragraph_style(input, pass, path).map(Into::into),
        "character" => character_style(input, pass, path).map(Into::into),
        "numbering" => numbering_style(input, pass, path).map(Into::into),
        "table" => table_style(input, pass, path).map(Into::into),
        _ => Err(Failure::invalid(&kind_path, "unknown style kind")),
    }
}

pub(crate) fn style_view(style: &StyleDefinition) -> JsValue {
    let (kind, view) = match style {
        StyleDefinition::Paragraph(value) => ("paragraph", paragraph_style_view(value)),
        StyleDefinition::Character(value) => ("character", character_style_view(value)),
        StyleDefinition::Numbering(value) => ("numbering", numbering_style_view(value)),
        StyleDefinition::Table(value) => ("table", table_style_view(value)),
    };
    b::set(&view, "kind", &kind.into());
    view
}

pub(crate) fn validate_numbering_definition(input: &Wire, path: &str) -> Result<JsValue> {
    let value = numbering_definition(input, &Operation::new().pass(), path)?;
    value.validate()?;
    Ok(JsValue::UNDEFINED)
}

pub(crate) fn validate_style_id(input: &Wire, path: &str) -> Result<JsValue> {
    StyleId::new(b::string(input, path)?)?;
    Ok(JsValue::UNDEFINED)
}

// Keep the accepted keys and their projections in one mapping. Defaults are only
// adapter conveniences for Rust default fields; explicit false and zero survive.
macro_rules! dto {
    ($decode:ident, $view:ident, $type:ident $(, $extra:literal)? {
        $($key:literal => $field:ident : $mode:ident($read:ident, $write:ident)),* $(,)?
    }) => {
        pub(crate) fn $decode(input: &Wire, pass: &Pass, path: &str) -> Result<$type> {
            let record = Record::new(input, path, &[$($extra,)? $($key),*])?;
            Ok($type {
                $($field: dto!(@read $mode, &record.get($key)?, pass, &record.path($key), $read)),*
            })
        }
        pub(crate) fn $view(value: &$type) -> JsValue {
            let result = b::record(&[]);
            $(dto!(@write $mode, &result, $key, &value.$field, $write);)*
            result
        }
    };
    (@read required, $wire:expr, $pass:expr, $path:expr, $decode:ident) => {
        $decode($wire, $pass, $path)?
    };
    (@read optional, $wire:expr, $pass:expr, $path:expr, $decode:ident) => {{
        let wire = $wire;
        (!wire.js().is_undefined()).then(|| $decode(wire, $pass, $path)).transpose()?
    }};
    (@read default, $wire:expr, $pass:expr, $path:expr, $decode:ident) => {
        dto!(@read optional, $wire, $pass, $path, $decode).unwrap_or_default()
    };
    (@read list, $wire:expr, $pass:expr, $path:expr, $decode:ident) => {{
        let wire = $wire;
        if wire.js().is_undefined() { Vec::new() } else { list(wire, $pass, $path, $decode)? }
    }};
    (@write required, $object:expr, $key:expr, $value:expr, $view:ident) => {
        b::set($object, $key, &$view($value))
    };
    (@write default, $object:expr, $key:expr, $value:expr, $view:ident) => {
        dto!(@write required, $object, $key, $value, $view)
    };
    (@write optional, $object:expr, $key:expr, $value:expr, $view:ident) => {
        if let Some(value) = $value { b::set($object, $key, &$view(value)); }
    };
    (@write list, $object:expr, $key:expr, $value:expr, $view:ident) => {
        b::set($object, $key, &list_view($value, $view))
    };
}

fn text_view(value: &str) -> JsValue {
    value.into()
}

fn boolean(input: &Wire, _: &Pass, path: &str) -> Result<bool> {
    input
        .js()
        .as_bool()
        .ok_or_else(|| Failure::invalid(path, "expected a boolean"))
}

fn boolean_view(value: &bool) -> JsValue {
    (*value).into()
}

fn number(input: &Wire, _: &Pass, path: &str) -> Result<f64> {
    input
        .js()
        .as_f64()
        .ok_or_else(|| Failure::invalid(path, "expected a number"))
}

fn number_view(value: &f64) -> JsValue {
    (*value).into()
}

fn uint<T: TryFrom<u64>>(input: &Wire, _: &Pass, path: &str) -> Result<T> {
    T::try_from(b::u64_number(input, path)?)
        .map_err(|_| Failure::invalid(path, "integer outside field bounds"))
}

fn uint_view<T: Copy + Into<f64>>(value: &T) -> JsValue {
    Into::<f64>::into(*value).into()
}

macro_rules! string_enum {
    ($decode:ident, $view:ident, $type:ident { $($key:literal => $variant:ident),+ $(,)? }) => {
        fn $decode(input: &Wire, _: &Pass, path: &str) -> Result<$type> {
            match b::string(input, path)?.as_str() {
                $($key => Ok($type::$variant),)+
                _ => Err(Failure::invalid(path, concat!("unknown ", stringify!($type)))),
            }
        }
        fn $view(value: &$type) -> JsValue {
            match value { $($type::$variant => $key,)+ }.into()
        }
    }
}

string_enum!(numbering_alignment, numbering_alignment_view, NumberingAlignment {
    "left" => Left, "center" => Center, "right" => Right, "start" => Start, "end" => End,
});
string_enum!(numbering_suffix, numbering_suffix_view, NumberingSuffix {
    "tab" => Tab, "space" => Space, "nothing" => Nothing,
});

use b::text;

pub(crate) fn optional_numbering_reference(
    input: &Wire,
    pass: &Pass,
    path: &str,
) -> Result<Option<NumberingReference>> {
    if input.js().is_undefined() {
        Ok(None)
    } else {
        numbering_reference(input, pass, path).map(Some)
    }
}

dto!(numbering_reference, numbering_reference_view, NumberingReference {
    "id" => id: required(uint, uint_view),
    "level" => level: required(uint, uint_view),
});
dto!(numbering_definition, numbering_definition_view, NumberingDefinition {
    "id" => id: required(uint, uint_view),
    "abstractId" => abstract_id: optional(uint, uint_view),
    "inheritStyle" => inherit_style: optional(text, text_view),
    "levels" => levels: list(numbering_level, numbering_level_view),
    "startOverrides" => start_overrides: list(start_override, start_override_view),
    "levelOverrides" => level_overrides: list(numbering_level, numbering_level_view),
});
dto!(numbering_level, numbering_level_view, NumberingLevel {
    "level" => level: required(uint, uint_view),
    "start" => start: required(uint, uint_view),
    "format" => format: required(text, text_view),
    "text" => text: required(text, text_view),
    "isLegal" => is_legal: optional(boolean, boolean_view),
    "suffix" => suffix: optional(numbering_suffix, numbering_suffix_view),
    "restart" => restart: optional(uint, uint_view),
    "alignment" => alignment: optional(numbering_alignment, numbering_alignment_view),
    "labelFormat" => label_format: optional(run_format, run_format_view),
    "paragraphFormat" => paragraph_format: optional(paragraph_format, paragraph_format_view),
    "paragraphStyle" => paragraph_style: optional(text, text_view),
});
dto!(start_override, start_override_view, NumberingStartOverride {
    "level" => level: required(uint, uint_view),
    "start" => start: required(uint, uint_view),
});

pub(crate) fn imported_numbering_level(l: &rocx::editor::ImportedNumberingLevel) -> JsValue {
    let result = b::record(&[
        ("level", l.level.into()),
        ("hasOpaqueContent", l.has_opaque_content.into()),
    ]);
    macro_rules! optional { ($(($key:literal, $value:expr)),+ $(,)?) => {$(
        if let Some(value) = $value { b::set(&result, $key, &value); }
    )+}; }
    optional!(
        ("start", l.start.map(Into::into)),
        ("format", l.format.as_deref().map(Into::into)),
        ("text", l.text.as_deref().map(Into::into)),
        ("isLegal", l.is_legal.map(Into::into)),
        ("restart", l.restart.map(Into::into)),
        ("suffix", l.suffix.as_ref().map(numbering_suffix_view)),
        (
            "alignment",
            l.alignment.as_ref().map(numbering_alignment_view)
        ),
        (
            "paragraphStyle",
            l.paragraph_style.as_deref().map(Into::into)
        ),
        ("labelFormat", l.label_format.as_ref().map(run_format_view)),
        (
            "paragraphFormat",
            l.paragraph_format.as_ref().map(paragraph_format_view)
        ),
    );
    result
}
dto!(paragraph_style, paragraph_style_view, ParagraphStyleDefinition, "kind" {
    "name" => name: required(text, text_view),
    "baseStyle" => base_style: optional(text, text_view),
    "nextParagraphStyle" => next_paragraph_style: optional(text, text_view),
    "linkedStyle" => linked_style: optional(text, text_view),
    "bidi" => bidi: optional(boolean, boolean_view),
    "bold" => bold: optional(boolean, boolean_view),
    "italic" => italic: optional(boolean, boolean_view),
    "underline" => underline: optional(boolean, boolean_view),
    "font" => font: optional(text, text_view),
    "fontSize" => font_size: optional(number, number_view),
    "color" => color: optional(rgb, rgb_view),
    "alignment" => alignment: optional(alignment, alignment_view),
    "spacingBefore" => spacing_before: optional(number, number_view),
    "spacingAfter" => spacing_after: optional(number, number_view),
    "lineSpacing" => line_spacing: optional(number, number_view),
    "lineHeight" => line_height: optional(line_height, line_height_view),
    "leftIndent" => left_indent: optional(number, number_view),
    "rightIndent" => right_indent: optional(number, number_view),
    "firstLineIndent" => first_line_indent: optional(number, number_view),
    "keepTogether" => keep_together: optional(boolean, boolean_view),
    "keepWithNext" => keep_with_next: optional(boolean, boolean_view),
    "pageBreakBefore" => page_break_before: optional(boolean, boolean_view),
    "widowControl" => widow_control: optional(boolean, boolean_view),
    "suppressLineNumbers" => suppress_line_numbers: optional(boolean, boolean_view),
    "contextualSpacing" => contextual_spacing: optional(boolean, boolean_view),
    "mirrorIndents" => mirror_indents: optional(boolean, boolean_view),
    "snapToGrid" => snap_to_grid: optional(boolean, boolean_view),
    "tabStops" => tab_stops: list(tab_stop, tab_stop_view),
    "language" => language: optional(run_language, run_language_view),
    "boldComplexScript" => bold_complex_script: optional(boolean, boolean_view),
    "italicComplexScript" => italic_complex_script: optional(boolean, boolean_view),
    "sizeComplexScript" => size_complex_script: optional(number, number_view),
    "rtl" => rtl: optional(boolean, boolean_view),
    "complexScript" => complex_script: optional(boolean, boolean_view),
    "fontEastAsia" => font_east_asia: optional(text, text_view),
    "fontComplexScript" => font_complex_script: optional(text, text_view),
});
dto!(character_style, character_style_view, CharacterStyleDefinition, "kind" {
    "name" => name: required(text, text_view),
    "baseStyle" => base_style: optional(text, text_view),
    "linkedStyle" => linked_style: optional(text, text_view),
    "format" => format: default(run_format, run_format_view),
});
dto!(numbering_style, numbering_style_view, NumberingStyleDefinition, "kind" {
    "name" => name: required(text, text_view),
    "numberingId" => numbering_id: required(uint, uint_view),
});
dto!(table_region, table_region_view, TableSectionFormat {
    "bold" => bold: optional(boolean, boolean_view),
    "italic" => italic: optional(boolean, boolean_view),
    "underline" => underline: optional(boolean, boolean_view),
    "font" => font: optional(text, text_view),
    "fontSize" => font_size: optional(number, number_view),
    "color" => color: optional(rgb, rgb_view),
    "background" => background: optional(rgb, rgb_view),
    "alignment" => alignment: optional(alignment, alignment_view),
    "spacingBefore" => spacing_before: optional(number, number_view),
    "spacingAfter" => spacing_after: optional(number, number_view),
    "lineHeight" => line_height: optional(line_height, line_height_view),
    "tabStops" => tab_stops: list(tab_stop, tab_stop_view),
    "bidi" => bidi: optional(boolean, boolean_view),
    "language" => language: optional(run_language, run_language_view),
    "boldComplexScript" => bold_complex_script: optional(boolean, boolean_view),
    "italicComplexScript" => italic_complex_script: optional(boolean, boolean_view),
    "sizeComplexScript" => size_complex_script: optional(number, number_view),
    "rtl" => rtl: optional(boolean, boolean_view),
    "complexScript" => complex_script: optional(boolean, boolean_view),
    "fontEastAsia" => font_east_asia: optional(text, text_view),
    "fontComplexScript" => font_complex_script: optional(text, text_view),
});
dto!(table_style, table_style_view, TableStyleDefinition, "kind" {
    "name" => name: required(text, text_view),
    "baseStyle" => base_style: optional(text, text_view),
    "wholeTable" => whole_table: optional(table_region, table_region_view),
    "firstRow" => first_row: optional(table_region, table_region_view),
    "lastRow" => last_row: optional(table_region, table_region_view),
    "firstColumn" => first_column: optional(table_region, table_region_view),
    "lastColumn" => last_column: optional(table_region, table_region_view),
    "banding1Row" => banding_1_row: optional(table_region, table_region_view),
    "banding2Row" => banding_2_row: optional(table_region, table_region_view),
    "banding1Column" => banding_1_column: optional(table_region, table_region_view),
    "banding2Column" => banding_2_column: optional(table_region, table_region_view),
    "topLeftCell" => top_left_cell: optional(table_region, table_region_view),
    "topRightCell" => top_right_cell: optional(table_region, table_region_view),
    "bottomLeftCell" => bottom_left_cell: optional(table_region, table_region_view),
    "bottomRightCell" => bottom_right_cell: optional(table_region, table_region_view),
});
