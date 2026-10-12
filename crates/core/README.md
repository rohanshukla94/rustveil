# rustveil-core

`rustveil-core` renames selected local value bindings in Rust functions. It is
used by the RustVeil procedural macros and command-line tool.

The transformer works on Rust syntax; it does not perform name resolution or
prove that transformed code is equivalent. By default it changes explicit
`mut`, `ref`, and `@` bindings and known shadowing. Nested destructuring and
match arms with guards are supported. The `assume_bindings` option also changes
bare identifier patterns, which can instead name a constant, unit struct, or
enum variant. Qualify these paths (such as `Option::None`) in reviewed mode.
Review selected code and run your own tests after transforming it.

Most applications should use the `rustveil` facade or the `rustveil-cli`
command instead of depending on this implementation crate directly.
