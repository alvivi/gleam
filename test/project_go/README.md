# test/project_go

A fixture that exercises the Gleam Go-backend M2 feature set end-to-end:
literals, arithmetic, `let` with shadowing, block expressions, direct and
recursive calls, and `assert`. FFI (`@external(go, ...)`) is deferred to
M6, so the program intentionally avoids the Go standard library.

## Compiling

```sh
cd test/project_go
cargo run --manifest-path ../../Cargo.toml -- build --target=go
```

This emits `build/dev/go/project_go/project_go.go` plus
`build/dev/go/project_go/prelude/prelude.go` and a per-package `go.mod`.

## Running manually

There is no `gleam run --target=go` yet (that is M8). To run the emitted
code, add a hand-written `package main` that imports the Gleam package
and calls `Run`:

```go
// build/dev/go/project_go/cmd/run/main.go
package main

import (
	"fmt"

	projectgo "gleam/project_go"
)

func main() {
	fmt.Println(projectgo.Run())
}
```

Then:

```sh
cd build/dev/go/project_go
go run ./cmd/run
```

Expected output: `22`.
