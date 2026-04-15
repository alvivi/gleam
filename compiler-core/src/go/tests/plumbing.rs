use crate::go::{go_mod, go_package_name};

#[test]
fn go_mod_has_project_name_and_version() {
    let output = go_mod("my_app");
    assert!(output.contains("module gleam/my_app"));
    assert!(output.contains("go 1.21"));
}

#[test]
fn package_name_uses_last_segment() {
    assert_eq!(go_package_name("foo/bar/baz"), "baz");
    assert_eq!(go_package_name("single"), "single");
}

#[test]
fn package_name_sanitizes_non_alphanumeric() {
    assert_eq!(go_package_name("foo-bar"), "foo_bar");
}

#[test]
fn package_name_escapes_go_reserved_words() {
    assert_eq!(go_package_name("func"), "func_");
    assert_eq!(go_package_name("gleam/type"), "type_");
}

#[test]
fn package_name_prefixes_leading_digit_and_non_letter() {
    assert_eq!(go_package_name("9lives"), "p9lives");
    assert_eq!(go_package_name("_hidden"), "p_hidden");
}
