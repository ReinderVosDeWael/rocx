//! Language and I/O boundaries.
pub(crate) mod io;

mod image;
#[cfg(feature = "python-bindings")]
mod python;
pub(crate) use image::resolve as inspect_image;
