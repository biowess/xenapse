# Xenapse engine: resolved specification decisions

Save as `docs/DECISIONS.md`. The blueprint (`docs/blueprint.md`) defines scope. This file resolves its open points (blueprint §18) and ambiguities. **If they conflict, this file wins.** Items marked ✔ are interpretations the blueprint doesn't state; they were reviewed and are confirmed (see the review notes at the end).

---

## D1. Blueprint §18 open decisions

1. **IMPORT**: exactly one `IMPORT FLOW|MINDMAP`, and it must be the first meaningful token. A duplicate is an error.
2. **`( ... )`** is kept: event/state, drawn as an ellipse.
3. **Database** is `DATABASE "..."`. `{ }` is reserved for blocks.
4. **IF** is fixed: `IF YES { } ELSE { }`, both blocks required, no custom branch labels. Branch edges carry the labels `Yes` / `No`.
5. **No loops or cycles** in 1.0.
6. **MINDMAP layouts**: all three are implemented. TREE and HORIZONTAL come first (one tidy-tree algorithm, transposed for HORIZONTAL), then RADIAL. Default is RADIAL.
7. **Text measurement**: a small `TextMeasurer` trait in core, plus a built-in deterministic approximate measurer. Hosts may supply their own. No font loading or Rust font stack in core.
8. Editor shell, WASM, PNG and PDF are out of scope.

## D2. Lexical rules (`.xnp`)

- UTF-8. Whitespace separates tokens and is never significant. Spans are byte offsets.
- Comment: `//` to end of line. Comments are emitted as tokens; the parser skips them.
- Keywords (uppercase, case-sensitive): `IMPORT FLOW MINDMAP STYLESHEET DIRECTION LAYOUT TB LR RADIAL TREE HORIZONTAL ROOT START END IF YES ELSE DATABASE`. `BT` / `RL` are not supported in 1.0.
- Identifier: `[A-Za-z_][A-Za-z0-9_-]*`, case-sensitive. ID token `#ident`; class token `.ident`.
- String: `"..."`, escapes `\\ \" \n \t` only. A raw newline inside a string or an unknown escape is an error.
- **Module detection**: the lexer determines the module from the first `IMPORT FLOW|MINDMAP` and lexes the rest in that mode. If it's absent, the lexer stays neutral and the parser reports the missing IMPORT.
- **FLOW shapes** (FLOW mode only) lex as ONE token containing kind and label:
  `[ ]` process · `( )` event · `< >` decision · `/ /` input/output · `| |` subprocess.
  - Label is either a single quoted string, or raw text up to the first closing delimiter on the same line. Raw text is trimmed and its whitespace runs collapse to one space. An empty label is an error.
  - A raw label cannot contain its closing delimiter. Use a quoted label (`[ "a ] b" ]`). `//` inside a label is literal text.
  - **Comment vs I/O**: at a token start, `//` is always a comment. `/` followed by anything else opens an I/O shape. Therefore an I/O label may not begin with `/`, and `//` can never be an empty I/O node.
  - Unterminated shape → error, resume at the next line.
- Shape opener characters in MINDMAP mode get a module-specific error ("FLOW shape syntax is not allowed in MINDMAP"), not a generic unknown-character error.

## D3. Grammar (`.xnp`)

```
document  := "IMPORT" module directive* body
directive := "STYLESHEET" string
           | "DIRECTION" ("TB"|"LR")                       -- FLOW only
           | "LAYOUT" ("RADIAL"|"TREE"|"HORIZONTAL")       -- MINDMAP only
suffix    := ["#"id] ("."class)*                           -- id first, then classes

FLOW body := stmt*
stmt      := node | if
node      := ("START" | "END" | shape | "DATABASE" string) suffix
if        := "IF" "YES" block "ELSE" block
block     := "{" stmt+ "}"

MINDMAP   := "ROOT" string suffix [mblock]                 -- exactly one, sole body construct
mnode     := string suffix [mblock]
mblock    := "{" mnode+ "}"
```

- Each directive at most once; directives precede the body. Directives valid for the other module are rejected **by the parser** with a module-specific error (this resolves blueprint §13's "unsupported layout directive" at syntax level).
- Defaults: `DIRECTION TB`, `LAYOUT RADIAL`.
- Empty blocks are errors. `ELSE` without `IF` is a syntax error. `IF NO` is an error.
- FLOW constructs in MINDMAP and `ROOT` in FLOW are module-isolation errors.
- Block nesting depth limit: 128 (diagnostic, never a stack overflow).
- Parser recovers at statement boundaries and collects multiple diagnostics. A partial AST may be returned for tooling, but **semantic validation and lowering run only when lexing and parsing produced no errors**.

## D4. FLOW semantics

- Node kinds: Start, End, Process, Event, Decision, InputOutput, Subprocess, Database. START/END default labels are "Start" / "End".
- Built-in node classes: `start end process event decision input output database subprocess`. ✔ A `/ /` node is one kind (InputOutput) and matches BOTH `.input` and `.output`; the source cannot distinguish them. Built-in edge classes: `yes`, `no`.
- **Reserved**: user classes using any built-in name (including `yes`/`no`) are errors, in both modules.
- Exactly one START, and it must be the first top-level statement. At least one END.
- IF must immediately follow a decision (`XN-FLOW-004`). Every decision must be immediately followed by an IF.
- Edges: consecutive statements connect in order. A decision connects to the first node of its YES branch (kind Yes) and ELSE branch (kind No). Branch tails that complete normally connect to the next statement after the IF (join). A branch ending in END does not join.
- **Unreachable** (error): any statement after an END in the same block, or after an IF whose branches both terminate in END.
- **Dead end** (warning): a path that ends without reaching END.
- Nested IFs are allowed.
- IDs unique per document (error at the second declaration, with a related location pointing at the first).
- **Identity**: `NodeId` / `EdgeId` are typed indices in source (pre-order) order. SVG ids are `xn-n{i}` / `xn-e{i}`; an explicit source id is exposed as `data-xn-id`.
- **Edge source span**: sequence and join edges use the target statement's span; Yes/No edges use the `IF YES` / `ELSE` keyword span.

## D5. MINDMAP semantics

- ROOT may have no block (a one-node map is valid). Child order is source order. IDs unique; classes reusable.
- Node ids are assigned in pre-order, so the root is index 0. Edges are implicit parent→child: edge `i` connects the parent to node `i+1`, with that child's span.
- Shapes: root is an ellipse, every other node a rounded rectangle. No built-in node classes in MINDMAP.

## D6. Diagnostics

- Code format `XN-<FAMILY>-NNN`. Families: `LEX`, `SYN`, `FLOW`, `MIND`, `SEM` (shared semantic), `CSS`. Codes are stable once assigned and defined in one `diagnostics/codes.rs`.
- Diagnostic fields: code, severity (Error|Warning), message, optional help, origin (Source|Stylesheet), span, optional related span+label.
- Rendered form:
  ```
  main.xnp:12:1: error[XN-FLOW-004]
  IF must immediately follow a decision node.
  help: ...
  note: first declared at main.xnp:3:5      (when related is present)
  ```
- Line and column are 1-based; column counts Unicode scalar values.
- Output order: stable sort by origin (Source first), then span start.
- **Any Error diagnostic means no SVG is produced.** Warnings do not block output.

## D7. XCSS

- ✔ Comments are `/* */`. No embedded-stylesheet syntax exists in the blueprint, so none is implemented: the core takes stylesheet **text** as a compile input. `STYLESHEET "path"` is parsed and exposed so the host can load the file. The core never touches the filesystem.
- Syntax: `selector { property: value; ... }`, last `;` optional. One selector per rule: no comma lists, no `@` rules, no pseudo-classes, no attribute selectors, no combinators other than the `text` suffix. Violations are syntax errors.
- Selector forms: `node`, `edge`, `.class`, `#id`, `node.class`, `edge.class`, `diagram`, and a trailing `text` on node-targeting selectors (`node text`, `.c text`, `#id text`).
  - ✔ `diagram` is the **only addition**: it is the home for the four layout properties, which otherwise have no selector.
  - A bare `.class` or `#id` targets nodes only. Edges are targeted by `edge` / `edge.yes` / `edge.no`.
- Specificity: type 1, class 10, id 100, summed for compounds. Higher wins; ties go to the later rule. Node-level `color` and font properties flow into the node's text; `text` rules override them.
- Values: colors `#RGB` `#RRGGBB` `#RRGGBBAA` (plus `none` for fill/stroke); lengths are non-negative numbers with optional `px` (no other units); font-family is a quoted string or ident list.

| Target | Property | Values | Class |
|---|---|---|---|
| node | fill, stroke | color \| none | paint |
| node | stroke-width | number ≥ 0 | paint |
| node | color | color | paint |
| node | font-family / font-size / font-weight | list / px > 0 / `normal`\|`bold`\|100..900 step 100 | **layout** |
| node | text-align | left\|center\|right | paint |
| node | padding, min-width, max-width | px (max/min > 0) | **layout** |
| node | border-radius | px ≥ 0 (Process, Subprocess, MINDMAP topics only; ignored elsewhere, no diagnostic) | paint |
| text | color, text-align / font-* | as above | paint / layout |
| edge | stroke | color \| none | paint |
| edge | stroke-width | number ≥ 0 | paint |
| edge | stroke-dasharray | numbers list \| none | paint |
| edge | arrowhead | triangle \| none | paint |
| edge | font-family, font-size / color | as above | layout / paint |
| diagram | node-gap, rank-gap, branch-gap, diagram-padding | px ≥ 0 | **layout** |

- Diagnostics: unknown selector → warning (rule skipped). Unsupported property, or a property not valid for its selector → warning (declaration skipped). **Invalid value → error** (declaration skipped). Selector matching nothing → warning.
- Built-in default theme (✔ values are adjustable):
  - Node: fill `#FFFFFF`, stroke `#344054`, stroke-width 1.5, color `#202938`, font-family `"IBM Plex Sans", "Helvetica Neue", Arial, sans-serif`, font-size 14, weight normal, text-align center, padding 12, min-width 72, max-width 220, border-radius 8.
  - Edge: stroke `#667085`, stroke-width 1.5, arrowhead triangle, font-size 12, color `#475467`.
  - Diagram: node-gap 32, rank-gap 48, branch-gap 40, diagram-padding 24.

## D8. Layout and text

- Core computes all geometry. The diagram model and the AST contain no coordinates.
- `TextMeasurer`: width of a string and line height for a `FontSpec { family list, size, weight }`. The built-in approximate measurer uses a per-character-class width table (narrow, wide, digit, uppercase, CJK = 1em, other) and is fully deterministic.
- Wrapping: greedy at whitespace up to the node's max text width. Words longer than the width break at character boundaries. Explicit `\n` is honored.
- FLOW: layered (longest-path rank from START; order by barycenter sweeps with stable tie-breaks, YES preferred left/top; simple median-relaxation x-placement, no Brandes–Köpf). LR is TB computed with swapped sizes and then transposed. Orthogonal routing. Yes/No labels sit beside the first segment.
  - `node-gap`: between real nodes in one layer.
  - `branch-gap`: clearance between a long-edge channel (virtual item) and its neighbours.
  - `rank-gap`: between layers.
- MINDMAP: TREE is a tidy tree with elbow connectors. HORIZONTAL is the same, transposed. RADIAL gives angular sectors proportional to leaf count starting at the top and going clockwise in source order. Ring radius is grown per depth until nodes don't overlap. Connectors are straight, clipped to node boxes. Labels stay horizontal inside nodes.
- Determinism: no random values and no hash-order iteration in anything that affects output. Fixed iteration counts. Stable sorts.

## D9. SVG

- Self-contained, no scripts, event handlers, external references or `<style>` block. Paint is written as inline presentation attributes.
- Structure: `<svg xmlns viewBox width height role="img">` → `<title>`, `<desc>`, edge group, node group. Each element has its stable id, classes (`xn-node xn-<builtin> <user classes>`), and `data-xn-*` source mapping. Each node has a `<title>` with its label.
- Arrowheads are filled `<path>` polygons using the edge stroke colour (no `<marker>`, for portability).
- Text is `<text>` with one `<tspan>` per wrapped line. All user text is XML-escaped. Numbers are rounded to 2 decimals with trailing zeros trimmed.

## D10. Public API and CLI

- `compile(source, stylesheet: Option<&str>, &CompileOptions) -> CompileResult`.
  - `CompileOptions { source_name, stylesheet_name, measurer }`.
  - `CompileResult { api_version, module, diagnostics, svg: Option<String>, elements, width, height }`, plus `format_diagnostics()`.
  - `declared_stylesheet(source) -> Option<String>` returns the `STYLESHEET` path.
  - `API_VERSION` constant.
- CLI (`std` only, no arg-parsing crate): `xenapse compile <in.xnp> [-o <out.svg>|-] [--style <f.xcss>]`, `xenapse check <in.xnp> [--style <f.xcss>]`, `--help`, `--version`.
  - Stylesheet precedence: `--style` > the `STYLESHEET` directive (relative to the input file's directory) > none. A missing requested or declared file is an error, never a silent fallback. If both are present, `--style` wins and a `note:` goes to stderr.
  - Exit codes: 0 OK (warnings allowed), 1 diagnostics contain errors, 2 usage or I/O error.
  - Diagnostics go to stderr.

---

## D11. Review of ✔ items

- **`/ /` matches `.input` and `.output`**: confirmed. The source cannot distinguish them, so matching both is the only choice that never surprises the author.
- **XCSS `/* */` comments, stylesheet text as a compile input**: confirmed. Keeps core free of filesystem access and the same input produces the same output on every host.
- **`diagram` selector**: confirmed as the only addition. Layout properties need a home, and a dedicated selector is clearer than overloading `node` or `edge`.
- **Default theme**: confirmed. Font stack widened to `"IBM Plex Sans", "Helvetica Neue", Arial, sans-serif` so output degrades predictably on hosts without IBM Plex Sans. All other values unchanged.
- **Source primitives (Prompt 01)**: no conflict. Spans are byte offsets; line and column are 1-based with columns in Unicode scalar values; a `\r` before `\n` is ordinary text.
- **Repository hygiene**: `Cargo.lock` is committed (the workspace ships a binary); `/target` is ignored.
