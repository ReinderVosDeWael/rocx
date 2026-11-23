//! Byte-backed template compilation and immutable OPC plan handles.
use crate::{
    boundary::{self as b, Failure, Operation, Paths, Record},
    host::Wire,
    program::{Hooks, render_options},
};
use js_sys::{Array, Reflect, Uint8Array};
use rocx::{
    ResolvedDocument, Result,
    application::{compile, compile_with_edits},
    opc::{PackagePlan, PartName},
    template::{EditScope, TemplateEdits, TextRevisionAction},
};
use wasm_bindgen::prelude::*;

pub(crate) const HOOKS: Hooks = Hooks {
    document_methods: &[("toDocxWithTemplate", "toDocxWithTemplate")],
    operations: &[
        ("compileTemplate", (compile_template, Paths::Package)),
        ("toDocxWithTemplate", (render_template, Paths::Package)),
        ("packagePlanParts", (parts, Paths::Package)),
        ("packagePlanPart", (part, Paths::Package)),
        ("packagePlanValidate", (validate, Paths::Package)),
        ("packagePlanToBytes", (to_bytes, Paths::Package)),
    ],
    ..Hooks::EMPTY
};

fn template_edits(input: &Wire, path: &str) -> Result<TemplateEdits> {
    let record = Record::new(
        input,
        path,
        &[
            "replacements",
            "contentControlReplacements",
            "textRevisionAction",
            "paragraphIndex",
            "scope",
        ],
    )?;
    let mut edits = TemplateEdits::default();
    let replacements = record.get("replacements")?;
    if !replacements.js().is_undefined() {
        let path = record.path("replacements");
        for (index, tuple) in b::list(&replacements, &path)?.iter().enumerate() {
            let path = format!("{path}[{index}]");
            let tuple = b::list(tuple, &path)?;
            let [needle, replacement] = tuple.as_slice() else {
                return Err(Failure::invalid(&path, "expected [needle, replacement]"));
            };
            edits.replacements.push((
                b::string(needle, &format!("{path}[0]"))?,
                b::string(replacement, &format!("{path}[1]"))?,
            ));
        }
    }
    let controls = record.get("contentControlReplacements")?.js();
    if !controls.is_undefined() {
        let path = record.path("contentControlReplacements");
        if !controls.is_object()
            || controls.is_null()
            || controls.is_function()
            || Array::is_array(&controls)
        {
            return Err(Failure::invalid(&path, "expected a tag-to-text record"));
        }
        let keys = Reflect::own_keys(&controls)
            .map_err(|cause| Failure::host("cannot read content control tags", cause))?;
        for key in keys.iter() {
            let tag = b::string(&Wire::new(key), &path)?;
            let replacement = b::string(
                &b::member(&controls, &tag, &path)?,
                &b::child_path(&path, &tag),
            )?;
            edits.content_control_replacements.insert(tag, replacement);
        }
    }
    let action = record.get("textRevisionAction")?;
    if !action.js().is_undefined() {
        let path = record.path("textRevisionAction");
        edits.text_revision_action = Some(match b::string(&action, &path)?.as_str() {
            "accept" => TextRevisionAction::Accept,
            "reject" => TextRevisionAction::Reject,
            _ => return Err(Failure::invalid(&path, "unknown text revision action")),
        });
    }
    let index = record.get("paragraphIndex")?;
    if !index.js().is_undefined() {
        let path = record.path("paragraphIndex");
        edits.paragraph_index = Some(
            usize::try_from(b::u64_number(&index, &path)?)
                .map_err(|_| Failure::invalid(&path, "outside usize bounds"))?,
        );
    }
    let scope = record.get("scope")?;
    if !scope.js().is_undefined() {
        let path = record.path("scope");
        edits.scope = match b::string(&scope, &path)?.as_str() {
            "body" => EditScope::Body,
            "bodyHeadersFooters" => EditScope::BodyHeadersFooters,
            "supportedStories" => EditScope::SupportedStories,
            _ => return Err(Failure::invalid(&path, "unknown template edit scope")),
        };
    }
    Ok(edits)
}

fn compile_source(document: &Wire, source: &Wire, options: Wire) -> Result<PackagePlan> {
    let document = b::borrow::<ResolvedDocument>(document, "document")?;
    let source = Record::new(source, "source", &["bytes", "edits"])?;
    let bytes = b::bytes(
        &source.get("bytes")?,
        &Operation::new().pass(),
        "source.bytes",
    )?;
    let options = render_options(options)?;
    let edits = source.get("edits")?;
    if edits.js().is_undefined() {
        compile(&document, &bytes, &options)
    } else {
        compile_with_edits(
            &document,
            &bytes,
            &template_edits(&edits, "source.edits")?,
            &options,
        )
    }
}

fn compile_template(input: &Wire) -> Result<JsValue> {
    let record = Record::new(input, "", &["document", "source", "options"])?;
    let plan = compile_source(
        &record.get("document")?,
        &record.get("source")?,
        record.get("options")?,
    )?;
    Ok(b::handle_with_methods(
        "packagePlan",
        plan,
        JsValue::UNDEFINED,
        &[
            ("parts", "packagePlanParts"),
            ("part", "packagePlanPart"),
            ("validate", "packagePlanValidate"),
            ("toBytes", "packagePlanToBytes"),
        ],
    ))
}

fn method(input: &Wire, min: usize, max: usize) -> Result<(Wire, Vec<Wire>)> {
    let record = Record::new(input, "", &["target", "args"])?;
    let args = b::list(&record.get("args")?, "args")?;
    if !(min..=max).contains(&args.len()) {
        return Err(Failure::invalid("args", "unexpected argument count"));
    }
    Ok((record.get("target")?, args))
}

fn render_template(input: &Wire) -> Result<JsValue> {
    let (target, args) = method(input, 1, 2)?;
    let mut args = args.into_iter();
    let source = args.next().expect("source argument");
    let options = args.next().unwrap_or_else(|| Wire::new(JsValue::UNDEFINED));
    to_js_bytes(compile_source(&target, &source, options)?.to_bytes())
}

fn parts(input: &Wire) -> Result<JsValue> {
    let (target, _) = method(input, 0, 0)?;
    let plan = b::borrow::<PackagePlan>(&target, "plan")?;
    Ok(plan
        .parts()
        .map(|(name, payload)| {
            JsValue::from(Array::of2(&name.into(), &Uint8Array::from(payload).into()))
        })
        .collect::<Array>()
        .into())
}

fn part(input: &Wire) -> Result<JsValue> {
    let (target, args) = method(input, 1, 1)?;
    let plan = b::borrow::<PackagePlan>(&target, "plan")?;
    let name = PartName::new(&b::string(&args[0], "name")?)?;
    Ok(plan.part(&name).map_or(JsValue::UNDEFINED, |payload| {
        Uint8Array::from(payload).into()
    }))
}

fn validate(input: &Wire) -> Result<JsValue> {
    let (target, _) = method(input, 0, 0)?;
    b::borrow::<PackagePlan>(&target, "plan")?.validate()?;
    Ok(JsValue::UNDEFINED)
}

fn to_bytes(input: &Wire) -> Result<JsValue> {
    let (target, _) = method(input, 0, 0)?;
    to_js_bytes(b::borrow::<PackagePlan>(&target, "plan")?.to_bytes())
}

fn to_js_bytes(bytes: Result<Vec<u8>>) -> Result<JsValue> {
    bytes.map(|bytes| Uint8Array::from(bytes.as_slice()).into())
}
