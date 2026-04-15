use crate::assert_go;

#[test]
fn bool_literals() {
    assert_go!(
        r#"
pub fn go() -> Bool {
  True
}
"#,
    );
}

#[test]
fn bool_operators() {
    assert_go!(
        r#"
pub fn go() -> Bool {
  True && False || !True
}
"#,
    );
}

#[test]
fn bool_equality() {
    assert_go!(
        r#"
pub fn go() -> Bool {
  True == False
}
"#,
    );
}
