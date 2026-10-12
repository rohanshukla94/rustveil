# rustveil-macros

Procedural macro implementation for the `rustveil` facade. Most applications
should depend on `rustveil` rather than using this crate directly.

Enable the facade's `enabled` feature in the build where you want obfuscation:

```toml
[dependencies]
rustveil = { version = "0.1", default-features = false }

[features]
obfuscation = ["rustveil/enabled"]
```

`#[obfuscate(...)]` renames selected local bindings. `veil_str!(...)` encodes a
string at compile time and decodes it at runtime. This is reversible
obfuscation, not encryption or secret storage. Review the `rustveil` crate
documentation for limits and examples.

Attribute options are `seed = <u64>`, `strict`, `assume_bindings`, and
`require_renames`. Duplicate options and invalid seeds are compiler errors.
With the `enabled` feature, `strict` rejects unsupported syntax;
`require_renames` additionally requires at least one renamed binding.
Without that feature, both coverage checks are disabled.

Destructuring and match arms with guards are supported. In `assume_bindings`
mode, all bare identifier patterns must be bindings: qualify constants and
unit variants (write `Option::None`, not `None`). Or-pattern alternatives share
one renamed binding per name. Conditional let patterns, nested items, macros,
and attributes remain conservatively skipped.
