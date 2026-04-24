#[test]
fn derive_parse_rejects_recursive_types_without_opt_in() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/recursive_without_opt_in.rs");
}

#[test]
fn derive_parse_allows_recursive_types_with_opt_in() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/recursive_with_opt_in.rs");
}
