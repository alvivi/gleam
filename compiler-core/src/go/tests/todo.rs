use crate::assert_go;

#[test]
fn plain_todo() {
    assert_go!(
        r#"
pub fn go() -> Int {
  todo
}
"#,
    );
}

#[test]
fn todo_with_message() {
    assert_go!(
        r#"
pub fn go() -> Int {
  todo as "implement me"
}
"#,
    );
}
