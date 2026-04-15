use crate::assert_go;

#[test]
fn simple_block() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = { 1 + 2 }
  x
}
"#,
    );
}

#[test]
fn block_with_let() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let y = {
    let a = 10
    let b = 20
    a + b
  }
  y
}
"#,
    );
}

#[test]
fn block_does_not_leak_shadowed_name() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  let y = {
    let x = 100
    x
  }
  x + y
}
"#,
    );
}

#[test]
fn nested_blocks() {
    assert_go!(
        r#"
pub fn go() -> Int {
  {
    {
      1 + 2
    }
  }
}
"#,
    );
}

#[test]
fn block_returning_bool() {
    assert_go!(
        r#"
pub fn go() -> Bool {
  { True && False }
}
"#,
    );
}
