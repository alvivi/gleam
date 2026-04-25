use crate::assert_go;

#[test]
fn use_simple() {
    assert_go!(
        r#"
fn with_int(callback: fn(Int) -> Int) -> Int {
  callback(5)
}

pub fn go() -> Int {
  use n <- with_int
  n + 1
}
"#,
    );
}

#[test]
fn use_chained() {
    assert_go!(
        r#"
fn with_a(callback: fn(Int) -> Int) -> Int {
  callback(1)
}

fn with_b(callback: fn(Int) -> Int) -> Int {
  callback(2)
}

pub fn go() -> Int {
  use a <- with_a
  use b <- with_b
  a + b
}
"#,
    );
}

#[test]
fn use_with_no_pattern() {
    assert_go!(
        r#"
fn with_unit(callback: fn() -> Int) -> Int {
  callback()
}

pub fn go() -> Int {
  use <- with_unit
  42
}
"#,
    );
}

#[test]
fn use_with_args_to_rhs() {
    assert_go!(
        r#"
fn try_int(value: Int, callback: fn(Int) -> Int) -> Int {
  callback(value + 1)
}

pub fn go() -> Int {
  use n <- try_int(10)
  n * 2
}
"#,
    );
}
