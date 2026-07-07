//! Infrastructure adapter: obtain flat facts by reading a `.pl` file from
//! disk. The one place in the kernel that touches the filesystem — the domain
//! ([`crate::domain`]) stays pure and receives facts through the
//! [`FactSource`](crate::domain::source::FactSource) port this implements.

use std::path::{Path, PathBuf};

use crate::domain::parse::{pairs_from_text, singles_from_text};
use crate::domain::source::FactSource;

/// The canonical Baba workspace directory name under `$HOME`. Mirrors
/// `baba-install`'s `BABA_HOME_REL` and the two crates' `BABA_DIR` — one
/// definition both share for the resolution tail (`~/Baba/corpus/data`).
pub const BABA_DIR: &str = "Baba";

/// Resolve a data file shipped under `corpus/data/` (`taxonomy.pl`,
/// `variables.pl`), CWD-independent. Order: `$<explicit_env>` (a file path) →
/// `$BABA_DATA_DIR/<name>` (injected by `baba-cli`) →
/// `$XDG_CONFIG_HOME/baba/<name>` → `~/.config/baba/<name>` →
/// `~/Baba/corpus/data/<name>`.
#[must_use]
pub fn resolve_data_file(explicit_env: &str, name: &str) -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(explicit_env) {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::var_os("BABA_DATA_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(xdg).join("baba"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".config").join("baba"));
        dirs.push(home.join(BABA_DIR).join("corpus").join("data"));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

/// A [`FactSource`] backed by one `.pl` file, read once at construction. A
/// missing/unreadable file yields an empty source (every query returns empty)
/// — callers decide whether absence is fatal.
pub struct FileFactSource {
    text: String,
}

impl FileFactSource {
    /// Read the facts at `path` (empty source if unreadable).
    #[must_use]
    pub fn at(path: &Path) -> Self {
        Self {
            text: std::fs::read_to_string(path).unwrap_or_default(),
        }
    }

    /// Resolve the file via [`resolve_data_file`], then read it. `None` when
    /// nothing resolves along the chain (the file is absent everywhere).
    #[must_use]
    pub fn resolve(explicit_env: &str, name: &str) -> Option<Self> {
        resolve_data_file(explicit_env, name).map(|p| Self::at(&p))
    }
}

impl FactSource for FileFactSource {
    fn pairs(&self, head: &str) -> Vec<(String, String)> {
        pairs_from_text(&self.text, head)
    }
    fn singles(&self, head: &str) -> Vec<String> {
        singles_from_text(&self.text, head)
    }
}

/// Read `path` and collect every `head(a, b).` fact as an `(a, b)` pair.
/// A missing/unreadable file yields an empty vec. Backward-compatible free
/// function over the [`FileFactSource`] adapter.
#[must_use]
pub fn parse_pairs(path: &Path, head: &str) -> Vec<(String, String)> {
    FileFactSource::at(path).pairs(head)
}

/// Read `path` and collect every single-argument `head(x).` fact, unquoted.
/// Backward-compatible free function over the [`FileFactSource`] adapter.
#[must_use]
pub fn parse_singles(path: &Path, head: &str) -> Vec<String> {
    FileFactSource::at(path).singles(head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_with(tag: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baba-config-io-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{tag}.pl"));
        std::fs::File::create(&p)
            .unwrap()
            .write_all(contents.as_bytes())
            .unwrap();
        p
    }

    #[test]
    fn file_fact_source_reads_pairs_and_singles() {
        let p = tmp_with(
            "src",
            "% c\nfiletype(rust, rs).\nlink_parser_flag('-limit=1').\n",
        );
        let src = FileFactSource::at(&p);
        assert_eq!(src.pairs("filetype("), vec![("rust".into(), "rs".into())]);
        assert_eq!(src.singles("link_parser_flag("), vec!["-limit=1".to_string()]);
    }

    #[test]
    fn parse_pairs_free_fn_matches_and_missing_is_empty() {
        let p = tmp_with("pairs", "filetype(python, py).\n");
        assert_eq!(parse_pairs(&p, "filetype("), vec![("python".into(), "py".into())]);
        let missing = std::env::temp_dir().join("baba-config-io-nope.pl");
        assert!(parse_pairs(&missing, "filetype(").is_empty());
        assert!(parse_singles(&missing, "x(").is_empty());
    }
}
