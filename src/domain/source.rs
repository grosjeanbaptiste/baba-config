//! The `FactSource` port — how the pure domain obtains flat facts without
//! touching the filesystem itself. Defined here in the domain (a port belongs
//! to the domain); the real adapter reading a `.pl` file lives in
//! [`crate::io::FileFactSource`] (infrastructure). Tests supply an in-memory
//! source so the taxonomy aggregate is built and exercised with zero I/O.

/// A source of flat Prolog facts, queried by fact head.
pub trait FactSource {
    /// Every `head(a, b).` fact, as `(a, b)` pairs in file order.
    fn pairs(&self, head: &str) -> Vec<(String, String)>;
    /// Every single-argument `head(x).` fact, unquoted, in file order.
    fn singles(&self, head: &str) -> Vec<String>;
}
