//! Standalone feasibility probe for a supervised WebAssembly process.

pub mod child;
#[cfg(feature = "component-proof")]
mod component;
mod interpreter;

pub use interpreter::evaluate;
pub use mundam_runtime_transport::{
    COMPONENT_HEADER, MAX_MODULE_BYTES, Outcome, PreviewRejection, encode, preview, supervised,
};

#[cfg(windows)]
pub use mundam_runtime_transport::job;
