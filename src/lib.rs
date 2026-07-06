//! Minimal, zero-dependency reader for Baba's Prolog-fact config files.
//!
//! This is the one piece of Rust shared by both runtime sub-repos: the cli
//! reads `link_parser_flag/1` from `variables.pl`, and the actions read
//! `filetype/2`, `is_a/2`, `verb_class/2`, `limit/2`, `repo_slug_prefix/1`,
//! … from the same family of files. Both used to carry their own ad-hoc line
//! reader; this crate is the single source of truth (ADR-022).
//!
//! Every accepted fact is `head(arg, arg[, arg]).` on its own line; blank
//! lines and `%` comments are ignored, and anything that doesn't match the
//! requested head is skipped. This is deliberately **not** a Prolog reader —
//! the `swipl` consult check in corpus CI validates full syntax; here we only
//! lift flat facts into Rust.
//!
//! `warn(missing_docs)` — every public item documents its contract.

#![warn(missing_docs)]

use std::path::{Path, PathBuf};

/// The canonical Baba workspace directory name under `$HOME`. Mirrors
/// `baba-install`'s `BABA_HOME_REL`, `baba-cli`'s and `baba-actions`'
/// `BABA_DIR` — kept here so the resolution tail (`~/Baba/corpus/data`) has
/// one definition both crates share.
pub const BABA_DIR: &str = "Baba";

/// Resolve a data file shipped under `corpus/data/` (`taxonomy.pl`,
/// `variables.pl`), CWD-independent — an action runs in the user's
/// arbitrary directory. Order: `$<explicit_env>` (a file path) →
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

/// Strip a `head(` prefix and the trailing `)` from a fact body (the body
/// being a line already stripped of its trailing `.`).
#[must_use]
pub fn strip_head<'a>(body: &'a str, head: &str) -> Option<&'a str> {
    body.strip_prefix(head)?.strip_suffix(')')
}

/// Split a 2-arg fact's inner argument list, trimming and unquoting each.
#[must_use]
pub fn split2(args: &str) -> Option<(String, String)> {
    let mut parts = args.splitn(2, ',').map(|s| unquote(s.trim()));
    Some((parts.next()?, parts.next()?))
}

/// Split a 3-arg fact's inner argument list, trimming and unquoting each.
#[must_use]
pub fn split3(args: &str) -> Option<(String, String, String)> {
    let mut parts = args.splitn(3, ',').map(|s| unquote(s.trim()));
    Some((parts.next()?, parts.next()?, parts.next()?))
}

/// Strip surrounding single quotes if present. Prolog atoms come quoted
/// (`'baba-list'`) when they contain non-alpha characters.
#[must_use]
pub fn unquote(s: &str) -> String {
    s.trim().trim_matches('\'').to_string()
}

/// Read `path` and collect every `head(a, b).` fact as an `(a, b)` pair.
/// A missing or unreadable file yields an empty vec — callers supply
/// their own compiled defaults, so absence is never an error here.
#[must_use]
pub fn parse_pairs(path: &Path, head: &str) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let Some(body) = trimmed.strip_suffix('.') else {
            continue;
        };
        if let Some(args) = strip_head(body, head)
            && let Some(pair) = split2(args)
        {
            out.push(pair);
        }
    }
    out
}

/// Pure parse of single-argument facts (`head(<value>).`), returning each
/// unquoted value in file order. The mirror of [`parse_pairs`] for the
/// one-arity settings (e.g. `repo_slug_prefix('grosjeanbaptiste/baba-')`,
/// `link_parser_flag('-limit=1')`).
#[must_use]
pub fn parse_singles(path: &Path, head: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let Some(body) = trimmed.strip_suffix('.') else {
            continue;
        };
        if let Some(arg) = strip_head(body, head) {
            out.push(unquote(arg.trim()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_with(tag: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baba-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{tag}.pl"));
        std::fs::File::create(&p)
            .unwrap()
            .write_all(contents.as_bytes())
            .unwrap();
        p
    }

    #[test]
    fn strip_head_matches_and_rejects() {
        assert_eq!(strip_head("filetype(rust, rs)", "filetype("), Some("rust, rs"));
        assert_eq!(strip_head("other(x, y)", "filetype("), None);
    }

    #[test]
    fn split2_trims_and_unquotes() {
        assert_eq!(split2("rust,  'rs' "), Some(("rust".into(), "rs".into())));
        assert_eq!(split2("only-one-arg"), None);
    }

    #[test]
    fn split3_reads_three() {
        assert_eq!(
            split3("list, line, 'baba-search'"),
            Some(("list".into(), "line".into(), "baba-search".into()))
        );
    }

    #[test]
    fn parse_pairs_reads_facts_and_skips_comments() {
        let p = tmp_with(
            "pairs",
            "% comment\nfiletype(rust, rs).\n\nfiletype(python, py).\nother(x, y).\n",
        );
        let got = parse_pairs(&p, "filetype(");
        assert_eq!(
            got,
            vec![
                ("rust".to_string(), "rs".to_string()),
                ("python".to_string(), "py".to_string())
            ]
        );
    }

    #[test]
    fn parse_singles_reads_unquoted_values_in_order() {
        let p = tmp_with(
            "singles",
            "% comment\nlink_parser_flag('-limit=1').\nlink_parser_flag('-echo=0').\nother(x).\n",
        );
        assert_eq!(
            parse_singles(&p, "link_parser_flag("),
            vec!["-limit=1".to_string(), "-echo=0".to_string()]
        );
        assert!(parse_singles(&p, "missing(").is_empty());
    }

    #[test]
    fn missing_file_yields_empty_not_error() {
        let p = std::env::temp_dir().join("baba-config-does-not-exist.pl");
        assert!(parse_pairs(&p, "filetype(").is_empty());
        assert!(parse_singles(&p, "filetype(").is_empty());
    }
}
