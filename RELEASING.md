# Releasing

Run CONTRIBUTING.md checks, inspect the package file lists, update CHANGELOG.md
and VALIDATION.md with actual results, and commit the reviewed release source.
Use the Cargo credential provider configured by `cargo login`; keep tokens out
of scripts, command arguments, and version control.

The example has `publish = false`. Publish the other packages in this order:

1. `rustveil-core`
2. `rustveil-macros`
3. `rustveil`
4. `rustveil-cli`

For each package, first run `cargo publish -p NAME --dry-run --locked`, then
`cargo publish -p NAME --locked`. Wait for Cargo's registry availability check
before moving to a dependent crate. For the first release, dependent dry runs
need the preceding crates to exist in the registry. Verify all workspace
packages together with `cargo package --workspace --locked` before upload.

Verify the published facade from a fresh consumer with the dependency renamed
to `rv`, with and without the `enabled` feature. Install the published CLI into
a temporary prefix and inspect `rustveil --help`. Create a version tag pointing
to the release commit and publish the feature branch/tag for review. Update
workspace dependency versions together on future releases.

Cargo's official publishing guide:
https://doc.rust-lang.org/cargo/reference/publishing.html
