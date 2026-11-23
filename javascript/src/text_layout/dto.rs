//! Structural conversions only; semantic checks belong to `rocx`.
use crate::{
    boundary::{self as b, Failure, Record},
    host::Wire,
};
use js_sys::{Array, Object, Reflect};
use rocx::*;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

type Read<T> = fn(&Wire, &str) -> Result<T>;

pub(super) fn optional<T>(record: &Record, key: &str, read: Read<T>) -> Result<Option<T>> {
    let value = record.get(key)?;
    if value.js().is_undefined() {
        Ok(None)
    } else {
        read(&value, &record.path(key)).map(Some)
    }
}

pub(super) fn bool_(value: &Wire, path: &str) -> Result<bool> {
    value
        .js()
        .as_bool()
        .ok_or_else(|| Failure::invalid(path, "expected a boolean"))
}

fn number(value: &Wire, path: &str) -> Result<f64> {
    value
        .js()
        .as_f64()
        .ok_or_else(|| Failure::invalid(path, "expected a number"))
}

fn uint<T: TryFrom<u64>>(value: &Wire, path: &str) -> Result<T> {
    T::try_from(b::u64_number(value, path)?)
        .map_err(|_| Failure::invalid(path, "integer outside field bounds"))
}

fn int32(value: &Wire, path: &str) -> Result<i32> {
    let value = number(value, path)?;
    if value.fract() == 0.0 && value >= i32::MIN as f64 && value <= i32::MAX as f64 {
        Ok(value as i32)
    } else {
        Err(Failure::invalid(path, "expected a signed 32-bit integer"))
    }
}

fn scalar<T: Copy + Into<JsValue>>(value: &T) -> JsValue {
    (*value).into()
}

fn string_view(value: &str) -> JsValue {
    value.into()
}

fn emu_view(value: &i64) -> JsValue {
    if value.unsigned_abs() <= 9_007_199_254_740_991 {
        (*value as f64).into()
    } else {
        (*value).into()
    }
}

macro_rules! tokens {
    ($decode:ident, $view:ident, $ty:ident { $($variant:ident => $token:literal),+ $(,)? }) => {
        pub(crate) fn $decode(value: &Wire, path: &str) -> Result<$ty> {
            match b::string(value, path)?.as_str() {
                $($token => Ok($ty::$variant),)+
                _ => Err(Failure::invalid(path, concat!("unknown ", stringify!($ty), " variant"))),
            }
        }
        pub(crate) fn $view(value: &$ty) -> JsValue {
            match value { $($ty::$variant => $token.into(),)+ }
        }
    };
}

macro_rules! dto {
    ($decode:ident, $view:ident, $ty:ident {
        $($field:ident : $key:literal => $read:path, $project:path; )*
        $(; $($direct:ident : $direct_key:literal => $direct_read:path, $direct_project:path; )*)?
    }) => {
        pub(crate) fn $decode(value: &Wire, path: &str) -> Result<$ty> {
            let record = Record::new(value, path, &[$($key,)* $($($direct_key,)*)?])?;
            Ok($ty {
                $($field: optional(&record, $key, $read)?,)*
                $($($direct: optional(&record, $direct_key, $direct_read)?.unwrap_or_default(),)*)?
            })
        }
        pub(crate) fn $view(value: &$ty) -> JsValue {
            let result = b::record(&[]);
            $(if let Some(field) = &value.$field {
                b::set(&result, $key, &$project(field));
            })*
            $($(b::set(&result, $direct_key, &$direct_project(&value.$direct));)*)?
            result
        }
    };
}

tokens!(alignment, alignment_view, Alignment {
    Left => "left", Center => "center", Right => "right", Justify => "justify",
    Distribute => "distribute", JustifyMedium => "justifyMedium", JustifyHigh => "justifyHigh",
    JustifyLow => "justifyLow", ThaiJustify => "thaiJustify",
});
tokens!(tab_alignment, tab_alignment_view, TabAlignment {
    Left => "left", Center => "center", Right => "right", Decimal => "decimal",
    Bar => "bar", Clear => "clear", Start => "start", End => "end",
});
tokens!(tab_leader, tab_leader_view, TabLeader {
    None => "none", Dot => "dot", Hyphen => "hyphen", Underscore => "underscore",
    Heavy => "heavy", MiddleDot => "middleDot",
});
tokens!(underline_token, underline_view, Underline {
    None => "none", Single => "single", Double => "double", Dotted => "dotted",
    Dash => "dash", Wave => "wave", Words => "words",
});
tokens!(theme_color, theme_color_view, ThemeColor {
    Dark1 => "dark1", Light1 => "light1", Dark2 => "dark2", Light2 => "light2",
    Accent1 => "accent1", Accent2 => "accent2", Accent3 => "accent3", Accent4 => "accent4",
    Accent5 => "accent5", Accent6 => "accent6", Hyperlink => "hyperlink",
    FollowedHyperlink => "followedHyperlink", Background1 => "background1", Text1 => "text1",
    Background2 => "background2", Text2 => "text2",
});

pub(crate) fn rgb(value: &Wire, path: &str) -> Result<[u8; 3]> {
    let list = b::list(value, path)?;
    let [red, green, blue] = list.as_slice() else {
        return Err(Failure::invalid(
            path,
            "expected exactly three RGB channels",
        ));
    };
    Ok([
        uint(red, &format!("{path}[0]"))?,
        uint(green, &format!("{path}[1]"))?,
        uint(blue, &format!("{path}[2]"))?,
    ])
}

pub(crate) fn rgb_view(value: &[u8; 3]) -> JsValue {
    Array::of3(&value[0].into(), &value[1].into(), &value[2].into()).into()
}

fn color(value: &Wire, path: &str) -> Result<Color> {
    if Array::is_array(&value.js()) {
        return rgb(value, path).map(Color::Rgb);
    }
    if value.js().is_string() {
        return match b::string(value, path)?.as_str() {
            "auto" => Ok(Color::Auto),
            _ => Err(Failure::invalid(
                path,
                "expected automatic color, RGB or a theme color",
            )),
        };
    }
    let record = Record::new(value, path, &["kind", "name", "tint", "shade"])?;
    if b::string(&record.get("kind")?, &record.path("kind"))? != "theme" {
        return Err(Failure::invalid(
            &record.path("kind"),
            "expected a theme color",
        ));
    }
    Ok(Color::Theme {
        name: theme_color(&record.get("name")?, &record.path("name"))?,
        tint: optional(&record, "tint", uint)?,
        shade: optional(&record, "shade", uint)?,
    })
}

fn color_view(value: &Color) -> JsValue {
    match value {
        Color::Rgb(channels) => rgb_view(channels),
        Color::Auto => "auto".into(),
        Color::Theme { name, tint, shade } => {
            let result = b::record(&[("kind", "theme".into()), ("name", theme_color_view(name))]);
            for (key, value) in [("tint", tint), ("shade", shade)] {
                if let Some(value) = value {
                    b::set(&result, key, &(*value).into());
                }
            }
            result
        }
    }
}

fn underline(value: &Wire, path: &str) -> Result<Underline> {
    if let Some(value) = value.js().as_bool() {
        Ok(value.into())
    } else {
        underline_token(value, path)
    }
}

pub(crate) fn line_height(value: &Wire, path: &str) -> Result<LineHeight> {
    let record = Record::new(value, path, &["kind", "value"])?;
    let amount = number(&record.get("value")?, &record.path("value"))?;
    match b::string(&record.get("kind")?, &record.path("kind"))?.as_str() {
        "exact" => Ok(LineHeight::Exact(amount)),
        "minimum" => Ok(LineHeight::AtLeast(amount)),
        "multiplier" => Ok(LineHeight::Multiple(amount)),
        _ => Err(Failure::invalid(
            &record.path("kind"),
            "unknown line height variant",
        )),
    }
}

pub(crate) fn line_height_view(value: &LineHeight) -> JsValue {
    let (kind, amount) = match value {
        LineHeight::Exact(amount) => ("exact", amount),
        LineHeight::AtLeast(amount) => ("minimum", amount),
        LineHeight::Multiple(amount) => ("multiplier", amount),
    };
    b::record(&[("kind", kind.into()), ("value", (*amount).into())])
}

dto!(tab_stop, tab_stop_view, TabStop {
    leader: "leader" => tab_leader, tab_leader_view;
    ;
    alignment: "alignment" => tab_alignment, tab_alignment_view;
    position: "position" => number, scalar;
});

pub(crate) fn tab_stops(value: &Wire, path: &str) -> Result<Vec<TabStop>> {
    b::list(value, path)?
        .iter()
        .enumerate()
        .map(|(index, item)| tab_stop(item, &format!("{path}[{index}]")))
        .collect()
}

pub(crate) fn tab_stops_view(value: &[TabStop]) -> JsValue {
    value.iter().map(tab_stop_view).collect::<Array>().into()
}

dto!(run_language, run_language_view, RunLanguage {
    latin: "latin" => b::string, string_view;
    east_asia: "eastAsia" => b::string, string_view;
    bidi: "bidi" => b::string, string_view;
});

dto!(run_format, run_format_view, RunFormat {
    style: "style" => b::string, string_view;
    bold: "bold" => bool_, scalar;
    italic: "italic" => bool_, scalar;
    underline: "underline" => underline, underline_view;
    font: "font" => b::string, string_view;
    size: "size" => number, scalar;
    color: "color" => color, color_view;
    superscript: "superscript" => bool_, scalar;
    subscript: "subscript" => bool_, scalar;
    strike: "strike" => bool_, scalar;
    all_caps: "allCaps" => bool_, scalar;
    small_caps: "smallCaps" => bool_, scalar;
    snap_to_grid: "snapToGrid" => bool_, scalar;
    language: "language" => run_language, run_language_view;
    bold_complex_script: "boldComplexScript" => bool_, scalar;
    italic_complex_script: "italicComplexScript" => bool_, scalar;
    size_complex_script: "sizeComplexScript" => number, scalar;
    rtl: "rtl" => bool_, scalar;
    complex_script: "complexScript" => bool_, scalar;
    font_east_asia: "fontEastAsia" => b::string, string_view;
    font_complex_script: "fontComplexScript" => b::string, string_view;
});

dto!(paragraph_format, paragraph_format_view, ParagraphFormat {
    bidi: "bidi" => bool_, scalar;
    heading: "heading" => uint, scalar;
    style: "style" => b::string, string_view;
    alignment: "alignment" => alignment, alignment_view;
    spacing_before: "spacingBefore" => number, scalar;
    spacing_after: "spacingAfter" => number, scalar;
    line_spacing: "lineSpacing" => number, scalar;
    line_height: "lineHeight" => line_height, line_height_view;
    left_indent: "leftIndent" => number, scalar;
    right_indent: "rightIndent" => number, scalar;
    first_line_indent: "firstLineIndent" => number, scalar;
    keep_together: "keepTogether" => bool_, scalar;
    keep_with_next: "keepWithNext" => bool_, scalar;
    page_break_before: "pageBreakBefore" => bool_, scalar;
    widow_control: "widowControl" => bool_, scalar;
    suppress_line_numbers: "suppressLineNumbers" => bool_, scalar;
    contextual_spacing: "contextualSpacing" => bool_, scalar;
    mirror_indents: "mirrorIndents" => bool_, scalar;
    snap_to_grid: "snapToGrid" => bool_, scalar;
    ;
    tab_stops: "tabStops" => tab_stops, tab_stops_view;
});

tokens!(orientation, orientation_view, Orientation {
    Portrait => "portrait", Landscape => "landscape",
});
tokens!(vertical_alignment, vertical_alignment_view, VerticalAlignment {
    Top => "top", Center => "center", Bottom => "bottom", Both => "both",
});
tokens!(section_type, section_type_view, SectionType {
    NextPage => "nextPage", NextColumn => "nextColumn", Continuous => "continuous",
    EvenPage => "evenPage", OddPage => "oddPage",
});
tokens!(line_restart, line_restart_view, LineNumberRestart {
    NewPage => "newPage", NewSection => "newSection", Continuous => "continuous",
});
tokens!(border_line, border_line_view, BorderLine {
    Single => "single", Dashed => "dashed",
});
tokens!(border_offset, border_offset_view, PageBorderOffset { Page => "page", Text => "text" });
tokens!(border_display, border_display_view, PageBorderDisplay {
    AllPages => "allPages", FirstPage => "firstPage", NotFirstPage => "notFirstPage",
});
tokens!(border_z_order, border_z_order_view, PageBorderZOrder { Front => "front", Back => "back" });
tokens!(grid_mode, grid_mode_view, DocumentGridMode {
    Default => "default", Lines => "lines", LinesAndChars => "linesAndChars",
    SnapToChars => "snapToChars",
});

dto!(page_size, page_size_view, PageSize {
    width: "width" => b::i64_number, emu_view;
    height: "height" => b::i64_number, emu_view;
});
dto!(page_margins, page_margins_view, PageMargins {
    top: "top" => b::i64_number, emu_view;
    bottom: "bottom" => b::i64_number, emu_view;
    left: "left" => b::i64_number, emu_view;
    right: "right" => b::i64_number, emu_view;
    header: "header" => b::i64_number, emu_view;
    footer: "footer" => b::i64_number, emu_view;
});
dto!(page_numbering, page_numbering_view, PageNumbering {
    start: "start" => uint, scalar;
    format: "format" => b::string, string_view;
});
dto!(line_numbering, line_numbering_view, LineNumbering {
    count_by: "countBy" => uint, scalar;
    start: "start" => uint, scalar;
    distance: "distance" => uint, scalar;
    restart: "restart" => line_restart, line_restart_view;
    ;
    clear: "clear" => bool_, scalar;
});
dto!(columns, columns_view, Columns {
    count: "count" => uint, scalar;
    space: "space" => uint, scalar;
    separator: "separator" => bool_, scalar;
});

fn page_border(value: &Wire, path: &str) -> Result<PageBorder> {
    let record = Record::new(value, path, &["style", "size", "space", "color"])?;
    Ok(PageBorder {
        style: border_line(&record.get("style")?, &record.path("style"))?,
        size: optional(&record, "size", uint)?,
        space: optional(&record, "space", uint)?,
        color: optional(&record, "color", rgb)?,
    })
}
fn page_border_view(value: &PageBorder) -> JsValue {
    let result = b::record(&[("style", border_line_view(&value.style))]);
    if let Some(size) = value.size {
        b::set(&result, "size", &size.into());
    }
    if let Some(space) = value.space {
        b::set(&result, "space", &space.into());
    }
    if let Some(color) = value.color {
        b::set(&result, "color", &rgb_view(&color));
    }
    result
}
dto!(page_borders, page_borders_view, PageBorders {
    offset_from: "offsetFrom" => border_offset, border_offset_view;
    display: "display" => border_display, border_display_view;
    z_order: "zOrder" => border_z_order, border_z_order_view;
    top: "top" => page_border, page_border_view;
    left: "left" => page_border, page_border_view;
    bottom: "bottom" => page_border, page_border_view;
    right: "right" => page_border, page_border_view;
});
dto!(document_grid, document_grid_view, DocumentGrid {
    mode: "mode" => grid_mode, grid_mode_view;
    line_pitch: "linePitch" => uint, scalar;
    character_pitch_delta: "characterPitchDelta" => int32, scalar;
});
dto!(section_properties, section_properties_view, SectionProperties {
    page_size: "pageSize" => page_size, page_size_view;
    page_orientation: "pageOrientation" => orientation, orientation_view;
    page_margins: "pageMargins" => page_margins, page_margins_view;
    page_borders: "pageBorders" => page_borders, page_borders_view;
    document_grid: "documentGrid" => document_grid, document_grid_view;
    page_numbering: "pageNumbering" => page_numbering, page_numbering_view;
    line_numbering: "lineNumbering" => line_numbering, line_numbering_view;
    columns: "columns" => columns, columns_view;
    vertical_align: "verticalAlign" => vertical_alignment, vertical_alignment_view;
    title_page: "titlePage" => bool_, scalar;
    r#type: "type" => section_type, section_type_view;
    bidi: "bidi" => bool_, scalar;
    rtl_gutter: "rtlGutter" => bool_, scalar;
});

tokens!(footnote_position, footnote_position_view, FootnotePosition {
    PageBottom => "pageBottom", BeneathText => "beneathText",
});
tokens!(endnote_position, endnote_position_view, EndnotePosition {
    SectionEnd => "sectionEnd", DocumentEnd => "documentEnd",
});
tokens!(note_restart, note_restart_view, NoteNumberRestart {
    Continuous => "continuous", EachSection => "eachSection", EachPage => "eachPage",
});
tokens!(note_format, note_format_view, NoteNumberFormat {
    Decimal => "decimal", LowerRoman => "lowerRoman", UpperRoman => "upperRoman",
    LowerLetter => "lowerLetter", UpperLetter => "upperLetter",
});
dto!(note_numbering, note_numbering_view, NoteNumbering {
    format: "format" => note_format, note_format_view;
    start: "start" => uint, scalar;
    restart: "restart" => note_restart, note_restart_view;
});
dto!(document_defaults, document_defaults_view, DocumentDefaults {
    run: "run" => run_format, run_format_view;
    paragraph: "paragraph" => paragraph_format, paragraph_format_view;
    footnote_numbering: "footnoteNumbering" => note_numbering, note_numbering_view;
    endnote_numbering: "endnoteNumbering" => note_numbering, note_numbering_view;
    footnote_position: "footnotePosition" => footnote_position, footnote_position_view;
    endnote_position: "endnotePosition" => endnote_position, endnote_position_view;
    mirror_margins: "mirrorMargins" => bool_, scalar;
    gutter_at_top: "gutterAtTop" => bool_, scalar;
    track_revisions: "trackRevisions" => bool_, scalar;
});

fn custom_map<T>(value: &Wire, path: &str, read: Read<T>) -> Result<BTreeMap<String, T>> {
    let object = value.js();
    if !object.is_object() || object.is_null() || object.is_function() || Array::is_array(&object) {
        return Err(Failure::invalid(path, "expected a named property record"));
    }
    Reflect::own_keys(&object)
        .map_err(|cause| Failure::host("cannot read custom property keys", cause))?
        .iter()
        .map(|key| {
            let name = b::string(&Wire::new(key), path)?;
            let value = b::member(&object, &name, path)?;
            let result = read(&value, &b::child_path(path, &name))?;
            Ok((name, result))
        })
        .collect()
}

fn custom_map_view<T>(map: &BTreeMap<String, T>, project: fn(&T) -> JsValue) -> JsValue {
    let object = Object::new();
    for (name, value) in map {
        // Define own data properties; "__proto__" is a valid custom name.
        Object::define_property(
            &object,
            &name.as_str().into(),
            &b::record(&[
                ("value", project(value)),
                ("enumerable", true.into()),
                ("writable", true.into()),
            ])
            .unchecked_into(),
        );
    }
    object.into()
}

fn string_map(value: &Wire, path: &str) -> Result<BTreeMap<String, String>> {
    custom_map(value, path, b::string)
}
fn boolean_map(value: &Wire, path: &str) -> Result<BTreeMap<String, bool>> {
    custom_map(value, path, bool_)
}
fn string_map_view(value: &BTreeMap<String, String>) -> JsValue {
    custom_map_view(value, |value| string_view(value))
}
fn boolean_map_view(value: &BTreeMap<String, bool>) -> JsValue {
    custom_map_view(value, scalar)
}

dto!(metadata, metadata_view, Metadata {
    creator: "creator" => b::string, string_view;
    title: "title" => b::string, string_view;
    subject: "subject" => b::string, string_view;
    description: "description" => b::string, string_view;
    keywords: "keywords" => b::string, string_view;
    category: "category" => b::string, string_view;
    comments: "comments" => b::string, string_view;
    version: "version" => b::string, string_view;
    company: "company" => b::string, string_view;
    manager: "manager" => b::string, string_view;
    ;
    custom_properties: "customProperties" => string_map, string_map_view;
    custom_boolean_properties: "customBooleanProperties" => boolean_map, boolean_map_view;
});
