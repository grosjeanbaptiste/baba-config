# baba-config

The one piece of Rust shared by both Baba runtime sub-repos. A minimal,
**zero-dependency** reader for Baba's Prolog-fact config files
(`taxonomy.pl`, `variables.pl`, and the domain pack's flat facts).

## Why a shared crate at all

The other sub-repos (`engine`, `cli`, `actions`, `corpus`) are deliberately
independent (ADR-003: sub-repos over monorepo). This crate is the one
sanctioned exception: the flat-fact line reader was being copy-pasted between
`baba-cli` and `baba-actions`, drifting out of sync and testable only in one
of them. ADR-022 records the trade-off — a compile-time `path` coupling
between two otherwise-independent repos, accepted to kill the duplication.

## What it is not

Not a Prolog reader. It lifts `head(a, b).` / `head(x).` facts into Rust and
nothing more; full syntax is validated by `swipl` consult in corpus CI.

## Consumers

```toml
# in baba-cli/Cargo.toml and baba-actions/Cargo.toml
baba-config = { path = "../config" }
```

- `baba-cli` — `link_parser_flag/1` from `variables.pl`.
- `baba-actions` — `filetype/2`, `time_window/2`, `is_a/2`, `verb_class/2`,
  `limit/2`, `repo_slug_prefix/1`, and the pack graph checks.
