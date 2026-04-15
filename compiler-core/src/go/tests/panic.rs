use crate::assert_go;

#[test]
fn plain_panic() {
    assert_go!(
        r#"
pub fn go() -> Int {
  panic
}
"#,
    );
}

#[test]
fn panic_with_message() {
    assert_go!(
        r#"
pub fn go() -> Int {
  panic as "something is wrong"
}
"#,
    );
}

#[test]
fn panic_returning_nil() {
    assert_go!(
        r#"
pub fn go() -> Nil {
  panic
}
"#,
    );
}

#[test]
fn panic_inside_block() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = { panic as "fail" }
  x
}
"#,
    );
}
