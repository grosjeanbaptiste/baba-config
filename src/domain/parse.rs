//! Pure fact-parsing primitives — zero I/O. They lift flat Prolog facts
//! (`head(arg, arg[, arg]).`, one per line) into Rust, operating on text that
//! has *already been read* (the file read is the infrastructure concern, in
//! [`crate::io`]). Blank lines and `%` comments are skipped; anything not
//! matching the requested head is ignored. Deliberately not a Prolog reader —
//! `swipl` consult in corpus CI validates full syntax.

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

/// Collect every `head(a, b).` fact in `text` as an `(a, b)` pair, in order.
#[must_use]
pub fn pairs_from_text(text: &str, head: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for body in fact_bodies(text) {
        if let Some(args) = strip_head(body, head)
            && let Some(pair) = split2(args)
        {
            out.push(pair);
        }
    }
    out
}

/// Collect every `head(a, b, c).` fact in `text` as an `(a, b, c)` triple, in
/// order. The 3-arity mirror of [`pairs_from_text`] — used by the package
/// registry (`pack(name, summary, basis).`).
#[must_use]
pub fn triples_from_text(text: &str, head: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for body in fact_bodies(text) {
        if let Some(args) = strip_head(body, head)
            && let Some(triple) = split3(args)
        {
            out.push(triple);
        }
    }
    out
}

/// Collect every single-argument `head(<value>).` fact in `text`, unquoted, in
/// order. The mirror of [`pairs_from_text`] for one-arity settings.
#[must_use]
pub fn singles_from_text(text: &str, head: &str) -> Vec<String> {
    let mut out = Vec::new();
    for body in fact_bodies(text) {
        if let Some(arg) = strip_head(body, head) {
            out.push(unquote(arg.trim()));
        }
    }
    out
}

/// Yield each non-comment, non-blank line stripped of its trailing `.` — the
/// shared skeleton of [`pairs_from_text`] and [`singles_from_text`].
fn fact_bodies(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            return None;
        }
        trimmed.strip_suffix('.')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_head_matches_and_rejects() {
        assert_eq!(
            strip_head("filetype(rust, rs)", "filetype("),
            Some("rust, rs")
        );
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
    fn pairs_from_text_reads_facts_and_skips_comments() {
        let text = "% comment\nfiletype(rust, rs).\n\nfiletype(python, py).\nother(x, y).\n";
        assert_eq!(
            pairs_from_text(text, "filetype("),
            vec![
                ("rust".to_string(), "rs".to_string()),
                ("python".to_string(), "py".to_string())
            ]
        );
    }

    #[test]
    fn triples_from_text_reads_three_arg_facts_in_order() {
        let text = "% c\npack(brew, 'Homebrew', 'scan brew').\npack(git, 'Git', 'scan git').\nother(x, y).\n";
        assert_eq!(
            triples_from_text(text, "pack("),
            vec![
                (
                    "brew".to_string(),
                    "Homebrew".to_string(),
                    "scan brew".to_string()
                ),
                ("git".to_string(), "Git".to_string(), "scan git".to_string()),
            ]
        );
        assert!(triples_from_text(text, "missing(").is_empty());
    }

    #[test]
    fn singles_from_text_reads_values_in_order() {
        let text = "% comment\nlink_parser_flag('-limit=1').\nlink_parser_flag('-echo=0').\n";
        assert_eq!(
            singles_from_text(text, "link_parser_flag("),
            vec!["-limit=1".to_string(), "-echo=0".to_string()]
        );
        assert!(singles_from_text(text, "missing(").is_empty());
    }
}
