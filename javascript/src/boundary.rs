//! Shared adapter types. All core paths and failures cross this boundary once.
use crate::host::Wire;
use js_sys::{Array, Error as JsError, JsString, Object, Reflect};
use rocx::{Condition, Error, Result};
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    future::{Future, poll_fn},
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
pub(crate) enum Paths {
    Declaration,
    Package,
}

pub(crate) struct Failure {
    message: String,
    path: String,
    cause: Option<Wire>,
    producer: bool,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundaryFailure")
            .field("message", &self.message)
            .finish()
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Failure {}
impl Failure {
    pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Error {
        Error::producer(Self {
            message: message.into(),
            path: path.into(),
            cause: None,
            producer: false,
        })
    }
    pub(crate) fn host(message: &str, cause: JsValue) -> Error {
        Self::with_cause(message, "".into(), cause, false)
    }
    pub(crate) fn predicate(path: String, cause: JsValue) -> Error {
        Self::with_cause("condition threw", path, cause, false)
    }
    pub(crate) fn callback(cause: JsValue) -> Error {
        Self::with_cause("content producer failed", "".into(), cause, true)
    }
    fn with_cause(message: &str, path: String, cause: JsValue, producer: bool) -> Error {
        Error::producer(Self {
            message: message.into(),
            path,
            cause: Some(Wire::new(cause)),
            producer,
        })
    }
}

fn find_failure(error: &Error) -> Option<&Failure> {
    use std::error::Error as _;
    let mut chain = error.source();
    while let Some(source) = chain {
        if let Some(failure) = source.downcast_ref() {
            return Some(failure);
        }
        chain = source.source();
    }
    None
}

fn rule(error: &Error) -> Option<rocx::Rule> {
    match find_failure(error) {
        Some(failure) if failure.producer => error.rule(),
        Some(_) => None,
        None => error.rule(),
    }
}

fn declaration_path(path: &str) -> String {
    // Rust's ParagraphContent::Inlines path is `children`; JS exposes `runs`.
    // Section/header `children` are not renamed. Paragraph-specific routes
    // normalize the path before this generic mapping when needed.
    let mut result = String::with_capacity(path.len());
    let mut uppercase = false;
    for ch in path.chars() {
        if ch == '_' {
            uppercase = true;
        } else if uppercase {
            result.push(ch.to_ascii_uppercase());
            uppercase = false;
        } else {
            result.push(ch);
        }
    }
    result
}

pub(crate) fn js_error(error: &Error, paths: Paths) -> JsValue {
    let failure = find_failure(error);
    let message = failure.map_or_else(|| error.to_string(), |f| f.message.clone());
    let result: JsValue = JsError::new(&message).into();
    if let Some(cause) = failure.and_then(|failure| failure.cause.as_ref()) {
        set(&result, "cause", &cause.js());
    }
    if let Some(rule) = rule(error) {
        set(&result, "rule", &rule.code().into());
    }
    let path = failure
        .map(|f| f.path.as_str())
        .filter(|p| !p.is_empty())
        .or_else(|| error.path());
    if let Some(path) = path {
        let path = match paths {
            Paths::Declaration => declaration_path(path),
            Paths::Package => path.to_owned(),
        };
        set(&result, "path", &path.into());
    }
    result
}

struct PassState {
    aborted: Option<Wire>,
    predicate: Option<Error>,
    waker: Option<Waker>,
    aliases: Vec<(String, String)>,
}

#[derive(Clone)]
pub(crate) struct Pass(Arc<Mutex<PassState>>);

#[wasm_bindgen]
pub struct Operation(Pass);
#[wasm_bindgen]
impl Operation {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(Pass(Arc::new(Mutex::new(PassState {
            aborted: None,
            predicate: None,
            waker: None,
            aliases: Vec::new(),
        }))))
    }
    pub fn abort(&self, cause: JsValue) {
        let waker = {
            let mut state = self.0.0.lock().unwrap();
            state.aborted = Some(Wire::new(cause));
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
impl Operation {
    pub(crate) fn pass(&self) -> Pass {
        self.0.clone()
    }
}

impl Pass {
    pub(crate) fn alias(&self, rust: String, js: String) {
        self.0.lock().unwrap().aliases.push((rust, js));
    }
    fn error(&self, error: &Error) -> JsValue {
        let result = js_error(error, Paths::Declaration);
        let path = Reflect::get(&result, &"path".into())
            .ok()
            .and_then(|p| p.as_string());
        if let Some(mut path) = path {
            for (rust, js) in &self.0.lock().unwrap().aliases {
                if let Some(suffix) = path.strip_prefix(rust)
                    && (suffix.is_empty() || suffix.starts_with(['.', '[']))
                {
                    path = format!("{js}{suffix}");
                }
            }
            set(&result, "path", &path.into());
        }
        result
    }
    pub(crate) fn condition(&self, input: Wire, path: String) -> Result<Condition> {
        let pass = self.clone();
        crate::host::predicate(input, path, move |cause| {
            let mut state = pass.0.lock().unwrap();
            if state.predicate.is_none() {
                state.predicate = Some(cause);
            }
        })
    }
    pub(crate) async fn run<T>(
        &self,
        future: impl Future<Output = Result<T>>,
    ) -> std::result::Result<T, JsValue> {
        let mut future = std::pin::pin!(future);
        let outcome = poll_fn(|cx| {
            let check = |waker: &Waker| {
                let mut state = self.0.lock().unwrap();
                state.waker = Some(waker.clone());
                (state.aborted.clone(), state.predicate.take())
            };
            let (abort, failure) = check(cx.waker());
            if let Some(reason) = abort {
                return Poll::Ready(Err(abort_error(reason)));
            }
            if let Some(error) = failure {
                return Poll::Ready(Err(self.error(&error)));
            }
            let outcome = future.as_mut().poll(cx);
            // A Rust Condition can only return bool. The predicate latch
            // overrides even a successful exclusion or a pending sibling.
            let (abort, failure) = check(cx.waker());
            if let Some(reason) = abort {
                return Poll::Ready(Err(abort_error(reason)));
            }
            if let Some(error) = failure {
                return Poll::Ready(Err(self.error(&error)));
            }
            outcome.map(|result| result.map_err(|error| self.error(&error)))
        })
        .await;
        self.0.lock().unwrap().waker.take();
        outcome
    }
}

fn abort_error(reason: Wire) -> JsValue {
    let error = JsError::new("The operation was aborted");
    error.set_name("AbortError");
    set(&error, "cause", &reason.js());
    error.into()
}

pub(crate) fn set(object: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(object, &key.into(), value).expect("private data object");
}

pub(crate) fn record(fields: &[(&str, JsValue)]) -> JsValue {
    let object: JsValue = Object::new().into();
    for (key, value) in fields {
        set(&object, key, value);
    }
    object
}

pub(crate) struct Record {
    value: JsValue,
    path: String,
}
impl Record {
    pub(crate) fn new(input: &Wire, path: &str, allowed: &[&str]) -> Result<Self> {
        let value = input.js();
        if !value.is_object() || value.is_null() || value.is_function() || Array::is_array(&value) {
            return Err(Failure::invalid(path, "expected a declaration record"));
        }
        let keys = Reflect::own_keys(&value)
            .map_err(|cause| Failure::host("cannot read record keys", cause))?;
        for key in keys.iter() {
            if !key
                .as_string()
                .is_some_and(|key| allowed.contains(&key.as_str()))
            {
                return Err(Failure::invalid(
                    path,
                    format!(
                        "unknown field `{}`",
                        key.as_string().as_deref().unwrap_or("<symbol>")
                    ),
                ));
            }
        }
        Ok(Self {
            value,
            path: path.into(),
        })
    }
    pub(crate) fn get(&self, key: &str) -> Result<Wire> {
        member(&self.value, key, &self.path)
    }
    pub(crate) fn path(&self, key: &str) -> String {
        child_path(&self.path, key)
    }
}

pub(crate) fn child_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

pub(crate) fn member(value: &JsValue, key: &str, path: &str) -> Result<Wire> {
    let path = child_path(path, key);
    if !value.is_object() || value.is_null() {
        return Err(Failure::invalid(&path, "expected a declaration record"));
    }
    let descriptor =
        Reflect::get_own_property_descriptor(value.unchecked_ref::<Object>(), &key.into())
            .map_err(|cause| {
                Failure::with_cause("cannot read field descriptor", path.clone(), cause, false)
            })?;
    if descriptor.is_undefined() {
        return Ok(Wire::new(JsValue::UNDEFINED));
    }
    if !Reflect::has(&descriptor, &"value".into()).unwrap_or(false) {
        return Err(Failure::invalid(
            &path,
            "record fields must be data properties",
        ));
    }
    Reflect::get(&descriptor, &"value".into())
        .map(Wire::new)
        .map_err(|cause| Failure::with_cause("cannot read field", path, cause, false))
}

pub(crate) fn list(input: &Wire, path: &str) -> Result<Vec<Wire>> {
    let value = input.js();
    if !Array::is_array(&value) {
        return Err(Failure::invalid(path, "expected an array"));
    }
    let len = Reflect::get(&value, &"length".into())
        .map_err(|cause| Failure::host("cannot read array length", cause))?
        .as_f64()
        .filter(|x| *x >= 0.0 && *x <= u32::MAX as f64 && x.fract() == 0.0)
        .ok_or_else(|| Failure::invalid(path, "invalid array length"))? as u32;
    (0..len)
        .map(|index| member(&value, &index.to_string(), path))
        .collect()
}

pub(crate) fn text(input: &Wire, _: &Pass, path: &str) -> Result<String> {
    string(input, path)
}

pub(crate) fn string(input: &Wire, path: &str) -> Result<String> {
    let value = input.js();
    if !value.is_string() {
        return Err(Failure::invalid(path, "expected a string"));
    }
    let value = value.unchecked_into::<JsString>();
    if !value.is_well_formed() {
        return Err(Failure::invalid(
            path,
            "malformed UTF-16: unpaired surrogate",
        ));
    }
    Ok(String::from(value))
}

pub(crate) fn u64_number(input: &Wire, path: &str) -> Result<u64> {
    input
        .js()
        .as_f64()
        .filter(|x| *x >= 0.0 && *x <= 9_007_199_254_740_991.0 && x.fract() == 0.0)
        .map(|value| value as u64)
        .ok_or_else(|| Failure::invalid(path, "expected a non-negative safe integer"))
}

#[allow(dead_code)] // Shared with B1 DTO decoders.
pub(crate) fn i64_number(input: &Wire, path: &str) -> Result<i64> {
    let value = input.js();
    if value.is_bigint() {
        return i64::try_from(value)
            .map_err(|_| Failure::invalid(path, "bigint is outside i64 bounds"));
    }
    value
        .as_f64()
        .filter(|x| x.abs() <= 9_007_199_254_740_991.0 && x.fract() == 0.0)
        .map(|value| value as i64)
        .ok_or_else(|| Failure::invalid(path, "expected a safe integer or i64 bigint"))
}

pub(crate) fn bytes(input: &Wire, _: &Pass, path: &str) -> Result<Vec<u8>> {
    super::image_bytes(&input.js())
        .map_err(|err| Failure::invalid(path, String::from(err.message())))
}

#[derive(Default)]
struct Handles {
    next: u32,
    values: HashMap<u32, Weak<dyn Any>>,
}
thread_local! {
    static HANDLES: RefCell<Handles> = RefCell::new(Handles::default());
}

/// JS wrappers own this Wasm token; the lookup index never owns the engine data.
#[wasm_bindgen]
pub struct EngineHandle {
    id: u32,
    _value: Rc<dyn Any>,
}
impl Drop for EngineHandle {
    fn drop(&mut self) {
        HANDLES.with(|handles| handles.borrow_mut().values.remove(&self.id));
    }
}
#[wasm_bindgen]
impl EngineHandle {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> u32 {
        self.id
    }
}
impl EngineHandle {
    pub(crate) fn new<T: 'static>(value: T) -> Self {
        let value: Rc<dyn Any> = Rc::new(value);
        let id = HANDLES.with(|handles| {
            let mut handles = handles.borrow_mut();
            handles.next = handles
                .next
                .checked_add(1)
                .expect("Wasm handle space exhausted");
            let id = handles.next;
            handles.values.insert(id, Rc::downgrade(&value));
            id
        });
        Self { id, _value: value }
    }
}

pub(crate) fn handle<T: 'static>(kind: &str, value: T, view: JsValue) -> JsValue {
    handle_with_methods(kind, value, view, &[])
}

pub(crate) fn handle_with_methods<T: 'static>(
    kind: &str,
    value: T,
    view: JsValue,
    methods: &[(&str, &str)],
) -> JsValue {
    // Return a private envelope (never exposed to consumers); JS materializes
    // the instance-bound wrapper and nested views.
    let result = record(&[
        ("$rocxHandle", EngineHandle::new(value).into()),
        ("kind", kind.into()),
        ("view", view),
    ]);
    if !methods.is_empty() {
        let methods = record(
            &methods
                .iter()
                .map(|(key, value)| (*key, (*value).into()))
                .collect::<Vec<_>>(),
        );
        set(&result, "methods", &methods);
    }
    result
}

pub(crate) fn borrow<T: 'static>(input: &Wire, path: &str) -> Result<Rc<T>> {
    let id = crate::host::invoke("handleId", &Array::of1(&input.js()))
        .map_err(|cause| Failure::with_cause("cannot inspect handle", path.into(), cause, false))?
        .as_f64()
        .unwrap_or(0.0) as u32;
    let invalid = || {
        Failure::invalid(
            path,
            "expected a resolved handle of the right type from this Wasm instance",
        )
    };
    HANDLES
        .with(|handles| handles.borrow().values.get(&id).and_then(Weak::upgrade))
        .and_then(|value| value.downcast().ok())
        .ok_or_else(invalid)
}
