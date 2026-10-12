# rustveil-cli

`rustveil` copies a Rust project and transforms selected `.rs` files in the
copy. The input project is not modified.

Install the command-line tool with:

```sh
cargo install rustveil-cli --locked
```

Inspect a selection before creating the output:

```sh
rustveil \
  --input /projects/my-app \
  --output /projects/my-app-obfuscated \
  --select 'crates/domain/src/**/*.rs' \
  --seed 42 \
  --dry-run
```

Remove `--dry-run` to write the copy. Review `.rustveil-report.json`, then run
your own tests against the output. The tool is syntax-based; it does not
resolve names or prove semantic equivalence. Read the repository documentation
for selection rules, exclusions, and unsupported syntax.

For build pipelines, add `--min-renamed 1` to reject empty selections or runs
that rename nothing. Add `--fail-on-skip` to reject unsupported selected code.
Both policies apply to dry runs and fail before publishing any output copy.
Report format 2 includes transformed-file and skipped-entry counts. A skipped
entry can represent a whole module or file, not just an individual function.
