# Xenapse

Xenapse is a small text-first diagram language and a Rust engine that compiles it.
You write a `.xnp` file (plus an optional `.xcss` stylesheet) and get one
self-contained, deterministic SVG: the same input gives the same bytes on every host.

```
IMPORT FLOW
START
< Is it ready? >
IF YES { [ Ship it ] } ELSE { [ Keep working ] }
END
```

## Build

Requires a stable Rust toolchain (edition 2021). There are no third-party dependencies.

```sh
cargo build --release          # binary: target/release/xenapse
cargo run -p xenapse-cli -- --help
```

## CLI

```sh
xenapse compile <input.xnp> [-o <out.svg>|-] [--style <file.xcss>]
xenapse check   <input.xnp> [--style <file.xcss>]
```

- `compile` writes `<input stem>.svg` next to the input by default; `-o -` writes to stdout.
- `check` runs the whole pipeline and writes nothing.
- Stylesheet precedence: `--style` beats the `STYLESHEET "path"` directive (resolved
  relative to the input file), which beats no stylesheet. A missing file is an error.
- Diagnostics go to stderr. Exit codes: `0` success (warnings allowed), `1` the diagram
  has errors (no SVG is written), `2` usage or I/O error.

Try the examples (generated SVGs are git-ignored):

```sh
xenapse check   examples/flow/support-ticket.xnp
xenapse compile examples/mindmap/release-plan.xnp -o release-plan.svg
```

## Library

The core crate never touches the filesystem: you pass text in and get text out.

```rust
use xenapse_core::{compile, CompileOptions};

fn main() {
    let source = "IMPORT FLOW\nSTART\n[ Hello ]\nEND\n";
    let result = compile(source, None, &CompileOptions::default());
    eprint!("{}", result.format_diagnostics());
    if let Some(svg) = result.svg {
        println!("{svg}");
    }
}
```

`result.svg` is `None` whenever any error diagnostic exists. `result.elements` maps every
node and edge back to its source span. Pass stylesheet text as the second argument, and
use `declared_stylesheet(source)` to find the `STYLESHEET` path a host should load.

## Scope

Two diagram modules only, `FLOW` and `MINDMAP`, and one output format, SVG. There is no
editor, WASM build, PNG or PDF export. The language rules, grammar, XCSS properties and
diagnostics are specified in [`docs/DECISIONS.md`](docs/DECISIONS.md), which takes
precedence over [`docs/blueprint.md`](docs/blueprint.md).

## File Types and OS Integration

Xenapse registers two operating system file types:
- `.xnp` — **Xenapse Diagram** (`application/vnd.xenapse.diagram`)
- `.xcss` — **Xenapse Stylesheet** (`text/vnd.xenapse.stylesheet`)

Optional registration scripts and icons are provided under `packaging/` for Linux and Windows.
Registration does not set the CLI as a graphical editor. Default graphical opening will be handled separately by Xenapse Studio.
See [`docs/file-types.md`](docs/file-types.md) for installation and uninstallation details.
