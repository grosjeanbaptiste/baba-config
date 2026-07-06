# Changelog — baba-config

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial extraction (workspace ADR-022). The zero-dependency reader for
  Baba's Prolog-fact config files — `resolve_data_file`, `strip_head`,
  `split2` / `split3`, `unquote`, `parse_pairs`, `parse_singles`, and the
  canonical `BABA_DIR` const — lifted verbatim out of `baba-actions`'
  `src/factfile.rs` so `baba-cli` and `baba-actions` share one
  implementation instead of each carrying an ad-hoc line reader. Unit tests
  travel with the code. Consumed as a `path` dependency by both crates.
