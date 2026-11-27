use rocx::{
    Alignment, BreakType, Color, FieldForm, LineHeight, ParagraphFormat, ResolvedInline, Rule,
    RunFormat, RunLanguage, TabAlignment, TabLeader, TabStop, ThemeColor, Underline,
};
use serde_json::{Value, json};

use super::*;
use crate::schema::ToolKind;

#[test]
fn format_payloads_preserve_all_direct_properties_and_explicit_false() {
    let paragraph: ParagraphInput = serde_json::from_value(json!({
        "inlines": [{
            "kind": "text", "text": "First", "format": {
                "style": "Emphasis", "bold": false, "italic": true, "underline": "double",
                "font": "Latin", "size": 11.5, "color": {"theme": "accent2", "shade": 150},
                "superscript": true, "subscript": false, "strike": true,
                "all_caps": false, "small_caps": true, "snap_to_grid": false,
                "language": {"latin": "en-US", "east_asia": "ja-JP", "bidi": "ar-SA"},
                "bold_complex_script": true, "italic_complex_script": false,
                "size_complex_script": 14, "rtl": false, "complex_script": true,
                "font_east_asia": "East Asia", "font_complex_script": "Complex"
            }
        }, {"kind": "text", "text": " second"}],
        "format": {
            "bidi": true, "heading": 2, "style": "Normal", "alignment": "justify-medium",
            "spacing_before": 4, "spacing_after": 6, "line_height": {"kind":"at-least","value":12},
            "left_indent": -3, "right_indent": 8, "first_line_indent": -2,
            "keep_together": true, "keep_with_next": false, "page_break_before": true,
            "widow_control": false, "suppress_line_numbers": true, "contextual_spacing": false,
            "mirror_indents": true, "snap_to_grid": false,
            "tab_stops": [{"position": -9, "alignment": "decimal", "leader": "middleDot"}]
        }
    }))
    .unwrap();
    let paragraph = paragraph.resolve().unwrap();
    assert_eq!(paragraph.text(), "First second");
    assert_eq!(
        paragraph.format(),
        &ParagraphFormat {
            bidi: Some(true),
            heading: Some(2),
            style: Some("Normal".into()),
            alignment: Some(Alignment::JustifyMedium),
            spacing_before: Some(4.),
            spacing_after: Some(6.),
            line_spacing: None,
            line_height: Some(LineHeight::AtLeast(12.)),
            left_indent: Some(-3.),
            right_indent: Some(8.),
            first_line_indent: Some(-2.),
            keep_together: Some(true),
            keep_with_next: Some(false),
            page_break_before: Some(true),
            widow_control: Some(false),
            suppress_line_numbers: Some(true),
            contextual_spacing: Some(false),
            mirror_indents: Some(true),
            snap_to_grid: Some(false),
            tab_stops: vec![TabStop {
                position: -9.,
                alignment: TabAlignment::Decimal,
                leader: Some(TabLeader::MiddleDot),
            }],
        },
    );
    assert_eq!(
        paragraph.runs()[0].format(),
        Some(&RunFormat {
            style: Some("Emphasis".into()),
            bold: Some(false),
            italic: Some(true),
            underline: Some(Underline::Double),
            font: Some("Latin".into()),
            size: Some(11.5),
            color: Some(Color::Theme {
                name: ThemeColor::Accent2,
                tint: None,
                shade: Some(150),
            }),
            superscript: Some(true),
            subscript: Some(false),
            strike: Some(true),
            all_caps: Some(false),
            small_caps: Some(true),
            snap_to_grid: Some(false),
            language: Some(RunLanguage {
                latin: Some("en-US".into()),
                east_asia: Some("ja-JP".into()),
                bidi: Some("ar-SA".into()),
            }),
            bold_complex_script: Some(true),
            italic_complex_script: Some(false),
            size_complex_script: Some(14.),
            rtl: Some(false),
            complex_script: Some(true),
            font_east_asia: Some("East Asia".into()),
            font_complex_script: Some("Complex".into()),
        }),
    );
    assert_eq!(paragraph.runs()[1].format(), Some(&RunFormat::default()));

    let legacy: ParagraphInput = serde_json::from_value(json!({
        "inlines": [], "format": {"line_spacing": 1.25}
    }))
    .unwrap();
    assert_eq!(legacy.resolve().unwrap().format().line_spacing, Some(1.25));
}

#[test]
fn resolves_inline_containers_and_leaf_content() {
    let bytes = include_bytes!("../../../../../tests/fixtures/images/sample.png");
    let block: BlockInput = serde_json::from_value(json!({
        "kind": "paragraph",
        "paragraph": {"inlines": [
            {"kind": "tab"},
            {"kind": "break"},
            {"kind": "break", "type": "page"},
            {"kind": "hyperlink", "target": {"kind": "url", "url": "https://example.com"},
             "children": [{"kind": "text", "text": "link", "format": {"color": [1, 2, 3]}}]},
            {"kind": "field", "instruction": {"kind": "num-pages"}, "complex": true, "locked": false,
             "children": [{"kind": "hyperlink", "target": {"kind": "bookmark", "name": "Target"},
                           "children": [{"kind": "text", "text": "cache", "format": {"color": "auto"}}]}]},
            {"kind": "fraction", "numerator": "1", "denominator": "2"},
            {"kind": "image", "bytes": bytes.as_slice()}
        ]}
    }))
    .unwrap();
    let ResolvedBlock::Paragraph(paragraph) = block.resolve().unwrap() else {
        panic!("expected paragraph");
    };
    let [
        ResolvedInline::Tab { .. },
        ResolvedInline::Break {
            kind: BreakType::Line,
            ..
        },
        ResolvedInline::Break {
            kind: BreakType::Page,
            ..
        },
        ResolvedInline::Hyperlink(link),
        ResolvedInline::Field(field),
        ResolvedInline::Fraction {
            numerator,
            denominator,
        },
        ResolvedInline::Image { .. },
    ] = paragraph.runs()
    else {
        panic!("unexpected inline projection: {:?}", paragraph.runs());
    };
    assert_eq!(paragraph.text(), "\t\nlinkcache");
    assert_eq!(link.target().as_str(), "https://example.com");
    assert_eq!(
        link.children()[0].format().unwrap().color,
        Some(Color::Rgb([1, 2, 3]))
    );
    assert_eq!(field.instruction().text(), " NUMPAGES ");
    assert_eq!(field.properties().form, FieldForm::Complex);
    assert_eq!(field.properties().locked, Some(false));
    let [ResolvedInline::Hyperlink(bookmark)] = field.children() else {
        panic!("expected bookmark link");
    };
    assert_eq!(
        bookmark.target().bookmark_name().unwrap().as_str(),
        "Target"
    );
    assert_eq!(
        bookmark.children()[0].format().unwrap().color,
        Some(Color::Auto)
    );
    assert_eq!((numerator.as_str(), denominator.as_str()), ("1", "2"));
}

#[test]
fn resolves_only_nonempty_rectangular_text_tables() {
    let block: BlockInput = serde_json::from_value(json!({
        "kind": "table", "rows": [["A", "B"], ["C", "D"]], "column_widths": [800, 1600]
    }))
    .unwrap();
    let ResolvedBlock::Table(table) = block.resolve().unwrap() else {
        panic!("expected table");
    };
    assert_eq!(table.column_widths(), Some([800, 1600].as_slice()));
    let [ResolvedBlock::Paragraph(cell)] = table.rows()[1].cells()[1].blocks() else {
        panic!("expected cell paragraph");
    };
    assert_eq!(cell.text(), "D");

    for rows in [json!([]), json!([[]]), json!([["A"], ["B", "C"]])] {
        let block: BlockInput =
            serde_json::from_value(json!({"kind": "table", "rows": rows})).unwrap();
        assert_rejected(
            block.resolve().unwrap_err(),
            "rows must form a nonempty rectangular table",
        );
    }
}

#[test]
fn rejects_unknown_keys_and_incorrect_wire_types() {
    for inline in [
        json!({"kind": "text", "text": "x", "format": {"bold": "false"}}),
        json!({"kind": "text", "text": "x", "format": {"language": {"unknown": "en-GB"}}}),
        json!({"kind": "text", "text": "x", "format": {"color": {"theme": "accent1", "extra": 0}}}),
        json!({"kind": "text", "text": "x", "format": {"color": [1, 2, 256]}}),
        json!({"kind": "tab", "extra": false}),
        json!({"kind": "break", "type": "section"}),
        json!({"kind": "image", "bytes": [1], "path": "/tmp/image"}),
        json!({"kind": "hyperlink", "target": {"kind": "url", "url": "https://example.com",
                                              "name": "Unexpected"}, "children": []}),
        json!({"kind": "field", "instruction": {"kind": "page", "extra": false}}),
    ] {
        assert!(
            serde_json::from_value::<InlineInput>(inline.clone()).is_err(),
            "{inline}"
        );
    }
    for format in [
        json!({"tab_stops": [{"position": 10, "alignment": "top"}]}),
        json!({"line_height": {"kind": "exact", "value": 12, "extra": true}}),
        json!({"heading": -1}),
    ] {
        assert!(
            serde_json::from_value::<ParagraphInput>(json!({
                "inlines": [], "format": format
            }))
            .is_err(),
            "{format}"
        );
    }
    assert!(serde_json::from_value::<ParagraphInput>(json!({"text":"x"})).is_err());
    assert!(
        serde_json::from_value::<BlockInput>(json!({
            "kind": "paragraph", "paragraph": {"inlines":[]}, "rows": []
        }))
        .is_err()
    );
}

#[test]
fn delegates_content_rules_to_native_validation() {
    let paragraph: ParagraphInput = serde_json::from_value(json!({
        "inlines": [], "format": {"line_spacing": 1.5, "line_height": {"kind":"exact","value":10}}
    }))
    .unwrap();
    assert_native_diagnostic(
        paragraph.resolve().unwrap_err(),
        Rule::InvalidContent,
        "paragraph.format.line_height",
        "line_height and line_spacing are mutually exclusive",
    );
    for (inline, rule, path, message) in [
        (
            json!({"kind": "hyperlink", "target": {"kind": "url", "url": "relative"}, "children": []}),
            Rule::InvalidRelationship,
            "target",
            "hyperlink target must be an absolute external URI/IRI with valid percent escapes",
        ),
        (
            json!({"kind": "field", "instruction": {"kind": "page"},
                   "children": [{"kind": "fraction", "numerator": "1", "denominator": "2"}]}),
            Rule::InvalidContent,
            "children[0]",
            "inline fractions in cached field results are unsupported",
        ),
        (
            json!({
                "kind": "field-tree",
                "nodes": [{
                    "instruction": {"kind": "page"},
                    "children": [{"kind": "tab", "format": {"size": -1}}]
                }]
            }),
            Rule::InvalidMeasurement,
            "size",
            "size cannot be negative",
        ),
    ] {
        let inline: InlineInput = serde_json::from_value(inline).unwrap();
        assert_native_diagnostic(inline.resolve().unwrap_err(), rule, path, message);
    }
}

#[test]
fn invalid_xml_keeps_the_native_error_wrapper_and_display() {
    let inline: InlineInput =
        serde_json::from_value(json!({"kind": "text", "text": "é\0"})).unwrap();
    let error = inline.resolve().unwrap_err();
    assert_eq!(
        error.to_string(),
        "text_run.text: invalid XML 1.0 character U+0000 at byte offset 2",
    );
    let ScopedEditError::Native(error) = error else {
        panic!("expected native XML error, got {error:?}");
    };
    assert_eq!(error.rule(), Some(Rule::InvalidXml));
    assert_eq!(error.path(), Some("text_run.text"));
}

#[test]
fn table_declarations_and_donors_keep_native_diagnostics() {
    let block: BlockInput = serde_json::from_value(json!({
        "kind": "table", "rows": [["cell"]], "column_widths": [0]
    }))
    .unwrap();
    let node = json!({
        "rows": [{"cells": [{"blocks": []}]}],
        "properties": {"column_widths": [0]}
    });

    let mut errors = vec![block.resolve().unwrap_err()];
    for table in [node.clone(), json!({"nodes": [node]})] {
        let fragment: RichTableInput = serde_json::from_value(table.clone()).unwrap();
        let donor: RichTableInput = serde_json::from_value(table).unwrap();
        errors.push(fragment.resolve().unwrap_err());
        errors.push(donor.resolve_donor().unwrap_err());
    }
    for error in errors {
        assert_native_diagnostic(
            error,
            Rule::InvalidContent,
            "table.column_widths",
            "column_widths must be positive",
        );
    }
}

#[test]
fn flat_declaration_limits_remain_transport_rejections() {
    let field: InlineInput =
        serde_json::from_value(json!({"kind": "field-tree", "nodes": []})).unwrap();
    assert_rejected(
        field.resolve().unwrap_err(),
        "field-tree requires 1..=16384 nodes",
    );

    let table: RichTableInput = serde_json::from_value(json!({"nodes": []})).unwrap();
    assert_rejected(
        table.resolve_donor().unwrap_err(),
        "flat rich table requires 1 to 1024 nodes",
    );
}

fn assert_rejected(error: ScopedEditError, message: &str) {
    assert!(matches!(&error, ScopedEditError::Rejected(_)), "{error:?}");
    assert_eq!(error.to_string(), message);
}

fn assert_native_diagnostic(error: ScopedEditError, rule: Rule, path: &str, detail: &str) {
    assert_eq!(error.to_string(), format!("{path}: {detail}"));
    let ScopedEditError::Native(rocx::Error::Diagnostic(diagnostic)) = error else {
        panic!("expected native diagnostic, got {error:?}");
    };
    assert_eq!(diagnostic.rule, rule);
    assert_eq!(diagnostic.location.as_deref(), Some(path));
    assert_eq!(diagnostic.message, detail);
}

#[test]
fn preview_schema_publishes_resolvable_inline_references() {
    let tool = ToolKind::Preview.definition();
    let schema = Value::Object((*tool.input_schema).clone());
    let inline = schema
        .pointer("/$defs/InlineInput")
        .expect("published inline schema");
    let variants = inline["oneOf"].as_array().unwrap();
    for variant in variants {
        assert_eq!(variant["additionalProperties"], false, "{variant}");
    }
    for kind in ["hyperlink", "field"] {
        let variant = variants
            .iter()
            .find(|variant| variant["properties"]["kind"]["const"] == kind)
            .unwrap();
        let reference = variant["properties"]["children"]["items"]["$ref"]
            .as_str()
            .unwrap();
        assert_eq!(
            schema.pointer(reference.strip_prefix('#').unwrap()),
            Some(inline)
        );
    }
}
