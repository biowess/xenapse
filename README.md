<div align=center>
<img src="assets/banner.png" alt="Xenapse Banner" width="600" />

# Xenapse

[![Rust](https://img.shields.io/badge/rust-stable-4B7571?style=flat&logo=rust&logoColor=EEF3F2&labelColor=172638)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-4B7571?style=flat&logoColor=EEF3F2&labelColor=172638)](LICENSE)
[![Release](https://img.shields.io/github/v/release/biowess/xenapse?style=flat&color=4B7571&logoColor=EEF3F2&labelColor=172638)](https://github.com/biowess/xenapse/releases)
[![No deps](https://img.shields.io/badge/dependencies-none-4B7571?style=flat&labelColor=172638)](Cargo.toml)

</div>

Xenapse is a small text-first diagram language and a Rust engine that compiles it.
You write a `.xnp` file (plus an optional `.xcss` stylesheet) and get one
self-contained, deterministic SVG: the same input gives the same bytes on every host.

## The Language

A Xenapse diagram starts with an `IMPORT` declaring its module, then a body of
layout instructions. Two modules are built in:

| Module | Use |
|---|---|
| `FLOW` | Flowcharts — decisions, branches, loops |
| `MINDMAP` | Radial concept maps |

**Flow example** — decisions use `< … >`, steps use `[ … ]`:

```
IMPORT FLOW
START
< Is it ready? >
IF YES { [ Ship it ] } ELSE { [ Keep working ] }
END
```

**Stylesheet** — visual properties live in a separate `.xcss` file, keeping the
diagram source readable:

```
STYLESHEET "style.xcss"
IMPORT FLOW
START
[ Deploy ]
END
```

The full grammar, all XCSS properties, node shapes, and diagnostic codes are in
[`docs/DECISIONS.md`](docs/DECISIONS.md) (authoritative).
[`docs/blueprint.md`](docs/blueprint.md) has the design rationale.

## Install

Pre-built binaries for Linux and Windows are in [`releases/`](releases/).
Verify the `.sha256sum` before use.

```sh
# Linux x86-64
unzip releases/xenapse-linux-x86_64.zip
./install.sh
```

Or build from source (no third-party dependencies, stable Rust required):

```sh
cargo build --release          # → target/release/xenapse
```

## CLI

```sh
xenapse compile <input.xnp> [-o <out.svg>|-] [--style <file.xcss>]
xenapse check   <input.xnp> [--style <file.xcss>]
```

- `compile` writes `<input stem>.svg` next to the input by default; `-o -` writes to stdout.
- `check` runs the full pipeline and writes nothing — useful in CI.
- Stylesheet precedence: `--style` > `STYLESHEET "path"` directive > none. A missing file is an error.
- Diagnostics go to stderr. Exit codes: `0` success, `1` diagram errors, `2` usage/I/O error.

```sh
xenapse check   examples/flow/support-ticket.xnp
xenapse compile examples/mindmap/release-plan.xnp -o release-plan.svg
```

## Library

The `xenapse-core` crate is filesystem-free: pass text in, get text out.

```rust
use xenapse_core::{compile, CompileOptions};

let source = "IMPORT FLOW\nSTART\n[ Hello ]\nEND\n";
let result = compile(source, None, &CompileOptions::default());
eprint!("{}", result.format_diagnostics());
if let Some(svg) = result.svg {
    println!("{svg}");
}
```

`result.svg` is `None` when any error diagnostic exists. `result.elements` maps every
node and edge back to its source span. Pass stylesheet text as the second argument;
use `declared_stylesheet(source)` to extract the `STYLESHEET` path for the host to load.

## File Types

Xenapse registers two OS MIME types:

| Extension | Type | MIME |
|---|---|---|
| `.xnp` | Xenapse Diagram | `application/vnd.xenapse.diagram` |
| `.xcss` | Xenapse Stylesheet | `text/vnd.xenapse.stylesheet` |

Registration scripts and icons are under `packaging/`. See [`docs/file-types.md`](docs/file-types.md).

## Scope

FLOW and MINDMAP modules, SVG output only. No editor, WASM target, PNG/PDF export.
Xenapse Studio (a graphical editor) is planned separately.
