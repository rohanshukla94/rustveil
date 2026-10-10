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