//! Typed property dependencies and cancellation-safe shared producers.
//! Execution state lives here; the document domain contains none of it.
use super::{Condition, Deferred, Value};
use crate::{Diagnostic, Error, Result, Rule};
use futures_util::future::BoxFuture;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    future::{Future, IntoFuture},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
};
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyId(u64);
#[derive(Default)]
struct Graph {
    waits: BTreeMap<PropertyId, BTreeMap<PropertyId, usize>>,
}
impl Graph {
    fn reaches(&self, start: PropertyId, target: PropertyId) -> bool {
        let mut pending = vec![start];
        let mut visited = BTreeSet::new();
        while let Some(key) = pending.pop() {
            if key == target {
                return true;
            }
            if visited.insert(key)
                && let Some(edges) = self.waits.get(&key)
            {
                pending.extend(edges.keys().copied());
            }
        }
        false
    }
}
type GraphHandle = Arc<Mutex<Graph>>;
thread_local! {static ACTIVE:RefCell<Option<(GraphHandle,Option<PropertyId>)>>=const {RefCell::new(None)};}
struct Restore(Option<(GraphHandle, Option<PropertyId>)>);
impl Drop for Restore {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = self.0.take());
    }
}
struct Scoped<F> {
    future: Pin<Box<F>>,
    graph: GraphHandle,
    id: Option<PropertyId>,
}
impl<F: Future> Future for Scoped<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let _restore =
            Restore(ACTIVE.with(|active| active.replace(Some((this.graph.clone(), this.id)))));
        this.future.as_mut().poll(context)
    }
}
pub(crate) async fn in_pass<F: Future>(future: F) -> F::Output {
    let (graph, id) = ACTIVE
        .with(|active| active.borrow().clone())
        .unwrap_or_default();
    Scoped {
        future: Box::pin(future),
        graph,
        id,
    }
    .await
}
struct Edge {
    graph: GraphHandle,
    caller: Option<PropertyId>,
    target: PropertyId,
}
impl Drop for Edge {
    fn drop(&mut self) {
        if let Some(caller) = self.caller {
            let mut graph = self.graph.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(edges) = graph.waits.get_mut(&caller)
                && let Some(count) = edges.get_mut(&self.target)
            {
                *count -= 1;
                if *count == 0 {
                    edges.remove(&self.target);
                }
            }
        }
    }
}
struct Inner<T> {
    id: PropertyId,
    label: String,
    value: Deferred<T>,
    guard: Arc<Mutex<Option<Condition>>>,
}
/// An owned field producer. References share its value and honor its own guard.
pub struct Property<T> {
    inner: Arc<Inner<T>>,
}
/// A typed dependency on a named property. Dropping one waiter leaves others usable.
pub struct PropertyRef<T> {
    inner: Arc<Inner<T>>,
}
impl<T> Clone for PropertyRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<T> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Property").field(&self.inner.label).finish()
    }
}
impl<T> std::fmt::Debug for PropertyRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PropertyRef")
            .field(&self.inner.label)
            .finish()
    }
}
impl<T: Clone + Send + Sync + 'static> Property<T> {
    pub fn new(name: impl Into<String>, value: Value<T>) -> Self {
        let id = PropertyId(NEXT.fetch_add(1, Ordering::Relaxed));
        let label = name.into();
        let error_label = label.clone();
        let guard = Arc::new(Mutex::new(Some(Condition::default())));
        let predicate = guard.clone();
        let value = Value::try_future(async move {
            let graph = ACTIVE
                .with(|active| active.borrow().as_ref().map(|(g, _)| g.clone()))
                .unwrap_or_default();
            Scoped {
                future: Box::pin(async move {
                    let condition = predicate
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .take()
                        .unwrap_or_default();
                    if !condition.evaluate() {
                        return Err(Error::from(Diagnostic::new(
                            Rule::ExcludedDependency,
                            format!("excluded property {error_label}"),
                        )));
                    }
                    value.resolve().await
                }),
                graph,
                id: Some(id),
            }
            .await
        })
        .deferred();
        Self {
            inner: Arc::new(Inner {
                id,
                label,
                value,
                guard,
            }),
        }
    }
    pub fn when(self, condition: impl Into<Condition>) -> Self {
        *self.inner.guard.lock().unwrap_or_else(|e| e.into_inner()) = Some(condition.into());
        self
    }
    pub fn reference(&self) -> PropertyRef<T> {
        PropertyRef {
            inner: self.inner.clone(),
        }
    }
}
impl<T: Clone + Send + Sync + 'static> PropertyRef<T> {
    pub fn id(&self) -> PropertyId {
        self.inner.id
    }
    pub async fn resolve(&self) -> Result<T> {
        let (graph, caller) = ACTIVE
            .with(|active| active.borrow().clone())
            .unwrap_or_default();
        {
            let mut state = graph.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(caller) = caller {
                if state.reaches(self.inner.id, caller) {
                    return Err(Diagnostic::new(
                        Rule::DependencyCycle,
                        format!("cyclic property dependency: {}", self.inner.label),
                    )
                    .into());
                }
                *state
                    .waits
                    .entry(caller)
                    .or_default()
                    .entry(self.inner.id)
                    .or_default() += 1;
            }
        }
        let _edge = Edge {
            graph: graph.clone(),
            caller,
            target: self.inner.id,
        };
        Scoped {
            future: Box::pin(self.inner.value.resolve()),
            graph,
            id: caller,
        }
        .await
    }
}
impl<T: Clone + Send + Sync + 'static> IntoFuture for PropertyRef<T> {
    type Output = Result<T>;
    type IntoFuture = BoxFuture<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.resolve().await })
    }
}
impl<T: Clone + Send + Sync + 'static> IntoFuture for Property<T> {
    type Output = Result<T>;
    type IntoFuture = BoxFuture<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        self.reference().into_future()
    }
}
impl<T: Clone + Send + Sync + 'static> From<Property<T>> for Value<T> {
    fn from(value: Property<T>) -> Self {
        Self::Async(Box::pin(async move { value.await.map(Self::ready) }))
    }
}
impl<T: Clone + Send + Sync + 'static> From<PropertyRef<T>> for Value<T> {
    fn from(value: PropertyRef<T>) -> Self {
        Self::Async(Box::pin(async move { value.await.map(Self::ready) }))
    }
}
