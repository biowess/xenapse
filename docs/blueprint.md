# Xenapse
## Product and Technical Blueprint

**Version:** 0.1 working blueprint
**Date:** 4 November 2026
**Status:** Architecture and product scope proposed for implementation planning
**Working identity:** Xenapse · `.xnp` source files · `.xcss` stylesheets · Xenapse Studio

---

## 1. Executive summary

Xenapse is a small, declarative diagramming language and editor. A user describes the structure and meaning of information in a readable text file. Xenapse parses that description, validates it, calculates a layout, and renders a polished SVG diagram.

The first release has two diagram modules:

- **FLOW**: structured processes, decisions, pathways and directed workflows.
- **MINDMAP**: rooted conceptual hierarchies and topic maps.

Both modules share one language engine, one source format, one styling system, one SVG renderer and one editor. They do **not** share a universal layout algorithm: a process graph and a mind map have different structural rules and should be laid out differently.

The engine is written in Rust. The editor is a separate frontend that communicates with the engine through a narrow API. The same Rust core should be reusable from a command-line compiler, a desktop application and, later, a browser/WebAssembly build.

**Product principle:** users describe meaning and relationships, not coordinates. Xenapse owns the geometry.

---

## 2. Product vision

### 2.1 What Xenapse is

Xenapse is a text-first diagramming system with a deliberately small language. It should feel closer to writing a structured document than programming a general-purpose computer.

A user should be able to:

1. Write a `.xnp` document using a small set of readable constructs.
2. See the diagram update in a live preview.
3. Receive clear diagnostics when syntax or structure is invalid.
4. Style the diagram through a companion `.xcss` stylesheet.
5. Select a diagram element and locate its source declaration.
6. Export a clean, scalable SVG.

### 2.2 What Xenapse is not

The first release is not intended to be:

- A general-purpose programming language.
- A freeform drawing or vector illustration program.
- A replacement for every feature in Mermaid, Graphviz, PlantUML, D2 or professional diagramming suites.
- A universal graph editor with arbitrary manual node positioning.
- A general CSS implementation or browser engine.
- A plugin marketplace or a large collection of diagram modules.

Avoid feature accumulation. A feature belongs in the language only when it contributes to describing the diagram's structure or meaning. Visual appearance belongs in `.xcss` or editor controls.

### 2.3 Design principles

- **Readable:** source should be understandable without opening the preview.
- **Whitespace-insensitive:** indentation and line wrapping improve readability but do not define meaning.
- **Predictable:** the same source and stylesheet produce the same diagram.
- **Strict by module:** FLOW constructs are not silently accepted in MINDMAP, or vice versa.
- **Accessible diagnostics:** explain what is wrong, where it is wrong and how to fix it.
- **Automatic layout:** no coordinates or mandatory manual positioning in version 1.0.
- **Portable core:** no dependency on a particular editor, operating system or rendering surface.
- **Small surface area:** a few reliable features are better than many fragile ones.

---

## 3. Scope of Xenapse 1.0

### Included

| Area | Version 1.0 scope |
|---|---|
| Modules | FLOW and MINDMAP |
| Source format | `.xnp` |
| Stylesheet format | `.xcss` |
| Engine | Rust library |
| Parsing | Lexer, parser, AST, semantic validation |
| Layout | Layered directed layout for FLOW; tree layout for MINDMAP |
| Output | SVG |
| Editor | Source editor, live preview, diagnostics, style editing |
| Selection | Preview element to source declaration mapping |
| Styling | Small CSS-inspired subset |
| Direction | Top-to-bottom and left-to-right flow; radial and hierarchical mind map layouts if implementation quality permits |
| Operation | Local-first; no account or cloud service required |

### Explicitly deferred

- NETWORK, TIMELINE, TREE or other new modules.
- Arbitrary user-defined functions, macros or a component system.
- User scripts or executable expressions inside diagrams.
- Unrestricted CSS, JavaScript, HTML or embedded SVG in source files.
- Manual coordinate editing and drag-to-position authoring.
- Animated diagrams and interactive runtime behaviour.
- Cross-links between mind map branches.
- General-purpose cyclic flow graphs and arbitrary edge routing.
- Collaboration, cloud projects, accounts and hosted publishing.
- PDF export until the SVG and PNG pipelines are reliable.

The deferred list is intentional, not a promise that these features will appear later.

---

## 4. The two modules

## 4.1 FLOW

FLOW describes a directed process. Its internal representation is a directed graph, but its initial syntax is structured and sequential so users do not have to manually connect every ordinary step.

### Proposed node forms

| Source form | Meaning | Suggested appearance |
|---|---|---|
| `START` | Entry point | Terminator / rounded capsule |
| `END` | Exit point | Terminator / rounded capsule |
| `[ ... ]` | Process or action | Rectangle |
| `( ... )` | Event or state | Ellipse |
| `< ... >` | Decision | Diamond |
| `/ ... /` | Input or output | Parallelogram |
| `\| ... \|` | Predefined process / sub-process | Double-sided rectangle |
| `DATABASE "..."` | Data store | Cylinder |

**Important grammar decision:** reserve `{ ... }` for structural blocks such as `IF` branches. Do not also use bare curly braces for database nodes. A form such as `DATABASE "Patient records"` is slightly more explicit, but it removes a genuine ambiguity between a data-store shape and a block delimiter.

A node may have an optional ID and one or more classes:

```text
[ Clinical assessment ] #assessment .clinical .important
```

IDs are unique within the document. Classes are reusable styling hooks. If an ID is omitted, the compiler generates a stable internal identifier for that compilation; source references and styles that need stable targeting should use an explicit ID.

### Proposed FLOW example

```text
IMPORT FLOW
STYLESHEET "styles.xcss"
DIRECTION TB

START #start

/ Receive patient / #intake
[ Clinical assessment ] #assessment
< Hemodynamically stable? > #stability

IF YES {
    [ Routine investigations ] #routine
    [ Review results ] #review
}
ELSE {
    | Emergency stabilization | #stabilize
}

[ Document outcome ] #documentation
END #finish
```

### FLOW sequencing semantics

These rules should be explicit in the language specification:

1. Ordinary statements at the same block level are connected in source order.
2. An `IF` block must immediately follow a decision node. It does not create another visible node; it describes the outgoing branches of that decision.
3. `IF YES` creates the affirmative branch. `ELSE` creates the alternative branch. Both branches are required in version 1.0 to avoid an unspecified false path.
4. Statements inside each branch are connected sequentially.
5. When a branch completes normally, its final node connects to the first statement after the `IF` block. This is the automatic join.
6. A branch that reaches `END` terminates there and does not connect to the join.
7. Nested `IF` blocks are allowed, provided each is attached to the immediately preceding decision node.
8. `ELSE` without a matching `IF`, an `IF` without a preceding decision, or a decision followed by malformed branch syntax is an error.

This gives the user readable control flow without requiring explicit connectors for ordinary cases.

### FLOW limits for version 1.0

- One `START` node is required.
- At least one `END` node is required. Multiple exits are allowed for different terminal outcomes.
- The structured syntax produces an acyclic graph. Cycles, back-edges and arbitrary cross-links are not part of the first release.
- A decision must have exactly two branches in version 1.0: affirmative and alternative.
- The compiler should detect disconnected or unreachable nodes and report them as errors or warnings according to severity.

If real use cases demonstrate a need for loops or arbitrary connections, design an explicit construct for them later. Do not quietly overload the meaning of `->` or make source order behave differently in special cases.

### Flow direction

Initial directions:

- `DIRECTION TB`: top to bottom.
- `DIRECTION LR`: left to right.

`BT` and `RL` can be added if they are inexpensive after the first two directions work well. Direction changes should transform the final layout, not require a separate grammar.

---

## 4.2 MINDMAP

MINDMAP represents one rooted hierarchy. It is a tree, not a general network. Child order follows source order.

### Proposed syntax

```text
IMPORT MINDMAP
STYLESHEET "styles.xcss"
LAYOUT RADIAL

ROOT "Hematology" #hematology {
    "Blood" #blood {
        "Red cells" #red-cells {
            "Anemia" #anemia
            "Polycythemia" #polycythemia
        }
        "White cells" #white-cells {
            "Leukemia" #leukemia
            "Lymphoma" #lymphoma
        }
        "Platelets" #platelets {
            "Thrombocytopenia" #thrombocytopenia
        }
    }
}
```

### MINDMAP rules

1. Exactly one `ROOT` declaration is required.
2. A node with a `{ ... }` block has children. A node without a block is a leaf.
3. Braces define hierarchy. Indentation is cosmetic only.
4. Every child belongs to the nearest enclosing node block.
5. Empty child blocks are invalid.
6. IDs must be unique. Classes may be reused.
7. Cross-links between branches are excluded from version 1.0. A cross-linked structure is a network, not a pure mind map, and should not be smuggled into the tree grammar.

### Layout modes

- `LAYOUT RADIAL`: root near the centre, branches distributed around it.
- `LAYOUT TREE`: conventional top-down hierarchy.
- `LAYOUT HORIZONTAL`: conventional left-to-right hierarchy.

RADIAL is the natural default for a classic mind map. TREE and HORIZONTAL are useful for long or deep hierarchies. The implementation should not expose all three until each produces legible diagrams with uneven subtree sizes and long labels.

---

## 5. Shared source-language rules

### 5.1 Module declaration

The first meaningful declaration is `IMPORT FLOW` or `IMPORT MINDMAP`. A document has one active module. The compiler must decide the module before parsing the body, then apply only that module's grammar.

**Recommended final rule:** require exactly one `IMPORT` declaration and report duplicate declarations. During early concept exploration, a “last import wins” rule was discussed. It is possible to implement, but it makes earlier declarations silently irrelevant and makes accidental module switching difficult to spot. A strict single declaration is easier to learn and diagnose. This remains a product decision to confirm before freezing the grammar.

### 5.2 Whitespace and comments

- Spaces, tabs and line breaks separate tokens but do not define hierarchy.
- Users may freely format or reformat the source without changing its meaning.
- Use `//` for single-line comments.
- Preserve source spans for every token and syntax node, including comments where useful for editor tooling.
- The lexer must distinguish a comment from an input/output node beginning with `/`.

### 5.3 Labels and strings

- FLOW shape delimiters may contain readable unquoted labels for concise writing.
- Quoted strings should be supported when a label contains delimiter characters or needs explicit escaping.
- MINDMAP labels should use quoted strings consistently.
- Define standard escapes at minimum: `\\`, `\"`, `\n` and `\t`.
- Define whether literal newlines are allowed in labels. Prefer escaped `\n` in version 1.0.
- IDs use a restricted identifier alphabet, for example letters, digits, `_` and `-`, with a letter or underscore as the first character.
- IDs are case-sensitive. Keywords are uppercase and case-sensitive in the first release to keep the grammar unambiguous.

### 5.4 Source locations

Every token, AST node and diagnostic should retain a source span. Diagnostics should expose:

- File name.
- Line and column.
- Offending token or source range.
- A direct explanation.
- A suggested correction when the compiler can provide one safely.

Example:

```text
main.xnp:12:1: error[XN-FLOW-004]
IF must immediately follow a decision node.
```

### 5.5 Module isolation

A FLOW keyword or statement in a MINDMAP document must not be accepted as ordinary text. For example, `START` under `IMPORT MINDMAP` should produce a module-specific diagnostic. The same applies to `ROOT` in FLOW mode.

---

## 6. `.xcss` styling system

`.xcss` is a deliberately restricted CSS-inspired stylesheet. It should look familiar to web developers without becoming a second full programming language.

### Example

```css
node {
    fill: #FFFFFF;
    stroke: #344054;
    stroke-width: 1.5;
    color: #202938;
    font-family: "IBM Plex Sans";
    font-size: 14px;
}

.process {
    fill: #F4F6F8;
}

.decision {
    fill: #F7E9E7;
    stroke: #702D3B;
}

#assessment {
    stroke-width: 2px;
}

edge {
    stroke: #667085;
    stroke-width: 1.5;
}

edge.yes {
    stroke: #287A55;
}
```

### Selector model

Start with a small selector set:

- `node`: all nodes.
- `edge`: all edges.
- `.class`: user-defined class.
- `#id`: explicit source ID.
- `.process`, `.decision`, `.event`, `.input`, `.output`, `.database`, `.subprocess`, `.start`, `.end`: built-in node-type classes.
- `edge.yes` and `edge.no`: built-in decision-branch edge classes.
- `node text` or `#id text`: text styling within a node.

Do not add pseudo-classes, attribute selectors, combinator chains, variables, imports or `@` rules to version 1.0. The editor does not need a full browser CSS cascade.

### Initial property whitelist

**Node appearance**
- `fill`
- `stroke`
- `stroke-width`
- `color`
- `font-family`
- `font-size`
- `font-weight`
- `text-align`
- `padding`
- `min-width`
- `max-width`
- `border-radius` (where applicable)

**Edge appearance**
- `stroke`
- `stroke-width`
- `stroke-dasharray`
- `arrowhead`
- `font-family`
- `font-size`
- `color`

**Diagram layout**
- `node-gap`
- `rank-gap`
- `branch-gap`
- `diagram-padding`

Layout properties must be resolved before geometry is calculated. Unknown selectors or unsupported properties should produce warnings, not silently disappear.

### Styling architecture

- Parse `.xcss` into a small stylesheet model.
- Resolve selectors and properties against the diagram model.
- Keep layout-affecting styles separate from paint-only styles.
- Apply the same resolved style model to SVG output and the visual inspector.
- The visual inspector edits the actual `.xcss` source. It must not maintain a separate hidden styling state that can drift out of sync.

The `.xcss` implementation should not evaluate arbitrary CSS, load remote resources, or execute code.

---

## 7. Technical architecture

### 7.1 High-level pipeline

```text
.xnp source + optional .xcss
              |
              v
          Lexer
              |
              v
          Parser
              |
              v
         Module AST
              |
              v
    Semantic validation
              |
              v
      Diagram model
              |
              +--------------------+
              |                    |
              v                    v
       FLOW graph model      MINDMAP tree model
              |                    |
              v                    v
       Layered layout        Tree/radial layout
              |                    |
              +---------+----------+
                        |
                        v
                 SVG renderer
                        |
                        v
                SVG + metadata
                        |
                        v
                 Xenapse Studio
```

### 7.2 Rust crate structure

Use a Cargo workspace with the core library separated from executable shells.

```text
xenapse/
├── Cargo.toml
├── crates/
│   ├── xenapse-core/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lexer/
│   │       ├── parser/
│   │       ├── ast/
│   │       ├── diagnostics/
│   │       ├── validate/
│   │       ├── model/
│   │       ├── style/
│   │       ├── layout/
│   │       │   ├── flow/
│   │       │   └── mindmap/
│   │       └── render/
│   ├── xenapse-cli/
│   └── xenapse-wasm/       # later, if browser execution is desired
├── studio/
│   ├── index.html
│   ├── styles/
│   ├── scripts/
│   └── assets/
├── examples/
│   ├── flow/
│   └── mindmap/
└── tests/
    ├── lexer/
    ├── parser/
    ├── validation/
    ├── layout/
    └── integration/
```

The exact folder layout can change. The non-negotiable boundary is that `xenapse-core` must not depend on the Studio frontend.

### 7.3 Core data structures

Keep at least three representations separate:

1. **AST:** faithful representation of what the user wrote, including source spans and syntax-level structure.
2. **Diagram model:** normalized meaning of the document, independent of syntax and visual coordinates.
3. **Layout model:** measured node dimensions, coordinates, edge routes and label positions.

Do not store screen coordinates in the AST. Do not let the SVG renderer interpret raw source tokens. Do not make the parser responsible for deciding where boxes appear.

A simplified model might include:

```rust
struct Diagram {
    kind: DiagramKind,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    settings: DiagramSettings,
}

struct Node {
    id: NodeId,
    source_id: Option<String>,
    label: String,
    kind: NodeKind,
    classes: Vec<String>,
    source_span: SourceSpan,
}

struct Edge {
    from: NodeId,
    to: NodeId,
    label: Option<String>,
    kind: EdgeKind,
    source_span: Option<SourceSpan>,
}
```

This is illustrative, not a final API. IDs used internally should be typed indices or opaque identifiers, not strings used as the sole identity mechanism.

### 7.4 Compiler API

Expose one small public interface, conceptually:

```text
compile(source, stylesheet, options) -> CompileResult
```

The result should contain:

- SVG output when rendering succeeds.
- Structured diagnostics.
- Active module.
- Node and edge metadata.
- Source spans for preview-to-editor navigation.
- Optional layout metadata for the inspector.

Do not expose parser internals to the frontend. Keep the API versioned so a future WASM wrapper or CLI can use it without changing language semantics.

### 7.5 Parsing strategy

For the first implementation, a handwritten lexer and recursive-descent parser are reasonable. The grammar is small and has explicit block delimiters. A handwritten implementation is easy to inspect and gives precise control over source spans and error messages.

Use a parser library only when it solves a real problem. Chumsky is a serious option if robust error recovery and collecting multiple parse errors become more important than keeping the parser handwritten. Logos is a lexer-generation option if token definitions become cumbersome. Do not add both by default just because they are popular crates.

### 7.6 Graph representation

FLOW lowers into a directed graph. MINDMAP lowers into a rooted tree. A general graph crate such as `petgraph` can be useful for graph storage and standard algorithms, but it is not mandatory for a small first implementation. Choose one representation and keep it behind the core model API.

---

## 8. Automatic layout strategy

This is the most important engineering challenge. Parsing the syntax is comparatively straightforward. Generating consistently attractive diagrams from uneven, real-world content is what will make Xenapse feel finished.

### 8.1 FLOW layout

Use a layered directed-graph approach inspired by the Sugiyama family of algorithms:

1. Establish directed layers / ranks.
2. Order nodes within each layer to reduce edge crossings.
3. Assign coordinates while respecting node dimensions and spacing.
4. Route edges and place labels.
5. Apply the requested orientation.

For version 1.0, structured FLOW syntax avoids cycles, simplifying the first implementation. A full graph layout system such as Graphviz `dot` and the Eclipse Layout Kernel demonstrates the value of layered placement, crossing reduction and configurable edge routing. Xenapse can implement a smaller Rust-native pipeline tailored to its restricted grammar.

Important cases to test:

- A simple sequence.
- A decision with two short branches.
- Branches with different numbers of steps.
- Nested decisions.
- Branches that rejoin.
- Multiple END nodes.
- Very long labels.
- Narrow and wide diagrams.
- Disconnected or unreachable nodes.

Orthogonal edges are a strong default for flowcharts. Avoid gratuitous curves. The routing engine should minimize crossings, avoid node interiors and keep arrowheads clear of node borders.

### 8.2 MINDMAP layout

Use a tidy-tree algorithm for conventional hierarchical layouts. Reingold-Tilford and later refinements are established approaches for producing compact, visually balanced tree drawings.

For radial mind maps:

1. Calculate each subtree's required angular allocation.
2. Assign angular sectors to children in source order.
3. Assign radius by depth, accounting for node dimensions.
4. Position and orient labels so they remain readable.
5. Route branch connectors without overlapping labels.

Do not force the FLOW layout algorithm to handle MINDMAP. Their structures and visual goals are different.

### 8.3 Text measurement

Node dimensions depend on actual text, font family, font size, wrapping and padding. This must be addressed early rather than patched after the layout algorithm is complete.

Recommended design: define a text-measurement interface used by the layout engine. In Studio, the browser can measure text using the actual loaded font and provide the measurements to the core. A future native CLI may use a Rust font stack or a documented fallback strategy. Do not assume character count is an adequate width estimate, especially with proportional fonts, non-Latin scripts and mixed text.

### 8.4 Determinism

The same source, stylesheet, fonts and options should produce the same geometry and SVG. Avoid random placement. Stable ordering and deterministic tie-breaking are mandatory for reliable previews, exports and tests.

---

## 9. SVG rendering and export

SVG is the canonical output format because it is vector-based, scalable and supported directly by browsers and many design tools.

The renderer should:

- Emit valid SVG with an appropriate `viewBox`.
- Escape all user-provided text.
- Use semantic groups and stable IDs/classes for nodes and edges.
- Keep text as SVG text rather than converting all labels to paths by default.
- Include accessible labels or `<title>` elements where practical.
- Avoid scripts, event handlers and external network resources.
- Make geometry and styling reproducible.
- Provide metadata connecting SVG elements to source spans.

### Export stages

1. **1.0:** SVG export is mandatory.
2. **1.0 or 1.1:** PNG export can be provided through browser rendering or a Rust rasterization route. `resvg` is a Rust SVG rendering option worth evaluating.
3. **Later:** PDF export should be added only after font handling and raster/vector fidelity have been tested. Do not claim native PDF support just because SVG works.

Font embedding and font licensing need their own review. SVG text may render differently on another machine if the required font is unavailable. The editor should warn when an export relies on non-portable fonts or provide a documented font-substitution policy.

---

## 10. Xenapse Studio

The editor should make the language approachable without turning into a freeform canvas application.

### 10.1 Initial workspace

A practical first layout:

```text
┌──────────────────────────────────────────────────────────────┐
│ File   Edit   View   Diagram   Style   Export                 │
├───────────────────────────┬──────────────────────────────────┤
│                           │                                  │
│       SOURCE EDITOR       │          LIVE PREVIEW             │
│                           │                                  │
│                           │                                  │
├───────────────────────────┴──────────────────────────────────┤
│ Diagnostics / Warnings / Errors                              │
└──────────────────────────────────────────────────────────────┘
```

A stylesheet panel or inspector can be opened when needed rather than permanently consuming space.

### 10.2 Essential editor behaviours

- Open, save and create `.xnp` files.
- Open and save an associated `.xcss` file.
- Live preview with a short debounce after edits.
- Syntax highlighting for the active module.
- Line numbers and useful error markers.
- Diagnostics with click-to-source navigation.
- Click a node in the preview to select its source declaration.
- Click a source declaration to highlight the corresponding preview node.
- Zoom, fit-to-view and pan in the preview.
- Export SVG.

### 10.3 Visual style inspector

The inspector edits the stylesheet, not the rendered SVG directly. Selecting a node should expose relevant properties such as fill, stroke, typography and padding. Editing a value updates the `.xcss` document and recompiles the diagram.

This preserves one source of truth: the stylesheet. There should not be a hidden GUI-only style layer that is lost when the file is reopened.

### 10.4 Editing philosophy

Version 1.0 should not permit arbitrary dragging of nodes to new coordinates. That would undermine the central promise of automatic layout and create a second layout-state format. Selection, zooming and inspection are in scope. Manual layout overrides can be reconsidered only after there is a clear persistence model.

### 10.5 Frontend technology

A vanilla HTML/CSS/JavaScript frontend is sufficient. Keep the UI independent of the engine. A plain text editor can be used for the first prototype; a dedicated editor component can be introduced later if syntax highlighting, selection mapping and large-document performance justify it.

---

## 11. Runtime and packaging strategy

Do not tie the language engine to its first host.

### Recommended order

1. **Rust library:** compile source to SVG from tests.
2. **CLI:** compile an `.xnp` file to `.svg` from the terminal.
3. **Studio prototype:** connect the frontend to the native engine or a WASM build.
4. **Desktop packaging:** consider Tauri once the editor's file/project workflow is stable.
5. **Browser playground:** consider WebAssembly if there is a concrete need for a zero-backend browser version.

Tauri offers typed commands between the frontend and Rust, making it a reasonable desktop shell. Rust can also compile to WebAssembly for browser use. Neither Tauri nor WASM should be allowed to dictate the language grammar or internal model.

### Local-first project layout

```text
Hematology/
├── main.xnp
├── styles.xcss
└── assets/
    └── ...
```

For version 1.0, external assets should be limited or excluded. If assets are later supported, use project-relative paths, clear missing-file diagnostics and an explicit policy for embedded versus linked assets.

---

## 12. Suggested dependency policy

Keep the initial dependency set small. Evaluate crates against the architecture rather than adding them in advance.

| Need | Candidate | Recommendation |
|---|---|---|
| Lexer | Handwritten Rust or `logos` | Start handwritten; use Logos if token rules become unwieldy |
| Parser | Handwritten recursive descent or `chumsky` | Start handwritten; consider Chumsky for multi-error recovery |
| Graph storage | `petgraph` | Useful, but optional |
| Layered layout | Custom Rust implementation; prototype existing algorithms | Own the final algorithm and test it against Xenapse's constraints |
| SVG rasterization | `resvg` | Evaluate for PNG export |
| Desktop shell | Tauri | Later, after the core and Studio workflow work |
| Browser compilation | `wasm-bindgen` / `wasm-pack` | Later, if browser-only execution is needed |

Do not embed Graphviz or a Java-based layout runtime as a mandatory dependency in the first release. They are useful references and possible optional backends, but an external runtime would complicate distribution and weaken the goal of a compact Rust core.

---

## 13. Diagnostics and validation

Validation should happen in distinct stages so the user receives accurate errors.

### Lexical

- Unknown characters or malformed string escapes.
- Unterminated strings or shape delimiters.
- Invalid identifiers.

### Syntactic

- Missing braces.
- Unexpected `ELSE`.
- Missing node labels.
- Invalid ordering of directives.
- Constructs not allowed by the active module.

### Semantic

- Duplicate IDs.
- Missing or duplicate ROOT.
- Missing START or END.
- IF not attached to a decision.
- Decision without valid branches.
- Invalid or unsupported references.
- Unreachable or disconnected flow nodes.
- Unsupported layout directives for the active module.

### Stylesheet

- Unknown selector.
- Unsupported property.
- Invalid property value.
- Selector that matches no element (warning).

Diagnostics must be structured data, not only preformatted strings. The Studio should decide how to display them.

The parser should recover at safe statement boundaries when feasible, so one typo does not erase all diagnostics below it. Recovery must never invent a valid diagram from structurally ambiguous input.

---

## 14. Testing and quality gates

A diagramming tool needs visual regression testing as well as ordinary compiler tests.

### Engine tests

- Lexer token and source-span tests.
- Parser tests for valid and invalid examples.
- Module-isolation tests.
- Semantic validation tests.
- ID and class resolution tests.
- Stylesheet selector/property tests.
- Layout invariants: no node overlap, valid edge endpoints, stable ordering and expected direction.
- SVG validity and escaping tests.
- Deterministic-output tests.

### Visual fixture library

Maintain a small, permanent set of test diagrams:

- Short and long linear flows.
- Two-way and nested decisions.
- Branches of unequal depth.
- Multiple terminal outcomes.
- Long labels and narrow nodes.
- Small and large mind maps.
- Deep trees and wide trees.
- Long labels around a radial mind map.
- Different fonts and non-Latin text.

For every fixture, preserve the source, stylesheet, generated SVG and a rendered reference image. Visual comparison should reveal geometry regressions that unit tests cannot catch.

### Acceptance criteria for 1.0

- The documented grammar parses consistently.
- Invalid module syntax is never silently accepted.
- Errors point to the relevant source span.
- Ordinary source edits do not require manual coordinates.
- Layout is deterministic.
- Nodes do not overlap in the supported test fixtures.
- Edges connect to the correct nodes and do not pass through node interiors in ordinary cases.
- Text remains readable at normal zoom and in exported SVG.
- Reopening a project preserves its source and styling.

---

## 15. Development roadmap

Build the engine vertically, proving one complete path before adding breadth.

### Phase 0 — Freeze the language decisions

- Confirm the exact FLOW shapes and semantics.
- Confirm the MINDMAP block syntax.
- Decide duplicate IMPORT behaviour.
- Decide comment syntax and string escaping.
- Define the initial `.xcss` selector/property subset.
- Write a short grammar reference before implementing the editor.

**Exit condition:** at least ten valid examples and ten invalid examples have unambiguous expected outcomes.

### Phase 1 — Lexer and parser

- Implement source spans.
- Tokenize keywords, identifiers, strings, delimiters, comments and shape markers.
- Parse FLOW and MINDMAP separately.
- Produce ASTs without rendering.

**Exit condition:** parser tests pass for the language examples, including malformed inputs.

### Phase 2 — Semantic model

- Resolve IDs and classes.
- Validate module-specific rules.
- Lower FLOW into a graph and MINDMAP into a tree.
- Produce structured diagnostics.

**Exit condition:** valid documents yield normalized diagram models; invalid documents yield actionable diagnostics.

### Phase 3 — First SVG proof

- Render simple nodes and labels.
- Add basic straight connectors.
- Add an SVG test/export command.

**Exit condition:** a CLI can turn one simple FLOW file and one simple MINDMAP file into valid SVG.

### Phase 4 — Layout quality

- Implement layered FLOW layout.
- Implement tidy MINDMAP layout.
- Add branch joins and routing.
- Add label measurement and wrapping.

**Exit condition:** the visual fixture library is legible without manual correction.

### Phase 5 — XCSS

- Parse the restricted stylesheet.
- Resolve selectors and declarations.
- Integrate layout-affecting styles before layout.
- Add paint-only styles in SVG rendering.

**Exit condition:** changing `.xcss` changes both geometry where appropriate and visual appearance predictably.

### Phase 6 — Xenapse Studio

- Build the source editor and preview.
- Add diagnostics and source mapping.
- Add preview-to-source selection.
- Add stylesheet editing and the visual inspector.

**Exit condition:** a user can create, edit, style, save and export both module types without using the CLI.

### Phase 7 — Packaging and polish

- Choose native/Tauri or browser/WASM delivery based on the desired distribution.
- Test font portability and SVG export.
- Add PNG export if reliable.
- Write user documentation and sample projects.

**Exit condition:** repeatable builds and a stable 1.0 project workflow.

---

## 16. Risks and mitigations

| Risk | Why it matters | Mitigation |
|---|---|---|
| FLOW grammar becomes ambiguous | A tiny syntax can become difficult to parse if delimiters serve multiple roles | Reserve braces for blocks; use explicit DATABASE keyword; publish grammar before coding |
| Automatic layout is poor | Visual quality is the product's main output | Build fixture diagrams early; test uneven branches and long labels continuously |
| Text measurement differs by platform | Nodes may overlap or shift | Define a measurement adapter and font fallback policy |
| Feature creep | More modules and syntax delay a useful release | Keep FLOW and MINDMAP as the only 1.0 modules |
| Editor becomes a drawing application | Manual coordinates conflict with text-first design | Keep geometry under the layout engine in 1.0 |
| CSS subset grows uncontrollably | Styling becomes another large language | Maintain a documented whitelist and reject unsupported constructs |
| Engine tied to one shell | Makes web, CLI and desktop builds harder | Keep the core UI-agnostic and expose a small compile API |
| Name conflicts | A public product name may collide with earlier rights | Perform formal trademark and domain checks before public launch |

---

## 17. Naming and project identity

Current working identity:

- **Language:** Xenapse
- **Source extension:** `.xnp`
- **Stylesheet extension:** `.xcss`
- **Editor:** Xenapse Studio
- **Modules:** `IMPORT FLOW` and `IMPORT MINDMAP`

Treat the name as a working choice, not a legal clearance. Before public release, search relevant trademark databases, especially WIPO's Global Brand Database and Tunisia's INNORPI resources, and consider a formal search for earlier marks. A general web search is not enough to establish availability.

---

## 18. Open decisions before grammar freeze

These questions should be answered before the parser implementation is considered stable:

1. **IMPORT behaviour:** enforce exactly one declaration, or retain the earlier last-import-wins idea? Recommended: exactly one.
2. **FLOW ellipse semantics:** should `( ... )` mean a generic event/state, or should it be removed in favour of fewer shapes?
3. **Database syntax:** recommended `DATABASE "..."` because `{ ... }` is needed for blocks.
4. **IF labels:** is `IF YES { ... } ELSE { ... }` enough for 1.0, or should branch labels be customizable from the start?
5. **Flow cycles:** keep all loops out of 1.0, or define a structured repeat construct after the first layout prototype?
6. **MINDMAP layouts:** implement radial first, or implement conventional tree layout first and add radial after?
7. **Text measurement:** browser measurement for Studio plus a CLI fallback, or a Rust-native font measurement stack from the beginning?
8. **Editor shell:** browser-first Studio, or desktop-first Tauri?

These are deliberate open decisions. They should not be silently settled by implementation convenience.

---

## 19. Reference projects and technical reading

These projects and specifications are useful references. They are not templates to copy wholesale.

- **Mermaid flowchart syntax:** useful for studying node and edge concepts, subgraphs, labels and the pitfalls of a compact diagram DSL.
  https://mermaid.js.org/syntax/flowchart.html
- **D2:** a modern declarative diagramming language and a useful comparison for source readability and layout behaviour.
  https://d2lang.com/
- **Graphviz `dot`:** a mature reference for directed, layered graph layout and crossing reduction.
  https://graphviz.org/docs/layouts/dot/
- **Eclipse Layout Kernel, ELK Layered:** a detailed reference for layered layouts, routing options and graph layout infrastructure.
  https://eclipse.dev/elk/reference/algorithms/org-eclipse-elk-layered.html
- **Reingold and Tilford, “Tidier Drawings of Trees”:** a foundational reference for tree layout.
  https://reingold.co/tidier-drawings.pdf
- **Logos:** Rust lexer-generation option.
  https://docs.rs/logos/latest/logos/
- **Chumsky:** Rust parser-combinator library with error-recovery support.
  https://docs.rs/chumsky/latest/chumsky/
- **Petgraph:** Rust graph data structures and algorithms.
  https://docs.rs/petgraph/latest/petgraph/
- **resvg:** Rust SVG rendering library worth evaluating for raster export.
  https://docs.rs/resvg/latest/resvg/
- **Tauri commands:** frontend-to-Rust communication for a possible desktop shell.
  https://v2.tauri.app/develop/calling-rust/
- **Rust to WebAssembly (MDN):** browser deployment path for a future WASM build.
  https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm
- **SVG 2 specification:** reference for SVG structure, text and rendering behaviour.
  https://www.w3.org/TR/SVG/
- **WIPO Global Brand Database:** trademark research resource.
  https://www.wipo.int/en/web/global-brand-database
- **INNORPI:** Tunisia's national industrial property institute and trademark resources.
  https://www.innorpi.tn/

---

## 20. Final product definition

Xenapse 1.0 should be a compact, local-first diagramming language with two excellent structural modules, a dependable Rust engine, a small styling language and a focused editor.

The product succeeds when a user can write a readable description, understand any diagnostics, and receive a diagram that is already well organized without dragging boxes around.

**The 1.0 objective is not to support every kind of diagram. It is to make FLOW and MINDMAP feel coherent, predictable and genuinely pleasant to use.**
