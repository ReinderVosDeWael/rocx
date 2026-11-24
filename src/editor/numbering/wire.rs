//! Direct-readback JSON shape shared by the CLI and Python adapter.
use super::*;
use crate::{Color, LineHeight, ParagraphFormat, RunFormat};
use serde::Serialize;
use serde_json::{Value, json};

macro_rules! optional {
    ($object:ident, $from:ident, $($field:ident),+ $(,)?) => {$(
        if let Some(value) = &$from.$field {
            $object[stringify!($field)] = json!(value);
        }
    )+};
}

fn run(f: &RunFormat) -> Value {
    let mut result = json!({});
    optional!(
        result,
        f,
        style,
        bold,
        italic,
        font,
        size,
        superscript,
        subscript,
        strike,
        all_caps,
        small_caps,
        snap_to_grid,
        bold_complex_script,
        italic_complex_script,
        size_complex_script,
        rtl,
        complex_script,
        font_east_asia,
        font_complex_script
    );
    if let Some(value) = f.underline {
        result["underline"] = json!(value.xml());
    }
    if let Some(lang) = &f.language {
        let mut language = json!({});
        optional!(language, lang, latin, east_asia, bidi);
        result["language"] = language;
    }
    if let Some(value) = f.color {
        result["color"] = match value {
            Color::Auto => json!("auto"),
            Color::Rgb(rgb) => json!(rgb),
            Color::Theme { name, tint, shade } => {
                let mut color = json!({"theme": name.xml()});
                if let Some(tint) = tint {
                    color["tint"] = json!(tint);
                }
                if let Some(shade) = shade {
                    color["shade"] = json!(shade);
                }
                color
            }
        };
    }
    result
}

fn paragraph(f: &ParagraphFormat) -> Value {
    let mut result = json!({});
    optional!(
        result,
        f,
        bidi,
        heading,
        style,
        spacing_before,
        spacing_after,
        line_spacing,
        left_indent,
        right_indent,
        first_line_indent,
        keep_together,
        keep_with_next,
        page_break_before,
        widow_control,
        suppress_line_numbers,
        contextual_spacing,
        mirror_indents,
        snap_to_grid
    );
    if let Some(value) = f.alignment {
        result["alignment"] = json!(value.xml());
    }
    if let Some(value) = f.line_height {
        let (kind, value) = match value {
            LineHeight::Multiple(v) => ("multiple", v),
            LineHeight::Exact(v) => ("exact", v),
            LineHeight::AtLeast(v) => ("at-least", v),
        };
        result["line_height"] = json!({"kind":kind,"value":value});
    }
    result["tab_stops"] = json!(
        f.tab_stops
            .iter()
            .map(|t| {
                let mut tab = json!({"position":t.position,"alignment":t.alignment.xml()});
                if let Some(leader) = t.leader {
                    tab["leader"] = json!(leader.xml());
                }
                tab
            })
            .collect::<Vec<_>>()
    );
    result
}

fn level(f: &ImportedNumberingLevel) -> Value {
    let mut result = json!({"level":f.level,"has_opaque_content":f.has_opaque_content});
    optional!(
        result,
        f,
        start,
        format,
        text,
        restart,
        is_legal,
        paragraph_style
    );
    if let Some(value) = f.suffix {
        result["suffix"] = json!(value.xml());
    }
    if let Some(value) = f.alignment {
        result["alignment"] = json!(value.xml());
    }
    if let Some(f) = &f.label_format {
        result["label_format"] = run(f);
    }
    if let Some(f) = &f.paragraph_format {
        result["paragraph_format"] = paragraph(f);
    }
    result
}

impl Serialize for ImportedNumberingCatalog {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let abstracts = self
            .abstracts
            .iter()
            .map(|a| {
                let mut result = json!({"abstract_id":a.abstract_id,
                "levels":a.levels.iter().map(level).collect::<Vec<_>>()});
                optional!(result, a, style_link, num_style_link);
                result
            })
            .collect::<Vec<_>>();
        let instances = self.instances.iter().map(|i| json!({
            "num_id":i.num_id, "abstract_id":i.abstract_id, "style_ids":i.style_ids,
            "start_overrides":i.start_overrides.iter().map(|s| json!({"level":s.level,"start":s.start})).collect::<Vec<_>>(),
            "level_overrides":i.level_overrides.iter().map(level).collect::<Vec<_>>(),
        })).collect::<Vec<_>>();
        json!({"abstracts":abstracts, "instances":instances}).serialize(serializer)
    }
}
