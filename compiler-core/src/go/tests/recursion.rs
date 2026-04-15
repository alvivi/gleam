use crate::assert_go;

// True terminating recursion requires `case`, which lands in M3. These tests
// exercise just the self/cross-reference wiring — the generated programs loop
// forever if run, which is fine for snapshot checking.

#[test]
fn self_recursive_reference() {
    assert_go!(
        r#"
pub fn loop_forever(n: Int) -> Int {
  loop_forever(n + 1)
}
"#,
    );
}

#[test]
fn mutual_recursion_reference() {
    assert_go!(
        r#"
pub fn ping(n: Int) -> Int {
  pong(n + 1)
}

pub fn pong(n: Int) -> Int {
  ping(n + 1)
}
"#,
    );
}
