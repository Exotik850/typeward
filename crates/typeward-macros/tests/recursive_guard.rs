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

#[test]
fn recursive_derive_runtime_guard_stops_same_offset_recursion() {
    use typeward::parse::Parse;
    use typeward_macros::Parse;

    #[derive(Parse, Debug)]
    #[parse(recursive)]
    enum RecursiveLoop {
        Next(Vec<RecursiveLoop>),
    }

    let err = RecursiveLoop::parse("input").expect_err("expected recursion guard failure");
    assert!(err.is_fatal());
    assert!(err.to_string().contains("made no progress"));
}
