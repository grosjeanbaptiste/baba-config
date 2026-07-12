//! First-party package registry (ADR-029) — baba's *own* packages, catalogued
//! in-repo the way kag ships its presets. A [`Pack`] is the value-object mirror
//! of kag's `Preset { name, summary, basis }`: a named vocabulary pack with a
//! human `summary` and a `basis` recording how it was derived (its provenance,
//! e.g. `scanned from brew --help`). The catalog is [`Registry`], parsed from
//! `corpus/packs/registry.pl`'s `pack(name, summary, basis).` facts.
//!
//! Pure — the file read is infrastructure ([`crate::io`]); this maps already-read
//! text into typed value objects.

use crate::domain::parse::triples_from_text;

/// One first-party package entry: identity by `name`, carrying its human
/// `summary` and derivation `basis`. A value object — equality by attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// Canonical package name — also the pack file stem (`<name>.pl`).
    pub name: String,
    /// One-line human description of what the package's vocabulary covers.
    pub summary: String,
    /// How the pack was derived — its provenance (`scanned from brew --help`).
    pub basis: String,
}

/// The first-party package catalog: the ordered set of [`Pack`]s baba ships.
/// Mirrors kag's `presets::all()`. Pure aggregate over the parsed registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    packs: Vec<Pack>,
}

impl Registry {
    /// Parse a registry catalog from `registry.pl` text: one [`Pack`] per
    /// `pack(name, summary, basis).` fact, in first-seen order.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let packs = triples_from_text(text, "pack(")
            .into_iter()
            .map(|(name, summary, basis)| Pack {
                name,
                summary,
                basis,
            })
            .collect();
        Self { packs }
    }

    /// The catalogued packs, in registry order.
    #[must_use]
    pub fn packs(&self) -> &[Pack] {
        &self.packs
    }

    /// The package names, in registry order — what the loader resolves to
    /// `<name>.pl` files.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.packs.iter().map(|p| p.name.clone()).collect()
    }

    /// Look up one package by name (`None` if not catalogued).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Pack> {
        self.packs.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = "\
% baba first-party package registry
pack(brew, 'Homebrew package manager', 'scanned from brew --help').
pack(git, 'Git version control', 'scanned from git --help').
";

    #[test]
    fn from_text_reads_each_pack_fact_as_a_value_object() {
        let reg = Registry::from_text(CATALOG);
        assert_eq!(
            reg.packs(),
            &[
                Pack {
                    name: "brew".into(),
                    summary: "Homebrew package manager".into(),
                    basis: "scanned from brew --help".into(),
                },
                Pack {
                    name: "git".into(),
                    summary: "Git version control".into(),
                    basis: "scanned from git --help".into(),
                },
            ]
        );
    }

    #[test]
    fn names_lists_stems_in_registry_order() {
        assert_eq!(Registry::from_text(CATALOG).names(), vec!["brew", "git"]);
    }

    #[test]
    fn get_finds_by_name_and_misses_are_none() {
        let reg = Registry::from_text(CATALOG);
        assert_eq!(reg.get("git").unwrap().summary, "Git version control");
        assert!(reg.get("absent").is_none());
    }

    #[test]
    fn an_empty_catalog_yields_no_packs() {
        assert!(Registry::from_text("% nothing here\n").packs().is_empty());
    }
}
