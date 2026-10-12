# Contributing

Use a feature branch and keep readable sources under version control. For new
transforms, include original-versus-transformed compile/run fixtures and a test
that unsupported neighboring syntax is left unchanged. Do not weaken the
conservative mode by guessing which bare patterns name constants or variants.

Before submitting:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.88.0 test --workspace --all-features --locked
python scripts/check_release.py
```

Keep documentation, diagnostics, and report-format changes synchronized.
Performance or protection claims need a reproducible measurement and comparison
to the original build. See ROADMAP.md for priorities and known limitations.
