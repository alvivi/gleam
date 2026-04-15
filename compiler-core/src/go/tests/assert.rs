use crate::assert_go;

#[test]
fn assert_literal() {
    assert_go!(
        r#"
pub fn go() -> Int {
  assert True
  1
}
"#,
    );
}

#[test]
fn assert_expression() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  assert x > 0
  x
}
"#,
    );
}

#[test]
fn assert_with_message() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  assert x > 0 as "must be positive"
  x
}
"#,
    );
}
