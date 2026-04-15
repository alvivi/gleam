// A small Gleam program exercising the M2 Go-backend feature set:
// literals, arithmetic and comparison, `let` bindings with shadowing,
// block expressions, direct and recursive calls, and `assert`.
//
// FFI (`@external(go, ...)`) is not yet lowered to Go (that is M6), so
// this fixture deliberately avoids calling out to the Go standard
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
  shadowed - checked_div(shadowed, 3)
}

fn double(n: Int) -> Int {
  n + n
}

fn checked_div(a: Int, b: Int) -> Int {
  a / b
}
