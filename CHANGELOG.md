# Changelog

## Unreleased

- Rename shared bindings in or-patterns, including nested alternatives and
  struct shorthand. Each binding is counted once across alternatives.

## 0.1.0 - 2026-10-12

First crates.io release.

- Deterministic local renaming with conservative and reviewed binding modes.
- Nested tuple, struct, tuple-struct, slice, reference, and `@` patterns;
  independent match-arm scopes and guard bindings; `let-else` initializer scope.
- Explicit runtime string encoding with a thread-safe per-call-site cache.
- Atomic project-copy workflow with selection globs, exclusions, and JSON reports.
- Coverage requirements: CLI `--fail-on-skip` / `--min-renamed`, and macro
  `require_renames`. Report format 2 adds transformed-file and skipped-entry counts.
- Compiler diagnostics for duplicate macro options and invalid seeds.
- Differential execution tests, compiler diagnostic fixtures, release artifact
  checks, and a CI matrix for Rust 1.88/stable on Linux, Windows, and macOS.

This release does not provide compiler-level or whole-program binary obfuscation.
