//! Syntax tree. Faithful to the source, spans only, no coordinates.

use crate::lang::{Direction, LayoutMode, Module};
use crate::source::Span;

/// A value together with the source span it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned<T> {
    pub(crate) value: T,
    pub(crate) span: Span,
}

/// The document header: `IMPORT` plus directives.
///
/// `import_span` covers `IMPORT` through the module keyword. Each directive's
/// span is that of its value token (the string, `TB`/`LR`, or layout keyword).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) module: Module,
    pub(crate) import_span: Span,
    pub(crate) stylesheet: Option<Spanned<String>>,
    pub(crate) direction: Option<Spanned<Direction>>,
    pub(crate) layout: Option<Spanned<LayoutMode>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlowNodeKind {
    Start,
    End,
    Process,
    Event,
    Decision,
    InputOutput,
    Subprocess,
    Database,
}

/// A FLOW node. `span` covers the keyword or shape through the last suffix
/// token. `label` is `None` for `START`/`END` (defaults are applied later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowNode {
    pub(crate) kind: FlowNodeKind,
    pub(crate) label: Option<String>,
    pub(crate) id: Option<Spanned<String>>,
    pub(crate) classes: Vec<Spanned<String>>,
    pub(crate) span: Span,
    pub(crate) hidden: bool,
}

/// `IF YES { } ELSE { }`. `span` runs from `IF` through the closing `}` of
/// the ELSE block; `yes_span` covers `IF YES`; `else_span` covers `ELSE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowIf {
    pub(crate) span: Span,
    pub(crate) yes_span: Span,
    pub(crate) else_span: Span,
    pub(crate) yes_block: Block,
    pub(crate) else_block: Block,
}

/// A `{ ... }` block; `span` covers both braces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    pub(crate) stmts: Vec<FlowStmt>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FlowStmt {
    Node(FlowNode),
    If(FlowIf),
}

/// A MINDMAP node. `span` covers the `ROOT` keyword (root only) or the label
/// through the last suffix token or the closing `}`; `label_span` covers the
/// label string token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MindNode {
    pub(crate) label: String,
    pub(crate) label_span: Span,
    pub(crate) id: Option<Spanned<String>>,
    pub(crate) classes: Vec<Spanned<String>>,
    pub(crate) children: Vec<MindNode>,
    pub(crate) span: Span,
}

/// The module-specific body of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Body {
    Flow(Vec<FlowStmt>),
    /// The root node. If the source has no `ROOT`, this is an empty-label
    /// placeholder at end of input and the diagnostics contain an error.
    Mindmap(MindNode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    pub(crate) header: Header,
    pub(crate) body: Body,
}
