# Validation record

Validated on Linux x86_64 with `rustc 1.99.0 (b940084d7 2026-09-28)`.
The workspace declares Rust 1.88 because its locked CLI dependencies require it;
the 1.88 toolchain itself was not tested. Use the included Cargo.lock for
reproducible dependency selection. Windows validation is listed below. macOS
and no_std were not tested; the runtime string facility requires std.

## Windows validation

Validated on Windows x86_64 with `rustc 1.95.0 (59807616e 2026-04-14)`.

| Check | Result |
| --- | --- |
| `cargo test --workspace --all-features --locked` | 13 tests passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo package --workspace --allow-dirty --no-verify` | All workspace packages assembled |
| `cargo publish -p rustveil-core --dry-run --allow-dirty` | Archive verification passed; nothing uploaded |

The Rust 1.88 toolchain was not tested.

## Completed checks

| Check | Result |
| --- | --- |
| `cargo test --workspace --locked` | 14 tests passed |
| `cargo test --workspace --all-features --locked` | 14 tests passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo fmt --all` | Source formatted |
| Release example with the obfuscation feature enabled | Ran successfully; output was `RustVeil workspace demo — ready: 1710` |
| Separate Cargo consumer using dependency alias `rv` | Enabled/disabled builds produced identical output |
| Strict attribute on a function containing a macro | Rejected with the expected compiler error |

## Behavior covered

* Local shadowing and initializer evaluation under the previous binding.
* Nested block scopes, simple closures, loop scopes, mutable references,
  delayed initialization, raw identifiers, methods, and const functions.
* Preservation of struct field labels when a shorthand expression is renamed.
* Macro passthrough and rejection of unsupported syntax without partial edits.
* Deterministic seeds and conservative treatment of ambiguous unit patterns.
* Unicode, NUL bytes, raw/empty strings, static lifetime, and concurrent cache access.
* Project copying, selection globs, dry-run, exclusions, output-path checks,
  symlink rejection, parse-error cleanup, and source preservation.
* Compiling both the original and transformed project fixtures and comparing
  their runtime output in both binding modes.

## Scope of these results

The tests validate the bundled fixtures and an external consumer. They do not
establish compatibility with an arbitrary large workspace. Run your own test
suite, feature matrix, and target builds before shipping transformed output.
The CLI is syntax-based and does not invoke rustc for name resolution or prove
semantic equivalence. It does not itself run or execute the project being copied.
