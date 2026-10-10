# rustveil-core

`rustveil-core` renames selected local value bindings in Rust functions. It is
used by the RustVeil procedural macros and command-line tool.

The transformer works on Rust syntax; it does not perform name resolution or
prove that transformed code is equivalent. By default it changes explicit
`mut` and `ref` bindings and known shadowing. The `assume_bindings` option also
changes simple bare patterns, which can instead name a constant or unit struct.
Review selected code and run your own tests after transforming it.

Most applications should use the `rustveil` facade or the `rustveil-cli`
command instead of depending on this implementation crate directly.