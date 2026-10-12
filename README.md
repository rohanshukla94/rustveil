# RustVeil

A small, inspectable Rust source-obfuscation toolkit with a project-copy CLI and
opt-in Cargo macros. Source is included for every component. This is an initial
implementation with regression tests, not a compiler-grade whole-program
obfuscator or a guarantee against reverse engineering.

## What it does

| Component | Purpose |
| --- | --- |
| `rustveil-core` | Scope-aware local-binding renaming using the `syn` AST |
| `rustveil-cli` | Copy a project, transform selected Rust files, emit a JSON report |
| `rustveil-macros` | `#[obfuscate(...)]` and `veil_str!(...)` procedural macros |
| `rustveil` | Public macro facade and small safe-Rust string decoder |
| `workspace-app` | Working example, including a renamed dependency (`rv`) |

Local renaming handles nested blocks, shadowing, initializer scope, assignments,
closure and loop bindings, tuple/struct/slice/reference destructuring, `@`
bindings, `let-else`, and match arms with guards. Struct field names are preserved
in both expressions and patterns by expanding shorthand when needed. It
preserves function names, function parameters, type names, struct field names,
lifetimes, public interfaces, and serialization names. Identifier seeds are
deterministic; they are not secrets.

There are two binding modes:

* **Default:** rename explicit `mut`/`ref`/`@` bindings and subsequent shadowing of
  known renamed bindings. Leave ambiguous bare identifier patterns unchanged.
* **Reviewed:** `assume_bindings` also renames ordinary immutable locals and
  closure/loop/match bindings. You assert that bare identifier patterns are
  variable bindings, not unit structs, constants, or enum variants. Qualify
  variants and constants (for example, `Option::None` and `crate::LIMIT`)
  before enabling this mode. Do not enable this over an unreviewed
  workspace. A successful compile alone cannot prove equivalent behavior.

Rust allows `let Unit = value;` to be a pattern, rather than a variable binding.
A syntax parser cannot fully resolve that distinction. Both modes skip entire
functions containing macros, or-patterns, conditional let patterns,
nested items, const patterns, inline const blocks, or body/signature
attributes. The CLI also skips attributed functions, modules, impls, traits,
and files. This includes harmless documentation and lint attributes: coverage
is intentionally conservative. Skips are visible in the CLI report; macro
`strict` mode turns a skip into a compiler error. `strict` does not require that
at least one identifier was renamed. Use `#[obfuscate(require_renames)]` to
reject both skipped functions and functions with no renamed bindings. These
checks apply only with the `enabled` feature; option syntax is always validated.

## Install from crates.io

```sh
cargo add rustveil@0.1
cargo install rustveil-cli --locked
```

The library's `enabled` feature is opt-in. See the integration example below.
The design assessment and next milestones are in [ROADMAP.md](ROADMAP.md).

## Try the workspace example

From this directory, with a Rust toolchain installed:

```sh
cargo test --workspace --all-features --locked
cargo run -p workspace-app
cargo run -p workspace-app --release --features obfuscation
```

Both runs print:

```text
RustVeil workspace demo — ready: 1710
```

The source targets Rust 2021. See `VALIDATION.md` for the actual tested compiler
and commands. Only the tested toolchain is claimed as validated.

## Integrate into a large Cargo workspace

Keep a sibling checkout of this toolkit, with its workspace manifest intact.
In **your workspace root**:

```toml
[workspace.dependencies]
rustveil = { path = "../rustveil/crates/rustveil", default-features = false }
```

In each crate that uses the macros:

```toml
[dependencies]
rustveil.workspace = true

[features]
obfuscation = ["rustveil/enabled"]
```

Example application code:

```rust
use rustveil::{obfuscate, veil_str};

// Reviewed: each simple pattern in this helper is a variable binding.
#[obfuscate(seed = 42, strict, assume_bindings)]
pub fn calculate_total(unit_price: u64, count: u64) -> u64 {
    let subtotal = unit_price.saturating_mul(count);
    let discount = if count >= 10 { subtotal / 20 } else { 0 };
    subtotal - discount
}

pub fn product_name() -> &'static str {
    veil_str!("Example product", 42)
}
```

Build a selected application with its package feature enabled:

```sh
cargo test -p your-app
cargo test -p your-app --features obfuscation
cargo build -p your-app --release --features obfuscation
```

Without the `enabled` feature, the attribute is a passthrough and `veil_str!`
expands to the original literal. Features are additive and unified by Cargo:
if another selected package enables `rustveil/enabled`, it can enable the same
dependency instance for other packages in that build. Use separate Cargo
invocations when you need independent feature configurations. This is an
explicit feature switch, not an automatic check for the release profile.

Use the attribute on ordinary functions or methods. Attribute-macro ordering
and third-party instrumentation can be complicated; keep transformed logic in
an unannotated helper and keep logging, tracing, derives, and async-trait
wrappers outside it. A function containing even `println!` or `veil_str!` is
skipped by the local renamer; the string macro itself still expands normally.

Suggested root release settings, after measuring their effect in your project:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
debug = false
```

## Create an obfuscated source copy

Install the included CLI locally:

```sh
cargo install --path crates/cli --locked
```

Inspect a selected source tree first. The output parent must already exist,
and the output directory must not exist or be inside the input:

```sh
rustveil \
  --input /projects/my-app \
  --output /projects/my-app-obfuscated \
  --select 'crates/domain/src/**/*.rs' \
  --seed 42 \
  --dry-run
```

Remove `--dry-run` to create the copy. Add `--assume-bindings` only for source
selections whose simple patterns you have reviewed. Repeat `--select` to select
several trees. All other regular files are copied byte-for-byte, except default
or explicit exclusions:

```sh
rustveil \
  --input /projects/my-app \
  --output /projects/my-app-obfuscated \
  --select 'crates/domain/src/**/*.rs' \
  --select 'crates/engine/src/**/*.rs' \
  --exclude 'vendor' \
  --exclude '.env*' \
  --seed 42

cargo test --manifest-path /projects/my-app-obfuscated/Cargo.toml --workspace
```

The CLI writes its report to stdout and `.rustveil-report.json` inside the copy.
Report format 2 includes `selected_files`, `transformed_files`, `renamed`, and
`skipped_entries`. A skipped entry can represent an entire attributed module,
impl, trait, or file, so it is not a count of skipped functions.
Use `--min-renamed 1` to reject empty selections and runs that rename nothing.
Use `--fail-on-skip` when every selected entry must be supported. Both checks
also apply to `--dry-run`; failures exit nonzero and do not publish an output.
The report records file paths, function names, rename counts, and skip reasons;
it is an internal build artifact, not something to ship to customers. It does
not contain an original-to-renamed variable map. Default exclusions are `.git`,
`target`, `node_modules`, and a pre-existing `.rustveil-report.json`.

The original tree is never edited. Parsing/copying happens in a staging directory
and failed processing does not publish a partial output. Existing output trees
are refused. Do not run multiple processes against the same output path.
Symlinks and special files are rejected rather than followed. File permissions
are preserved; directory metadata, timestamps, ACLs, and extended attributes are
not reproduced. The input should not change during copying.

This is a directory transformer, not a Cargo dependency-graph resolver. Include
the entire workspace in `--input`; dependencies outside it remain external.
Build scripts relying on `.git`, absolute paths, custom source parsers, generated
code, or relative external dependencies need project-specific validation.

## String macro behavior and costs

```rust
let label: &'static str = rustveil::veil_str!("An example label", 123);
```

With obfuscation enabled, the procedural macro emits encoded bytes and a seed.
A decoder initializes a `OnceLock<String>` once per call site, then returns an
`&'static str`. It supports Unicode, raw string literals, empty strings, and NUL
bytes. It requires `std`; it is not a `no_std` facility.

The first access performs O(length) decoding and allocates for nonempty data;
later accesses use the cached result. Decoded strings remain in memory until
process exit. Different call sites have separate caches. Using this on many
large literals increases retained memory and can increase binary size.

This is reversible obfuscation, **not encryption or secret storage**. The seed
and decoder are shipped with the application. `black_box` and `inline(never)`
are optimizer hints, not security boundaries or guarantees that plaintext will
never appear in a binary. Inspect your final artifact and relevant toolchain.
This does not transform every literal automatically. Do not use it where a
literal is required: const/static initializers, attributes, patterns, or the
format-string position of `format!`/`println!`. Use `println!("{}", veil_str!("x"))`
instead. Never rely on embedded client-side credentials being kept secret.

## Limits that matter

* Local renaming mostly makes **distributed source** harder to read. Optimized
  Rust binaries generally do not retain local variable names anyway.
* This does not rename functions across crates, types, fields, exported symbols,
  or APIs. It has no control-flow flattening, virtualization, or opaque predicates.
* The source CLI does not encode strings; string encoding is the explicit macro.
* Rewritten files are regenerated with `prettyplease`: ordinary comments are
  lost, documentation attributes are retained, and formatting/line numbers
  change. Keep license notices in a separate LICENSE file and retain originals.
* Even skipped functions inside a rewritten file may be reformatted. Source-text
  or location-sensitive constructs (`stringify!`, `include_str!`, `line!`,
  `column!`, procedural macros that inspect spans/text, source snapshots) require
  exclusions or separate review. There is no global semantic-equivalence claim.
* Do not treat skipped code as obfuscated. Review the report and test relevant
  feature combinations, targets, integration flows, and benchmarks.

For large projects, start with a few self-contained logic functions and selected
strings. Keep source-control originals readable, make output disposable, and
run the project's own checks on the result before shipping it.

## References

* [syn AST traversal](https://docs.rs/syn/2/syn/visit_mut/index.html)
* [Rust name resolution](https://doc.rust-lang.org/reference/names/name-resolution.html)
* [OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
* [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification)

## License

MIT. See `LICENSE`. Dependency licenses remain their respective authors' licenses.
