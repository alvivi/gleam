use crate::assert_go;

#[test]
fn basic_let() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  x + 2
}
"#,
    );
}

#[test]
fn multiple_lets() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  let y = 2
  x + y
}
"#,
    );
}

#[test]
fn shadowing() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  let x = x + 10
  let x = x + 100
  x
}
"#,
    );
}

#[test]
fn parameter_shadowing() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  let x = x + 1
  x
}
"#,
    );
}

#[test]
fn discard_assignment() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let _ = 99
  1
}
"#,
    );
}

#[test]
fn unused_binding() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  2
}
"#,
    );
}

#[test]
fn reserved_word_as_binding() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let type_ = 1
  type_
}
"#,
    );
}
