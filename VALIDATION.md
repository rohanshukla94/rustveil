# Validation record

Release 0.1.0, checked on 2026-10-12 on Windows x86_64.

## Locally completed checks

| Check | Result |
| --- | --- |
| Rust 1.95.0, default workspace tests | 22 tests + 1 doctest passed |
| Rust 1.95.0, all-features workspace tests | 22 tests + 1 doctest passed |
| Rust 1.88.0, all-features workspace tests | 22 tests + 1 doctest passed |
| Four compiler diagnostic fixtures | Expected failures with enabled; coverage fixtures pass when disabled |
| Formatting and Clippy with warnings denied | Passed |
| Documentation with rustdoc warnings denied | Passed |
| Workspace package verification | All package archives built successfully |
| Release artifact comparison, Rust 1.95.0 | Identical runtime output with feature off/on |
| Plaintext marker in plain release binary | Present; binary 125,952 bytes |
| Plaintext marker in encoded release binary | Absent; binary 129,536 bytes |
| Per-crate publish dry runs | All four archives verified, with MIT license files |
| crates.io publication | core, macros, facade, and CLI 0.1.0 published and available |
| Fresh registry-only consumer with dependency alias `rv` | Both release modes printed `registry-ok:7` |
| Published CLI installation into a temporary workspace prefix | Version and help checks passed, including both coverage flags |

The published source is commit `af9cc38`, tagged `v0.1.0` on GitHub.

Commands are in CONTRIBUTING.md. Compiler versions were rustc 1.95.0
(59807616e 2026-04-14) and rustc 1.88.0 (6b00bc388 2025-06-23).
The release-artifact check uses the renamed `rv` dependency in workspace-app.

## Behavior covered

- Nested scopes, shadowing, initializer scope, delayed initialization, closures,
  loops, mutable references, raw identifiers, methods, and const functions.
- Tuple, struct, tuple-struct, slice/rest, reference and @ bindings; match guards,
  arm-local shadowing, failed guards, and restoration of outer bindings.
- let-else fallback scope and struct shorthand field-name preservation.
- Conservative treatment of bare variants/constants and rejection without
  partial changes for unsupported syntax and pattern attributes.
- Determinism, feature passthrough, duplicate-option/invalid-seed diagnostics,
  and require_renames failures for skipped or unchanged functions.
- UTF-8, NUL, empty/raw strings, static lifetime, and concurrent cache access.
- Copy selection, exclusions, dry run, coverage requirements, path checks,
  parse-error cleanup, staging cleanup, and source preservation.
- Compilation and execution of original/transformed fixtures in both binding modes.

## Scope of the evidence

The CI matrix is configured for Linux, Windows, and macOS on Rust 1.88 and
stable. Linux/macOS runs of this revision have not been observed locally;
configuration alone is not a validation result. Unix symlink tests run only
on Unix. no_std is unsupported by the runtime string facility.

Marker absence is a regression check for this example, target, and compiler,
not proof that all plaintext is hidden or that analysis cannot recover it.
Tests cover the bundled fixtures, not arbitrary workspaces. The transformer
has no compiler name resolution or general semantic-equivalence proof.
