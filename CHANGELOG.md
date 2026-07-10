# Changelog — baba-config

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **`VerbClass::parse` / `FileKind::parse` are now public.** They are the
  domain's single source of truth for valid `verb_class` / `walk_kind` values;
  exposing them lets the `baba-verify` taxonomy-integrity check validate the
  data file against the same parser the aggregate uses, not a duplicated list.
  Pure, no behaviour change.
- **Promoted to a domain kernel (workspace ADR-023).** Beyond the flat-fact
  reader, the crate now carries the rich, taxonomy-driven domain model both
  runtime crates share: `domain/` holds pure value objects built from
  `taxonomy.pl` — `Verb` (+ `needs_confirm`, fail-safe), `VerbClass`,
  `Object` (+ `walk_kind`), `FileKind`, `FileType`, `TimeWindow`, and the
  `Taxonomy` aggregate — plus the fact-parsing primitives, the `FactSource`
  port, and `KernelError`. `io.rs` is the infrastructure adapter
  (`FileFactSource`) that reads a `.pl` file and feeds the domain through the
  port. Stays zero-dependency (own error type, no `anyhow`). The prior free
  functions (`parse_pairs`/`parse_singles`/`resolve_data_file`/`strip_head`/
  …/`BABA_DIR`) are preserved as re-exports, so consumers build unchanged.

- Initial extraction (workspace ADR-022). The zero-dependency reader for
  Baba's Prolog-fact config files — `resolve_data_file`, `strip_head`,
  `split2` / `split3`, `unquote`, `parse_pairs`, `parse_singles`, and the
  canonical `BABA_DIR` const — lifted verbatim out of `baba-actions`'
  `src/factfile.rs` so `baba-cli` and `baba-actions` share one
  implementation instead of each carrying an ad-hoc line reader. Unit tests
  travel with the code. Consumed as a `path` dependency by both crates.
