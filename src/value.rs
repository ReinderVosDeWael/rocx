use crate::{Error, Result};
use std::{fmt, future::Future, pin::Pin};

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
