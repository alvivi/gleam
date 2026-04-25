// A small Gleam program exercising the Go-backend feature set through
// M3:
//
//   M2: literals, arithmetic and comparison, `let` bindings with
//       shadowing, block expressions, direct and recursive calls,
//       and `assert`.
//   M3: `case` on literals, variable bindings and guards;
//       anonymous functions, closures, captures; `use` sugar.
//
// FFI (`@external(go, ...)`) is not yet lowered to Go (that is M6),
// so this fixture deliberately avoids calling out to the Go standard
// library. A hand-written `package main` wrapper that imports this
// package and prints the result of `run` is the manual way to
// exercise the output with `go run`; see README.md.

pub fn run() -> Int {
  let base = double(5)
  let shadowed = {
    let base = base + 1
    base * 3
  }
  assert base < shadowed
  let m2_result = shadowed - checked_div(shadowed, 3)

  // M3: `case` with guards and a variable catch-all.
  let classified = classify(7) + classify(0) + classify(-3)

  // M3: closure capturing an outer binding.
  let bias = 100
  let bias_apply = fn(x: Int) -> Int { x + bias }
  let closure_result = bias_apply(11)

  // M3: capture syntax desugars to an anonymous function.
  let inc = add(_, 1)
  let captured_result = inc(5)

  // M3: `use` sugar — desugars into a nested closure call.
  let use_result = with_double(use_demo)

  m2_result + classified + closure_result + captured_result + use_result
}

fn double(n: Int) -> Int {
  n + n
}

fn checked_div(a: Int, b: Int) -> Int {
  a / b
}

fn add(a: Int, b: Int) -> Int {
  a + b
}

fn classify(n: Int) -> Int {
  case n {
    0 -> 0
    n if n > 0 -> 1
    _ -> -1
  }
}

fn with_double(callback: fn(Int) -> Int) -> Int {
  callback(21)
}

fn use_demo(seed: Int) -> Int {
  use n <- with_double
  n + seed
}
