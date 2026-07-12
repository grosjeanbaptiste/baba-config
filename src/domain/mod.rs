//! The pure domain kernel: value objects derived from Baba's taxonomy, the
//! fact-parsing primitives, the `FactSource` port, and the kernel error type.
//! Zero I/O — everything here is a pure function of already-read facts.

pub mod error;
pub mod packs;
pub mod parse;
pub mod source;
pub mod taxonomy;

pub use error::KernelError;
pub use packs::{Pack, Registry};
pub use source::FactSource;
pub use taxonomy::{FileKind, FileType, Object, Taxonomy, TimeWindow, Verb, VerbClass};
