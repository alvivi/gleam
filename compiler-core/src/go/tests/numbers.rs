use crate::assert_go;

#[test]
fn int_literals() {
    assert_go!(
        r#"
pub fn go() -> Int {
  42
}
"#,
    );
}

#[test]
fn int_arithmetic() {
    assert_go!(
        r#"
pub fn go() -> Int {
  1 + 2 * 3 - 4 / 2 % 3
}
"#,
    );
}

#[test]
fn int_negate() {
    assert_go!(
        r#"
pub fn go() -> Int {
  -7
}
"#,
    );
}

#[test]
fn float_literals() {
    assert_go!(
        r#"
pub fn go() -> Float {
  1.5
}
"#,
    );
}

#[test]
fn float_arithmetic() {
    assert_go!(
        r#"
pub fn go() -> Float {
  1.0 +. 2.0 *. 3.5 -. 4.0 /. 2.0
}
"#,
    );
}

#[test]
fn int_div() {
    assert_go!(
        r#"
pub fn go(a: Int, b: Int) -> Int {
  a / b
}
"#,
    );
}

#[test]
fn int_remainder() {
    assert_go!(
        r#"
pub fn go(a: Int, b: Int) -> Int {
  a % b
}
"#,
    );
}

#[test]
fn float_div() {
    assert_go!(
        r#"
pub fn go(a: Float, b: Float) -> Float {
  a /. b
}
"#,
    );
}

#[test]
fn no_prelude_when_unused() {
    assert_go!(
        r#"
pub fn go(a: Int, b: Int) -> Int {
  a + b
}
"#,
    );
}

#[test]
fn int_comparison() {
    assert_go!(
        r#"
pub fn go() -> Bool {
  1 < 2
}
"#,
    );
}
