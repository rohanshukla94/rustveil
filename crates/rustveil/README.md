# rustveil

Opt-in Rust macros for conservative local-binding renaming and runtime string
obfuscation.

Add the crate without enabling obfuscation by default, then opt in through an
application feature:

```toml
[dependencies]
rustveil = { version = "0.1", default-features = false }

[features]
obfuscation = ["rustveil/enabled"]
```

```rust
use rustveil::{obfuscate, veil_str};

#[obfuscate(seed = 42, strict, assume_bindings)]
fn total(price: u64, count: u64) -> u64 {
    let subtotal = price * count;
    subtotal
}

fn label() -> &'static str {
    veil_str!("Example label", 42)
}
```

This is reversible obfuscation, not encryption. A seed is not a secret, and
the macros do not guarantee semantic equivalence or protect embedded secrets.
See the project repository for supported syntax, costs, and validation details.

Use `#[obfuscate(require_renames)]` to fail compilation when obfuscation is
enabled but a function is skipped or no bindings are renamed. `strict` rejects
skipped functions only. Options are validated even with obfuscation disabled;
duplicate options are rejected.

Tuple, struct, slice, reference, `@`, and or-patterns and match guards are supported.
In `assume_bindings` mode, bare identifier patterns must be variable bindings.
Qualify constants and unit variants, for example `crate::LIMIT` and
`Option::None`, to keep their meaning. Macros, conditional let
patterns, nested items, const blocks/patterns, and attributes are skipped.
