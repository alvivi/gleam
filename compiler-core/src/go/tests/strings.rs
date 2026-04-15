use crate::assert_go;

#[test]
fn string_literal() {
    assert_go!(
        r#"
pub fn go() -> String {
  "hello"
}
"#,
    );
}

#[test]
fn string_concat() {
    assert_go!(
        r#"
pub fn go() -> String {
  "hello" <> " " <> "world"
}
"#,
    );
}

#[test]
fn string_with_escapes() {
    assert_go!(
        r#"
pub fn go() -> String {
  "a\nb\"c\\d"
}
"#,
    );
}

#[test]
fn string_all_simple_escapes() {
    assert_go!(
        r#"
pub fn go() -> String {
  "\r\t\f"
}
"#,
    );
}

#[test]
fn string_unicode_escape_bmp() {
    assert_go!(
        r#"
pub fn go() -> String {
  "\u{00E9}"
}
"#,
    );
}

#[test]
fn string_unicode_escape_short_hex_is_padded() {
    assert_go!(
        r#"
pub fn go() -> String {
  "\u{A}"
}
"#,
    );
}

#[test]
fn string_unicode_escape_astral() {
    assert_go!(
        r#"
pub fn go() -> String {
  "\u{1F600}"
}
"#,
    );
}

#[test]
fn string_multiline_raw_newline() {
    assert_go!(
        "
pub fn go() -> String {
  \"line one
line two\"
}
",
    );
}
