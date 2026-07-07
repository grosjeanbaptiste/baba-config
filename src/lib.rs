//! `baba-config` — the shared **domain kernel** for Baba's runtime crates.
//!
//! One zero-dependency crate that both `baba-cli` and `baba-actions` build
//! against (ADR-022), now promoted from a flat-fact *reader* to a rich,
//! taxonomy-driven **domain model** (ADR-023):
//!
//! - [`domain`] — pure value objects derived from `taxonomy.pl` ([`Verb`],
//!   [`VerbClass`], [`Object`], [`FileKind`], [`FileType`], [`TimeWindow`],
//!   the [`Taxonomy`] aggregate), the fact-parsing primitives, the
//!   [`FactSource`] port, and [`KernelError`]. Zero I/O.
//! - [`io`] — the infrastructure adapter ([`FileFactSource`]) that reads a
//!   `.pl` file and feeds the domain through the port.
//!
//! The taxonomy is the ubiquitous language, shared: verbs carry their
//! confirmation policy, objects know their walk kind, filetypes their
//! extension — all read from data, none compiled in.
//!
//! `warn(missing_docs)` — every public item documents its contract.

#![warn(missing_docs)]

pub mod domain;
pub mod io;

// --- Domain surface -------------------------------------------------------
pub use domain::parse::{split2, split3, strip_head, unquote};
pub use domain::{
    FactSource, FileKind, FileType, KernelError, Object, Taxonomy, TimeWindow, Verb, VerbClass,
};

// --- Infrastructure surface ----------------------------------------------
pub use io::{BABA_DIR, FileFactSource, parse_pairs, parse_singles, resolve_data_file};
