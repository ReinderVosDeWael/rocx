//! A11–A12: images, DrawingML shapes and charts, fractions and run separators.
mod geometry;

use crate::{
    boundary::{self as b, Pass, Record},
    host::Wire,
    program::{self, Hooks, optional, required},
};
use geometry::{alt_text, chart, shape, transformation};
use js_sys::Uint8Array;
use rocx::{
    Break, BreakType, ColumnChartRun, ImageData, ImageRun, Inline, InlineFraction, ResolvedComment,
    ResolvedImage, ResolvedInline, Result, RunFormat, ShapeRun, Tab,
};
use wasm_bindgen::JsValue;

pub(crate) const HOOKS: Hooks = Hooks {
    components: &[
        ("image", None, resolve_image),
        ("shape", None, resolve_shape),
        ("columnChart", None, resolve_chart),
        ("fraction", None, resolve_fraction),
        ("tab", None, resolve_tab),
        ("break", None, resolve_break),
    ],
    inlines: &[
        ("image", inline_image),
        ("shape", inline_shape),
        ("columnChart", inline_chart),
        ("fraction", inline_fraction),
        ("tab", inline_tab),
        ("break", inline_break),
    ],
    inline_views: &[inline_view],
    ..Hooks::EMPTY
};

fn image_data(input: &Wire, pass: &Pass, path: &str) -> Result<ImageData> {
    let value = input.js();
    if js_sys::ArrayBuffer::is_view(&value) || !value.is_object() || value.is_null() {
        return b::bytes(input, pass, path).map(ImageData::Bytes);
    }
    // The Node factory reads the file before creating this value. The source
    // name is metadata only; Wasm never opens or interprets the path.
    let record = Record::new(input, path, &["bytes", "sourceName"])?;
    Ok(ImageData::NamedBytes {
        bytes: b::bytes(&record.get("bytes")?, pass, &record.path("bytes"))?,
        source_name: b::string(&record.get("sourceName")?, &record.path("sourceName"))?,
    })
}

fn maybe<T>(
    decode: program::Decode<T>,
    input: &Wire,
    pass: &Pass,
    path: &str,
) -> Result<Option<T>> {
    if input.js().is_undefined() {
        Ok(None)
    } else {
        decode(input, pass, path).map(Some)
    }
}

fn guard(record: &Record, pass: &Pass) -> Result<rocx::Condition> {
    pass.condition(record.get("condition")?, record.path("condition"))
}

fn image_run(input: &Wire, pass: &Pass, path: &str) -> Result<ImageRun> {
    let record = Record::new(
        input,
        path,
        &[
            "kind",
            "condition",
            "data",
            "type",
            "transformation",
            "altText",
        ],
    )?;
    let mut image = ImageRun::deferred(required(&record, pass, "data", image_data)?)
        .when(guard(&record, pass)?);
    image.r#type = maybe(b::text, &record.get("type")?, pass, &record.path("type"))?;
    image.transformation = optional(&record, pass, "transformation", |wire, pass, path| {
        maybe(transformation, wire, pass, path)
    })?
    .unwrap_or_default();
    image.alt_text = optional(&record, pass, "altText", |wire, pass, path| {
        maybe(alt_text, wire, pass, path)
    })?
    .unwrap_or_default();
    Ok(image)
}

fn shape_run(input: &Wire, pass: &Pass, path: &str) -> Result<ShapeRun> {
    let record = Record::new(input, path, &["kind", "properties", "condition"])?;
    Ok(ShapeRun::new(required(&record, pass, "properties", shape)?).when(guard(&record, pass)?))
}

fn chart_run(input: &Wire, pass: &Pass, path: &str) -> Result<ColumnChartRun> {
    let record = Record::new(input, path, &["kind", "properties", "condition"])?;
    Ok(
        ColumnChartRun::new(required(&record, pass, "properties", chart)?)
            .when(guard(&record, pass)?),
    )
}

fn fraction(input: &Wire, pass: &Pass, path: &str) -> Result<InlineFraction> {
    let record = Record::new(
        input,
        path,
        &["kind", "numerator", "denominator", "condition"],
    )?;
    Ok(InlineFraction::new(
        required(&record, pass, "numerator", b::text)?,
        required(&record, pass, "denominator", b::text)?,
    )
    .when(guard(&record, pass)?))
}

fn tab(input: &Wire, pass: &Pass, path: &str) -> Result<Tab> {
    let record = Record::new(input, path, &["kind", "condition"])?;
    Ok(Tab::default().when(guard(&record, pass)?))
}

fn break_run(input: &Wire, pass: &Pass, path: &str) -> Result<Break> {
    let record = Record::new(input, path, &["kind", "breakType", "condition"])?;
    Ok(Break::new(read_break_type(&record)?).when(guard(&record, pass)?))
}

fn read_break_type(record: &Record) -> Result<BreakType> {
    geometry::get(record, "breakType", geometry::break_type)
        .map(|kind| kind.unwrap_or_else(|| Break::default().r#type))
}

macro_rules! component_hooks {
    ($inline:ident, $resolve:ident, $decode:ident, $root:literal) => {
        fn $inline(input: &Wire, pass: &Pass, path: &str) -> Result<Inline> {
            $decode(input, pass, path).map(Inline::from)
        }
        fn $resolve(
            input: Wire,
            pass: Pass,
        ) -> futures_util::future::BoxFuture<'static, Result<Option<program::Projection>>> {
            Box::pin(async move {
                program::resolve($decode(&input, &pass, $root)?, program::inline_handle).await
            })
        }
    };
}
component_hooks!(inline_image, resolve_image, image_run, "imageRun");
component_hooks!(inline_shape, resolve_shape, shape_run, "shapeRun");
component_hooks!(inline_chart, resolve_chart, chart_run, "columnChartRun");
component_hooks!(
    inline_fraction,
    resolve_fraction,
    fraction,
    "inlineFraction"
);
component_hooks!(inline_tab, resolve_tab, tab, "tab");
component_hooks!(inline_break, resolve_break, break_run, "break");

fn image_handle(image: &ResolvedImage) -> JsValue {
    let (width, height) = image.dimensions_emu();
    // Rust bounds picture dimensions by i32, so the JS numbers are exact.
    let view = b::record(&[
        ("name", image.name().into()),
        ("bytes", Uint8Array::from(image.data()).into()),
        (
            "dimensionsEmu",
            js_sys::Array::of2(&(width as f64).into(), &(height as f64).into()).into(),
        ),
        ("contentType", image.content_type().into()),
    ]);
    if let Some(alt) = image.alt_text() {
        b::set(&view, "altText", &geometry::alt_text_view(alt));
    }
    b::handle("image", image.clone(), view)
}

fn inline_view(inline: &ResolvedInline) -> Option<JsValue> {
    let view = match inline {
        ResolvedInline::Image { image, .. } => {
            b::record(&[("kind", "image".into()), ("image", image_handle(image))])
        }
        ResolvedInline::Shape(properties) => b::record(&[
            ("kind", "shape".into()),
            ("properties", geometry::shape_view(properties)),
        ]),
        ResolvedInline::ColumnChart(properties) => b::record(&[
            ("kind", "columnChart".into()),
            ("properties", geometry::chart_view(properties)),
        ]),
        ResolvedInline::Fraction {
            numerator,
            denominator,
        } => b::record(&[
            ("kind", "fraction".into()),
            ("numerator", numerator.as_str().into()),
            ("denominator", denominator.as_str().into()),
        ]),
        ResolvedInline::Tab { .. } => b::record(&[("kind", "tab".into())]),
        ResolvedInline::Break { kind, .. } => b::record(&[
            ("kind", "break".into()),
            ("breakType", geometry::break_type_token(*kind).into()),
        ]),
        _ => return None,
    };
    b::set(&view, "displayText", &inline.text().as_ref().into());
    if let Some(format) = inline.format() {
        b::set(
            &view,
            "format",
            &crate::text_layout::run_format_view(format),
        );
    }
    if let Some(comment) = inline.comment() {
        b::set(
            &view,
            "comment",
            &crate::structured::plain_comment_view(comment),
        );
    }
    Some(b::handle("inline", inline.clone(), view))
}

/// R06 route owned by the shared validator. Return Rust diagnostics unchanged.
pub(crate) fn validate_transformation(input: &Wire, pass: &Pass, path: &str) -> Result<()> {
    Ok(transformation(input, pass, path)?.validate()?)
}

/// R05 enum arms called by the shared constructor, which supplies the common
/// RunFormat/comment conversion. Contextual checks stay in Rust validation.
pub(crate) fn construct_inline(
    input: &Wire,
    kind: &str,
    pass: &Pass,
    path: &str,
    run_props: impl FnOnce(&Record) -> Result<(RunFormat, Option<ResolvedComment>)>,
) -> Result<Option<ResolvedInline>> {
    Ok(Some(match kind {
        "image" => {
            let record = Record::new(input, path, &["kind", "image", "format", "comment"])?;
            let image = b::borrow::<ResolvedImage>(&record.get("image")?, &record.path("image"))?;
            let (format, comment) = run_props(&record)?;
            ResolvedInline::Image {
                image: (*image).clone(),
                format,
                comment,
            }
        }
        "tab" => {
            let record = Record::new(input, path, &["kind", "format", "comment"])?;
            let (format, comment) = run_props(&record)?;
            ResolvedInline::Tab { format, comment }
        }
        "break" => {
            let record = Record::new(input, path, &["kind", "breakType", "format", "comment"])?;
            let kind = read_break_type(&record)?;
            let (format, comment) = run_props(&record)?;
            ResolvedInline::Break {
                kind,
                format,
                comment,
            }
        }
        "shape" => {
            let record = Record::new(input, path, &["kind", "properties"])?;
            ResolvedInline::Shape(shape(
                &record.get("properties")?,
                pass,
                &record.path("properties"),
            )?)
        }
        "columnChart" => {
            let record = Record::new(input, path, &["kind", "properties"])?;
            ResolvedInline::ColumnChart(chart(
                &record.get("properties")?,
                pass,
                &record.path("properties"),
            )?)
        }
        "fraction" => {
            let record = Record::new(input, path, &["kind", "numerator", "denominator"])?;
            ResolvedInline::Fraction {
                numerator: b::string(&record.get("numerator")?, &record.path("numerator"))?,
                denominator: b::string(&record.get("denominator")?, &record.path("denominator"))?,
            }
        }
        _ => return Ok(None),
    }))
}
