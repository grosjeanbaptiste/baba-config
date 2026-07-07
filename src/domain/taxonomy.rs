//! The taxonomy aggregate and its value objects — the rich, behaviour-carrying
//! Rust representation of Baba's domain vocabulary, **built from the shared
//! `taxonomy.pl`** rather than hardcoded. This is the single source of truth
//! both `baba-cli` and `baba-actions` consume: verbs know their confirmation
//! policy, objects know whether they're file-walkable, filetypes know their
//! extension — all read from data, none compiled in (ADR-015 / ADR-019 / ADR-023).

use std::collections::HashMap;

use super::error::KernelError;
use super::source::FactSource;

/// Whether a walk should yield regular files or directories. Resolved from the
/// taxonomy's `walk_kind/2` facts — no compiled `file`/`directory` mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// Regular files only.
    File,
    /// Directories only.
    Dir,
}

impl FileKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "file" => Some(Self::File),
            "directory" | "dir" => Some(Self::Dir),
            _ => None,
        }
    }
}

/// A verb's confirmation policy (ADR-019). Drives whether a use-case prompts
/// before acting; an *unclassified* verb is fail-safe (treated as confirm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerbClass {
    /// Pure query — never prompts.
    Readonly,
    /// Mutates/erases local state — prompts.
    Destructive,
    /// Reaches outside the machine (network, remote) — prompts.
    Outward,
}

impl VerbClass {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "readonly" => Some(Self::Readonly),
            "destructive" => Some(Self::Destructive),
            "outward" => Some(Self::Outward),
            _ => None,
        }
    }

    /// Whether acting under this class requires confirmation. Only `readonly`
    /// proceeds silently.
    #[must_use]
    pub fn needs_confirm(self) -> bool {
        !matches!(self, VerbClass::Readonly)
    }
}

/// A canonical verb (e.g. `delete`, `build`) resolved against the taxonomy,
/// carrying its confirmation policy. `class` is `None` when the verb has no
/// `verb_class/2` fact — the fail-safe case ([`Verb::needs_confirm`] → true).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verb {
    name: String,
    class: Option<VerbClass>,
}

impl Verb {
    /// The canonical verb name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The resolved confirmation class, if the taxonomy classifies this verb.
    #[must_use]
    pub fn class(&self) -> Option<VerbClass> {
        self.class
    }

    /// Whether acting on this verb needs confirmation. **Fail-safe:** an
    /// unclassified verb (missing `verb_class/2`) prompts — a missing fact can
    /// only *add* confirmations, never silently drop one.
    #[must_use]
    pub fn needs_confirm(&self) -> bool {
        self.class.is_none_or(VerbClass::needs_confirm)
    }
}

/// A canonical object noun (e.g. `file`, `directory`, `cli`) resolved against
/// the taxonomy, carrying its walk classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    name: String,
    walk_kind: Option<FileKind>,
}

impl Object {
    /// The canonical object name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The kind of entry a walk over this object yields, or `None` when the
    /// object isn't file-walkable (e.g. `package`, `repo`).
    #[must_use]
    pub fn walk_kind(&self) -> Option<FileKind> {
        self.walk_kind
    }
}

/// A canonical filetype and the file extension it maps to (`rust` → `rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileType {
    canonical: String,
    extension: String,
}

impl FileType {
    /// The canonical, language-neutral type name (`rust`, `markdown`).
    #[must_use]
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// The file extension without a dot (`rs`, `md`).
    #[must_use]
    pub fn extension(&self) -> &str {
        &self.extension
    }
}

/// A static time reference and its age window in seconds (`yesterday` → 86400).
/// `today` is deliberately absent from the data — it's a runtime computation
/// against the clock, not a fixed window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeWindow {
    seconds: u64,
}

impl TimeWindow {
    /// The age window in seconds.
    #[must_use]
    pub fn seconds(self) -> u64 {
        self.seconds
    }
}

/// The taxonomy aggregate: every domain classification loaded from a
/// [`FactSource`], with typed accessors and value-object factories. Pure — no
/// I/O; the source performs any read before this is built.
#[derive(Clone, Debug, Default)]
pub struct Taxonomy {
    filetypes: HashMap<String, String>,
    time_windows: HashMap<String, u64>,
    is_a: Vec<(String, String)>,
    verb_classes: HashMap<String, VerbClass>,
    walk_kinds: HashMap<String, FileKind>,
}

impl Taxonomy {
    /// Build the aggregate from a fact source (file, in-memory fake, …).
    /// Malformed values (non-numeric window, unknown class) are skipped — the
    /// `swipl` consult check guards full syntax upstream.
    #[must_use]
    pub fn from_source(source: &dyn FactSource) -> Self {
        Self {
            filetypes: source.pairs("filetype(").into_iter().collect(),
            time_windows: source
                .pairs("time_window(")
                .into_iter()
                .filter_map(|(k, v)| v.parse::<u64>().ok().map(|n| (k, n)))
                .collect(),
            is_a: source.pairs("is_a("),
            verb_classes: source
                .pairs("verb_class(")
                .into_iter()
                .filter_map(|(k, v)| VerbClass::parse(&v).map(|c| (k, c)))
                .collect(),
            walk_kinds: source
                .pairs("walk_kind(")
                .into_iter()
                .filter_map(|(k, v)| FileKind::parse(&v).map(|fk| (k, fk)))
                .collect(),
        }
    }

    /// Resolve a canonical verb, carrying its confirmation policy.
    #[must_use]
    pub fn verb(&self, name: &str) -> Verb {
        Verb {
            name: name.to_string(),
            class: self.verb_classes.get(name).copied(),
        }
    }

    /// Resolve a canonical object noun, carrying its walk classification.
    #[must_use]
    pub fn object(&self, name: &str) -> Object {
        Object {
            name: name.to_string(),
            walk_kind: self.walk_kinds.get(name).copied(),
        }
    }

    /// The walk kind for an object noun, or an error when it isn't
    /// file-walkable — the data-driven replacement for the old compiled
    /// `FileKind::from_object`.
    ///
    /// # Errors
    /// [`KernelError::UnwalkableObject`] when the object has no `walk_kind/2`.
    pub fn file_kind(&self, object: &str) -> Result<FileKind, KernelError> {
        self.walk_kinds
            .get(object)
            .copied()
            .ok_or_else(|| KernelError::UnwalkableObject(object.to_string()))
    }

    /// The file extension for a canonical filetype (`rust` → `rs`).
    #[must_use]
    pub fn extension_for(&self, filetype: &str) -> Option<&str> {
        self.filetypes.get(filetype).map(String::as_str)
    }

    /// The filetype value object for a canonical name, if known.
    #[must_use]
    pub fn filetype(&self, canonical: &str) -> Option<FileType> {
        self.filetypes.get(canonical).map(|ext| FileType {
            canonical: canonical.to_string(),
            extension: ext.clone(),
        })
    }

    /// The age window in seconds for a static time reference (`yesterday`).
    #[must_use]
    pub fn window_seconds(&self, time_ref: &str) -> Option<u64> {
        self.time_windows.get(time_ref).copied()
    }

    /// The time-window value object for a static reference, if known.
    #[must_use]
    pub fn time_window(&self, time_ref: &str) -> Option<TimeWindow> {
        self.time_windows
            .get(time_ref)
            .map(|&seconds| TimeWindow { seconds })
    }

    /// Canonical objects belonging to a class via `is_a/2` (e.g. members of
    /// `crate` or `repo`), in file order.
    #[must_use]
    pub fn members_of(&self, class: &str) -> Vec<&str> {
        self.is_a
            .iter()
            .filter(|(_, c)| c == class)
            .map(|(object, _)| object.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory fact source: `(head, [args...])` tuples, no file I/O.
    struct FakeFacts(Vec<(&'static str, Vec<&'static str>)>);
    impl FactSource for FakeFacts {
        fn pairs(&self, head: &str) -> Vec<(String, String)> {
            let want = head.trim_end_matches('(');
            self.0
                .iter()
                .filter(|(h, args)| *h == want && args.len() == 2)
                .map(|(_, a)| (a[0].to_string(), a[1].to_string()))
                .collect()
        }
        fn singles(&self, head: &str) -> Vec<String> {
            let want = head.trim_end_matches('(');
            self.0
                .iter()
                .filter(|(h, args)| *h == want && args.len() == 1)
                .map(|(_, a)| a[0].to_string())
                .collect()
        }
    }

    fn sample() -> Taxonomy {
        Taxonomy::from_source(&FakeFacts(vec![
            ("filetype", vec!["rust", "rs"]),
            ("time_window", vec!["yesterday", "86400"]),
            ("time_window", vec!["bogus", "notanumber"]),
            ("is_a", vec!["cli", "crate"]),
            ("is_a", vec!["actions", "crate"]),
            ("is_a", vec!["engine", "repo"]),
            ("verb_class", vec!["delete", "destructive"]),
            ("verb_class", vec!["build", "readonly"]),
            ("verb_class", vec!["push", "outward"]),
            ("walk_kind", vec!["file", "file"]),
            ("walk_kind", vec!["directory", "directory"]),
        ]))
    }

    #[test]
    fn verb_confirmation_policy_and_fail_safe() {
        let t = sample();
        assert!(!t.verb("build").needs_confirm(), "readonly proceeds");
        assert!(t.verb("delete").needs_confirm(), "destructive prompts");
        assert!(t.verb("push").needs_confirm(), "outward prompts");
        assert!(
            t.verb("mystery").needs_confirm(),
            "unclassified is fail-safe → prompt"
        );
        assert_eq!(t.verb("delete").class(), Some(VerbClass::Destructive));
    }

    #[test]
    fn file_kind_is_data_driven() {
        let t = sample();
        assert_eq!(t.file_kind("file").unwrap(), FileKind::File);
        assert_eq!(t.file_kind("directory").unwrap(), FileKind::Dir);
        assert_eq!(t.object("file").walk_kind(), Some(FileKind::File));
        assert_eq!(t.object("package").walk_kind(), None);
        let err = t.file_kind("package").unwrap_err();
        assert_eq!(err, KernelError::UnwalkableObject("package".to_string()));
    }

    #[test]
    fn filetype_and_time_window_value_objects() {
        let t = sample();
        assert_eq!(t.extension_for("rust"), Some("rs"));
        assert_eq!(t.filetype("rust").unwrap().extension(), "rs");
        assert_eq!(t.window_seconds("yesterday"), Some(86400));
        assert_eq!(t.time_window("yesterday").unwrap().seconds(), 86400);
        assert_eq!(t.window_seconds("bogus"), None, "non-numeric skipped");
        assert_eq!(
            t.window_seconds("today"),
            None,
            "today is runtime, not data"
        );
    }

    #[test]
    fn members_of_reads_is_a() {
        let t = sample();
        assert_eq!(t.members_of("crate"), vec!["cli", "actions"]);
        assert_eq!(t.members_of("repo"), vec!["engine"]);
        assert!(t.members_of("unknown").is_empty());
    }
}
