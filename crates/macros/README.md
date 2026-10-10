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