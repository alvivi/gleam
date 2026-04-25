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

#[test]
fn direct_call_same_module_public() {
    assert_go!(
        r#"
pub fn double(x: Int) -> Int {
  x + x
}

pub fn four() -> Int {
  double(2)
}
"#,
    );
}

#[test]
fn direct_call_same_module_private() {
    assert_go!(
        r#"
fn helper(x: Int) -> Int {
  x + 1
}

pub fn go() -> Int {
  helper(41)
}
"#,
    );
}

#[test]
fn trailing_underscore_name_does_not_collide() {
    // Gleam allows `foo_` alongside `foo` (the trailing underscore is the
    // conventional way to escape a keyword as an identifier). Naive
    // `snake_case` → `camelCase` folding drops the underscore and collapses
    // both into the same Go name, which would emit two clashing `func foo`
    // declarations.
    assert_go!(
        r#"
fn foo() -> Int {
  1
}

fn foo_() -> Int {
  2
}

pub fn go() -> Int {
  foo() + foo_()
}
"#,
    );
}

#[test]
fn pipeline_call() {
    assert_go!(
        r#"
pub fn inc(x: Int) -> Int {
  x + 1
}

pub fn double(x: Int) -> Int {
  x + x
}

pub fn go() -> Int {
  1 |> inc |> double
}
"#,
    );
}

#[test]
fn multiple_args() {
    assert_go!(
        r#"
pub fn add(a: Int, b: Int) -> Int {
  a + b
}

pub fn go() -> Int {
  add(1, 2)
}
"#,
    );
}

#[test]
fn anonymous_function_in_let() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let f = fn(x: Int) -> Int { x + 1 }
  f(2)
}
"#,
    );
}

#[test]
fn anonymous_function_called_immediately() {
    assert_go!(
        r#"
pub fn go() -> Int {
  fn(x: Int) -> Int { x * 2 }(5)
}
"#,
    );
}

#[test]
fn anonymous_function_with_multiple_args() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let add = fn(a: Int, b: Int) -> Int { a + b }
  add(3, 4)
}
"#,
    );
}

#[test]
fn closure_captures_outer_variable() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let n = 10
  let add_n = fn(x: Int) -> Int { x + n }
  add_n(5)
}
"#,
    );
}

#[test]
fn closure_param_shadows_outer_binding() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let x = 1
  let f = fn(x: Int) -> Int { x }
  f(99) + x
}
"#,
    );
}

#[test]
fn nested_closures() {
    assert_go!(
        r#"
pub fn go() -> Int {
  let make_adder = fn(a: Int) -> fn(Int) -> Int {
    fn(b: Int) -> Int { a + b }
  }
  make_adder(10)(5)
}
"#,
    );
}

#[test]
fn higher_order_passing_top_level_fn() {
    assert_go!(
        r#"
fn double(x: Int) -> Int {
  x * 2
}

fn apply(f: fn(Int) -> Int, x: Int) -> Int {
  f(x)
}

pub fn go() -> Int {
  apply(double, 5)
}
"#,
    );
}

#[test]
fn higher_order_passing_anonymous_fn() {
    assert_go!(
        r#"
fn apply(f: fn(Int) -> Int, x: Int) -> Int {
  f(x)
}

pub fn go() -> Int {
  apply(fn(x: Int) -> Int { x + 100 }, 1)
}
"#,
    );
}

#[test]
fn capture_syntax() {
    assert_go!(
        r#"
fn add(a: Int, b: Int) -> Int {
  a + b
}

pub fn go() -> Int {
  let inc = add(_, 1)
  inc(41)
}
"#,
    );
}

#[test]
fn closure_returning_nil() {
    assert_go!(
        r#"
pub fn go() -> fn() -> Nil {
  fn() { Nil }
}
"#,
    );
}
