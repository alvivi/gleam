use crate::assert_go;

#[test]
fn public_function_with_int_return() {
    assert_go!(
        r#"
pub fn answer() -> Int {
  42
}
"#,
    );
}

#[test]
fn private_function() {
    assert_go!(
        r#"
fn helper() -> Int {
  1
}
"#,
    );
}

#[test]
fn snake_case_name_becomes_pascal_or_camel() {
    assert_go!(
        r#"
pub fn two_words() -> Int {
  0
}

fn three_word_name() -> Int {
  0
}
"#,
    );
}

#[test]
fn function_with_args() {
    assert_go!(
        r#"
pub fn add(a: Int, b: Int) -> Int {
  a + b
}
"#,
    );
}

#[test]
fn function_returning_nil() {
    assert_go!(
        r#"
pub fn noop() -> Nil {
  Nil
}
"#,
    );
}

#[test]
fn reserved_word_as_function_name() {
    assert_go!(
        r#"
pub fn type_() -> Int {
  0
}
"#,
    );
}
