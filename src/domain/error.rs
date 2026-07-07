//! The kernel's own error type — kept zero-dependency (no `anyhow`) so the
//! shared crate stays stock-toolchain buildable. Implements `std::error::Error`
//! so `?` lifts it into the consumers' `anyhow::Result` transparently.

use std::fmt;

/// A domain-rule violation surfaced by the taxonomy kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// An object noun has no `walk_kind/2` classification in the taxonomy, so
    /// it can't be resolved to a [`crate::FileKind`] (it isn't file-walkable).
    /// Carries the offending object name.
    UnwalkableObject(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::UnwalkableObject(object) => write!(
                f,
                "object kind '{object}' not supported (no walk_kind fact in the taxonomy)"
            ),
        }
    }
}

impl std::error::Error for KernelError {}
