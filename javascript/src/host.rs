//! Poll JavaScript work from the requesting Rust Value, without sending JsValues.
use futures_util::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use js_sys::{Array, Function, Reflect};
use rocx::{Condition, Deferred, Error, Property, PropertyRef, Result, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    future::poll_fn,
    sync::{Arc, Mutex, Weak},
    task::{Poll, Waker},
};
use wasm_bindgen::prelude::*;

use crate::boundary::{self, Failure, Pass};

#[derive(Default)]
struct Registry {
    next: u32,
    values: HashMap<u32, JsValue>,
    calls: HashMap<u32, Pending>,
    shared: HashMap<u32, Weak<Shared>>,
    host: Option<JsValue>,
}
impl Registry {
    fn id(&mut self) -> u32 {
        self.next = self
            .next
            .checked_add(1)
            .expect("Wasm ticket space exhausted");
        self.next
    }
}
thread_local! { static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default()); }

struct Ticket(u32);
impl Drop for Ticket {
    fn drop(&mut self) {
        REGISTRY.with(|r| r.borrow_mut().values.remove(&self.0));
    }
}

/// A scoped, owned reference to a local JS value. Only the ticket crosses Rust awaits.
#[derive(Clone)]
pub(crate) struct Wire(Arc<Ticket>);
impl Wire {
    pub(crate) fn new(value: JsValue) -> Self {
        let id = REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            let id = r.id();
            r.values.insert(id, value);
            id
        });
        Self(Arc::new(Ticket(id)))
    }
    pub(crate) fn js(&self) -> JsValue {
        REGISTRY.with(|r| r.borrow().values[&self.0.0].clone())
    }

    /// Keep an owned copy of a forced byte value in a Rust share, and never
    /// hand that copy back to JavaScript callers or transform callbacks.
    pub(crate) fn output(&self) -> JsValue {
        let value = self.js();
        Self::byte_copy(&value).unwrap_or(value)
    }

    fn ready(self) -> Value<Self> {
        Value::ready(Self::byte_copy(&self.js()).map_or(self, Self::new))
    }

    fn byte_copy(value: &JsValue) -> Option<JsValue> {
        js_sys::ArrayBuffer::is_view(value)
            .then(|| crate::copy_image_view(value).ok().map(Into::into))
            .flatten()
    }
}

#[wasm_bindgen]
pub fn configure_host(host: JsValue) {
    REGISTRY.with(|r| r.borrow_mut().host = Some(host));
}

pub(crate) fn invoke(method: &str, args: &Array) -> std::result::Result<JsValue, JsValue> {
    let host = REGISTRY.with(|r| r.borrow().host.clone().expect("configured JS host"));
    Reflect::get(&host, &method.into())?
        .dyn_into::<Function>()?
        .apply(&host, args)
}

pub(crate) fn inspect(value: &JsValue, consume: bool) -> Result<Array> {
    invoke("inspect", &Array::of2(value, &consume.into()))
        .map(|value| value.unchecked_into::<Array>())
        .map_err(|cause| Failure::host("cannot inspect value", cause))
}

struct Request {
    id: u32,
    target: Wire,
    callback: Wire,
}
#[derive(Default)]
struct Pending {
    result: Option<Settlement>,
    requests: Vec<Request>,
    errors: HashMap<u32, Arc<Error>>,
    waker: Option<Waker>,
}
enum Settlement {
    Ready(Wire),
    Failed(Wire, u32),
}

#[wasm_bindgen]
pub fn request_dependency(call: u32, target: JsValue, callback: Function) -> bool {
    let request = Request {
        id: REGISTRY.with(|r| r.borrow_mut().id()),
        target: Wire::new(target),
        callback: Wire::new(callback.into()),
    };
    let (active, waker) = REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        if let Some(pending) = r.calls.get_mut(&call).filter(|p| p.result.is_none()) {
            pending.requests.push(request);
            (true, pending.waker.take())
        } else {
            (false, None)
        }
    });
    if let Some(waker) = waker {
        waker.wake();
    }
    active
}

#[wasm_bindgen]
pub fn settle_producer(call: u32, success: bool, value: JsValue, dependency: u32) {
    let value = Wire::new(value);
    let settlement = if success {
        Settlement::Ready(value)
    } else {
        Settlement::Failed(value, dependency)
    };
    let waker = REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        if let Some(pending) = r.calls.get_mut(&call).filter(|p| p.result.is_none()) {
            pending.result = Some(settlement);
            pending.waker.take()
        } else {
            None
        }
    });
    if let Some(waker) = waker {
        waker.wake();
    }
}

struct Call(u32, bool);
impl Call {
    fn new() -> Self {
        let id = REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            let id = r.id();
            r.calls.insert(id, Pending::default());
            id
        });
        Self(id, false)
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        let pending = REGISTRY.with(|r| r.borrow_mut().calls.remove(&self.0));
        drop(pending);
        let _ = invoke("finish", &Array::of2(&self.0.into(), &(!self.1).into()));
    }
}

async fn produce(function: Wire, arg: Option<Wire>, synchronous: bool) -> Result<Wire> {
    let mut call = Call::new();
    {
        let mode = if arg.is_some() {
            2
        } else {
            u32::from(synchronous)
        };
        let args = Array::of4(
            &call.0.into(),
            &function.js(),
            &arg.as_ref().map_or(JsValue::UNDEFINED, Wire::output),
            &mode.into(),
        );
        invoke("start", &args).map_err(Failure::callback)?;
    }
    let mut work = FuturesUnordered::<BoxFuture<'static, (Request, Result<Wire>)>>::new();
    poll_fn(|cx| {
        let (result, requests) = REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            let pending = r.calls.get_mut(&call.0).unwrap();
            pending.waker = Some(cx.waker().clone());
            (pending.result.take(), std::mem::take(&mut pending.requests))
        });
        if let Some(result) = result {
            call.1 = true;
            return Poll::Ready(match result {
                Settlement::Ready(value) => Ok(value),
                Settlement::Failed(cause, id) => {
                    let inherited = REGISTRY.with(|r| {
                        r.borrow_mut()
                            .calls
                            .get_mut(&call.0)
                            .unwrap()
                            .errors
                            .remove(&id)
                    });
                    Err(inherited.map_or_else(|| Failure::callback(cause.js()), Error::Shared))
                }
            });
        }
        for request in requests {
            work.push(Box::pin(async move {
                // Poll inside THIS Rust producer. PropertyRef observes its caller
                // even when the request was issued after an arbitrary JS await.
                let result = shared_ref(&request.target, false)
                    .map(|shared| async move { shared.resolve().await });
                let result = match result {
                    Ok(resolve) => resolve.await,
                    Err(error) => Err(error),
                };
                (request, result)
            }));
        }
        while let Poll::Ready(Some((request, result))) = work.poll_next_unpin(cx) {
            let (ok, value) = match result {
                Ok(value) => (true, value.output()),
                Err(error) => {
                    let js = boundary::js_error(&error, boundary::Paths::Declaration);
                    REGISTRY.with(|r| {
                        r.borrow_mut()
                            .calls
                            .get_mut(&call.0)
                            .unwrap()
                            .errors
                            .insert(request.id, Arc::new(error))
                    });
                    (false, js)
                }
            };
            let callback = request.callback.js().unchecked_into::<Function>();
            let _ = callback.call3(&JsValue::UNDEFINED, &ok.into(), &value, &request.id.into());
        }
        Poll::Pending
    })
    .await
}

enum SharedKind {
    Deferred(Deferred<Wire>),
    Property {
        reference: PropertyRef<Wire>,
        predicate_error: Arc<Mutex<Option<Arc<Error>>>>,
    },
}
struct Shared {
    kind: SharedKind,
}
impl Shared {
    async fn resolve(self: Arc<Self>) -> Result<Wire> {
        match &self.kind {
            SharedKind::Deferred(value) => value.resolve().await,
            SharedKind::Property {
                reference,
                predicate_error,
            } => {
                let result = reference.resolve().await;
                if let Some(error) = predicate_error.lock().unwrap().clone() {
                    Err(Error::Shared(error))
                } else {
                    result
                }
            }
        }
    }
}

#[wasm_bindgen]
pub struct SharedValue {
    id: u32,
    _shared: Arc<Shared>,
}
impl Drop for SharedValue {
    fn drop(&mut self) {
        REGISTRY.with(|r| r.borrow_mut().shared.remove(&self.id));
    }
}
#[wasm_bindgen]
impl SharedValue {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> u32 {
        self.id
    }
}

#[wasm_bindgen]
pub fn make_shared(
    source: JsValue,
    name: Option<String>,
    guard: JsValue,
) -> std::result::Result<SharedValue, JsValue> {
    let value = wire_value(Wire::new(source));
    let kind = match name {
        None => SharedKind::Deferred(value.deferred()),
        Some(name) => {
            let error = Arc::new(Mutex::new(None));
            let latch = error.clone();
            let condition = predicate(Wire::new(guard), "condition".into(), move |cause| {
                *latch.lock().unwrap() = Some(Arc::new(cause));
            })
            .map_err(|error| boundary::js_error(&error, boundary::Paths::Declaration))?;
            SharedKind::Property {
                reference: Property::new(name, value).when(condition).reference(),
                predicate_error: error,
            }
        }
    };
    let shared = Arc::new(Shared { kind });
    let id = REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        let id = r.id();
        r.shared.insert(id, Arc::downgrade(&shared));
        id
    });
    Ok(SharedValue {
        id,
        _shared: shared,
    })
}

fn find_shared(id: JsValue) -> Result<Arc<Shared>> {
    let id = id.as_f64().unwrap_or(0.0) as u32;
    REGISTRY
        .with(|r| r.borrow().shared.get(&id).and_then(Weak::upgrade))
        .ok_or_else(|| Failure::invalid("", "invalid shared value handle"))
}

fn shared_ref(wire: &Wire, allow_property: bool) -> Result<Arc<Shared>> {
    let token = inspect(&wire.js(), false)?;
    match token.get(0).as_string().as_deref() {
        Some("deferred" | "ref") => find_shared(token.get(1)),
        Some("property") if allow_property => find_shared(token.get(1)),
        _ => Err(Failure::invalid(
            "",
            "context.get requires a PropertyRef or Deferred",
        )),
    }
}

fn wire_value(wire: Wire) -> Value<Wire> {
    Value::Async(Box::pin(async move {
        let token = inspect(&wire.js(), true)?;
        let payload = || Wire::new(token.get(1));
        match token.get(0).as_string().as_deref() {
            Some("ready") => Ok(payload().ready()),
            Some("lazy" | "future") => {
                let synchronous = token.get(0).as_string().as_deref() == Some("lazy");
                let callback = payload();
                Ok(Value::Async(Box::pin(async move {
                    Ok(wire_value(produce(callback, None, synchronous).await?))
                })))
            }
            Some("map") => {
                let source = payload();
                let callback = Wire::new(token.get(2));
                Ok(Value::Async(Box::pin(async move {
                    let input = wire_value(source).resolve().await?;
                    Ok(wire_value(produce(callback, Some(input), true).await?))
                })))
            }
            Some("deferred" | "property" | "ref") => {
                let shared = find_shared(token.get(1))?;
                Ok(Value::Async(Box::pin(async move {
                    Ok(Value::ready(shared.resolve().await?))
                })))
            }
            _ => Ok(wire.ready()),
        }
    }))
}

/// Conversion after the Rust field actually requests it; no eager payload traversal.
pub(crate) fn field<T: Send + 'static>(
    input: Wire,
    pass: Pass,
    path: String,
    decode: fn(&Wire, &Pass, &str) -> Result<T>,
) -> Value<T> {
    Value::Async(Box::pin(async move {
        let wire = wire_value(input).resolve().await?;
        decode(&wire, &pass, &path).map(Value::ready)
    }))
}

pub(crate) async fn resolve_value(input: Wire) -> Result<Wire> {
    wire_value(input).resolve().await
}

pub(crate) fn predicate(
    input: Wire,
    path: String,
    latch: impl FnOnce(Error) + Send + 'static,
) -> Result<Condition> {
    let value = input.js();
    if value.is_undefined() {
        return Ok(Condition::default());
    }
    if let Some(value) = value.as_bool() {
        return Ok(value.into());
    }
    if !value.is_function() {
        return Err(Failure::invalid(
            &path,
            "condition must be a boolean or function",
        ));
    }
    Ok(Condition::lazy(move || {
        match input
            .js()
            .unchecked_into::<Function>()
            .call0(&JsValue::UNDEFINED)
        {
            Ok(value) if value.as_bool().is_some() => value.as_bool().unwrap(),
            Ok(_) => {
                latch(Failure::invalid(&path, "condition must return a boolean"));
                false
            }
            Err(cause) => {
                latch(Failure::predicate(path, cause));
                false
            }
        }
    }))
}
