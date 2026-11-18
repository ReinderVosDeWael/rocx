use crate::{Error, Result};
use futures_util::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use std::{
    fmt,
    future::{Future, IntoFuture},
    pin::Pin,
    sync::Arc,
};

/// A value stored directly in the tree, produced lazily, or fetched asynchronously.
///
/// Deferred producers are single-use. Enum variants may return another `Value`,
/// allowing lazy and asynchronous stages to be composed without eager evaluation.
pub enum Value<T> {
    Ready(T),
    Lazy(Box<dyn FnOnce() -> Result<Value<T>> + Send + 'static>),
    Async(Pin<Box<dyn Future<Output = Result<Value<T>>> + Send + 'static>>),
}

impl<T> Value<T> {
    pub fn ready(value: T) -> Self {
        Self::Ready(value)
    }

    pub fn lazy(producer: impl FnOnce() -> T + Send + 'static) -> Self {
        Self::Lazy(Box::new(move || Ok(Self::Ready(producer()))))
    }

    pub fn try_lazy<E>(
        producer: impl FnOnce() -> std::result::Result<T, E> + Send + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Lazy(Box::new(move || {
            producer().map(Self::Ready).map_err(Error::producer)
        }))
    }

    pub fn future(future: impl Future<Output = T> + Send + 'static) -> Self {
        Self::Async(Box::pin(async move { Ok(Self::Ready(future.await)) }))
    }

    pub fn try_future<E>(
        future: impl Future<Output = std::result::Result<T, E>> + Send + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Async(Box::pin(async move {
            future.await.map(Self::Ready).map_err(Error::producer)
        }))
    }

    /// Resolve all nested deferred stages of this value.
    pub async fn resolve(self) -> Result<T> {
        let mut current = self;
        loop {
            current = match current {
                Self::Ready(value) => return Ok(value),
                Self::Lazy(producer) => producer()?,
                Self::Async(future) => future.await?,
            };
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Value<T> {
    /// Share this property's producer with other fields without starting it.
    /// Every consumer receives a clone of the same result.
    pub fn deferred(self) -> Deferred<T> {
        Deferred::new(self)
    }
}

/// A reusable, lazy dependency on a property value.
///
/// Put a clone into the source field, and await another clone from dependent
/// producers. The producer runs once, regardless of declaration order. No
/// executor or background task is created; consumers poll the shared future.
/// Values must be cloneable because the Rust tree owns each resolved field.
/// A handle refers to its producer, independently of component conditions.
///
/// ```
/// use rv_docx_rs::{Error, Value};
/// # futures::executor::block_on(async {
/// let count = Value::future(async { 42_u64 }).deferred();
/// let dependency = count.clone();
/// let summary = Value::try_future(async move {
///     Ok::<_, Error>(format!("Count: {}", dependency.await?))
/// });
/// assert_eq!(summary.resolve().await?, "Count: 42");
/// assert_eq!(count.await?, 42);
/// # Ok::<(), Error>(())
/// # }).unwrap();
/// ```
pub struct Deferred<T> {
    future: Shared<BoxFuture<'static, std::result::Result<T, Arc<Error>>>>,
}

impl<T> Clone for Deferred<T> {
    fn clone(&self) -> Self {
        Self {
            future: self.future.clone(),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Deferred<T> {
    pub fn new(value: Value<T>) -> Self {
        Self {
            future: async move { value.resolve().await.map_err(Arc::new) }
                .boxed()
                .shared(),
        }
    }

    /// Await the cached property result, preserving shared producer errors.
    pub async fn resolve(&self) -> Result<T> {
        self.future.clone().await.map_err(Error::Shared)
    }
}

impl<T: Clone + Send + Sync + 'static> IntoFuture for Deferred<T> {
    type Output = Result<T>;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move { self.resolve().await }.boxed()
    }
}

impl<T: Clone + Send + Sync + 'static> From<Deferred<T>> for Value<T> {
    fn from(value: Deferred<T>) -> Self {
        Self::Async(async move { value.await.map(Self::Ready) }.boxed())
    }
}

impl<T> fmt::Debug for Deferred<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Deferred(..)")
    }
}

impl<T> From<T> for Value<T> {
    fn from(value: T) -> Self {
        Self::Ready(value)
    }
}
impl From<&str> for Value<String> {
    fn from(value: &str) -> Self {
        Self::Ready(value.into())
    }
}
impl<T: Default> Default for Value<T> {
    fn default() -> Self {
        Self::Ready(T::default())
    }
}
impl<T: fmt::Debug> fmt::Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(value) => f.debug_tuple("Ready").field(value).finish(),
            Self::Lazy(_) => f.write_str("Lazy(..)"),
            Self::Async(_) => f.write_str("Async(..)"),
        }
    }
}

/// A synchronous condition evaluated once before a component's fields resolve.
pub enum Condition {
    Ready(bool),
    Lazy(Box<dyn FnOnce() -> bool + Send + 'static>),
}

impl Condition {
    pub fn lazy(predicate: impl FnOnce() -> bool + Send + 'static) -> Self {
        Self::Lazy(Box::new(predicate))
    }
    pub(crate) fn evaluate(self) -> bool {
        match self {
            Self::Ready(value) => value,
            Self::Lazy(predicate) => predicate(),
        }
    }
}
impl From<bool> for Condition {
    fn from(value: bool) -> Self {
        Self::Ready(value)
    }
}
impl Default for Condition {
    fn default() -> Self {
        Self::Ready(true)
    }
}
impl fmt::Debug for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(value) => f.debug_tuple("Ready").field(value).finish(),
            Self::Lazy(_) => f.write_str("Lazy(..)"),
        }
    }
}

mod property;
pub(crate) use property::in_pass;
pub use property::{Property, PropertyId, PropertyRef};

impl<T: Clone + Send + Sync + 'static> Value<T> {
    pub fn property(self, name: impl Into<String>) -> Property<T> {
        Property::new(name, self)
    }
}

mod interpreter;

mod components;
pub use components::Component;
