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


#[test]
fn case_multi_subject_literals() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x, y {
    0, 0 -> 1
    1, 2 -> 2
    _, _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_multi_subject_mixed_literal_and_discard() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x, y {
    0, _ -> 10
    _, 0 -> 20
    _, _ -> 30
  }
}
"#,
    );
}

#[test]
fn case_multi_subject_with_variable_bindings() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x, y {
    0, n -> n
    n, 0 -> n
    a, b -> a + b
  }
}
"#,
    );
}

#[test]
fn case_multi_subject_all_variables() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x, y {
    a, b -> a * b
  }
}
"#,
    );
}

#[test]
fn case_multi_subject_evaluates_each_once() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x + 1, y * 2 {
    0, 0 -> 1
    _, _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_multi_subject_bool() {
    assert_go!(
        r#"
pub fn go(a: Bool, b: Bool) -> Int {
  case a, b {
    True, True -> 1
    True, False -> 2
    False, True -> 3
    False, False -> 4
  }
}
"#,
    );
}

#[test]
fn case_guard_simple() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    n if n > 0 -> n
    _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_guard_compound_bool() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    n if n > 0 && n < 10 -> n
    _ -> -1
  }
}
"#,
    );
}

#[test]
fn case_guard_with_or() {
    assert_go!(
        r#"
pub fn go(x: Int) -> Int {
  case x {
    n if n == 0 || n == 1 -> n
    _ -> -1
  }
}
"#,
    );
}

#[test]
fn case_guard_with_not() {
    assert_go!(
        r#"
pub fn go(x: Bool, y: Bool) -> Int {
  case x {
    a if !y -> 1
    _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_guard_references_outer_binding() {
    assert_go!(
        r#"
pub fn go(x: Int, threshold: Int) -> Int {
  case x {
    n if n > threshold -> 1
    _ -> 0
  }
}
"#,
    );
}

#[test]
fn case_guard_on_literal_pattern() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x {
    0 if y > 0 -> 1
    0 -> 2
    _ -> 3
  }
}
"#,
    );
}

#[test]
fn case_guard_multi_subject() {
    assert_go!(
        r#"
pub fn go(x: Int, y: Int) -> Int {
  case x, y {
    a, b if a > b -> a
    a, b -> b
  }
}
"#,
    );
}

#[test]
fn case_guard_string_eq() {
    assert_go!(
        r#"
pub fn go(s: String) -> Int {
  case s {
    n if n == "hi" -> 1
    _ -> 0
  }
}
"#,
    );
}
