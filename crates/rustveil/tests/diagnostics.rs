#[test]
fn macro_diagnostics_and_feature_passthrough() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/duplicate_option.rs");
    cases.compile_fail("tests/ui/invalid_seed.rs");
    if cfg!(feature = "enabled") {
        cases.compile_fail("tests/ui/no_renames.rs");
        cases.compile_fail("tests/ui/unsupported.rs");
    } else {
        cases.pass("tests/ui/no_renames.rs");
        cases.pass("tests/ui/unsupported.rs");
    }
}
