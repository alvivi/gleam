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
