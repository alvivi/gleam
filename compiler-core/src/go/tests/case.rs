use crate::assert_go;

#[test]
fn case_int_literals() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    0 -> 100
    1 -> 200
    _ -> 999
  }
}
"#,
    );
}

#[test]
fn case_float_literals() {
    assert_go!(
        r#"
pub fn go(x: Float) -> Int {
  case x {
    0.0 -> 1
    1.5 -> 2
    _ -> 3
  }
}
"#,
    );
}

#[test]
fn case_string_literals() {
    assert_go!(
        r#"
pub fn go(x: String) -> Int {
  case x {
    "hello" -> 1
    "world" -> 2
    _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_bool_literals() {
    assert_go!(
        r#"
pub fn go(x: Bool) -> Int {
  case x {
    True -> 1
    False -> 0
  }
}
"#,
    );
}

#[test]
fn case_returns_string() {
    assert_go!(
        r#"
pub fn go(x: Int) -> String {
  case x {
    1 -> "one"
    _ -> "other"
  }
}
"#,
    );
}

#[test]
fn case_with_complex_subject() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x + y {
    0 -> 1
    _ -> 2
  }
}
"#,
    );
}

#[test]
fn case_in_let() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  let result = case x {
    0 -> 100
    _ -> 200
  }
  result + 1
}
"#,
    );
}

#[test]
fn case_variable_pattern() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    n -> n + 1
  }
}
"#,
    );
}

#[test]
fn case_literal_then_variable_catch_all() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    0 -> 100
    n -> n * 2
  }
}
"#,
    );
}

#[test]
fn case_variable_pattern_shadows_outer() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  let x = x + 1
  case x {
    x -> x * 10
  }
}
"#,
    );
}

#[test]
fn case_variable_binding_does_not_leak_out_of_clause() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  let result = case x {
    n -> n + 1
  }
  result
}
"#,
    );
}

#[test]
fn case_each_clause_has_independent_binding() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    0 -> 0
    n -> n
  }
}
"#,
    );
}

#[test]
fn case_named_discard() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    _ignored -> 42
  }
}
"#,
    );
}

