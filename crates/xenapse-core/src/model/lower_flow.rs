//! Lowers a validated FLOW AST into a `FlowDiagram` (decisions D4).

use super::flow::{FlowDiagram, FlowEdge, FlowEdgeKind, FlowNode, FlowNodeKind};
use super::{EdgeId, NodeId};
use crate::ast::{self, FlowStmt, Header};
use crate::lang::Direction;
use crate::source::Span;

/// Nesting guard; the parser already limits depth to 128.
const MAX_DEPTH: usize = 256;

/// An open end of the flow: the edge that will be created once the next node
/// is known. `span` is `None` for sequence edges (target span is used).
#[derive(Clone)]
struct Tail {
    from: NodeId,
    kind: FlowEdgeKind,
    label: Option<String>,
    span: Option<Span>,
}

struct Lowerer {
    nodes: Vec<FlowNode>,
    edges: Vec<FlowEdge>,
}

/// Precondition: `validate_flow` reported no errors. Returns `None` if an
/// invariant is violated.
pub(crate) fn lower_flow(header: &Header, body: &[FlowStmt]) -> Option<FlowDiagram> {
    let first = match body.first()? {
        FlowStmt::Node(n) if n.kind == ast::FlowNodeKind::Start => n,
        _ => return None,
    };
    let mut lw = Lowerer {
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    lw.block(body, Vec::new(), 0)?;
    let start = lw.nodes.iter().find(|n| n.span == first.span)?.id;
    let direction = header
        .direction
        .as_ref()
        .map_or(Direction::TopToBottom, |d| d.value);
    FlowDiagram::new(lw.nodes, lw.edges, start, direction)
}

impl Lowerer {
    /// Lowers a statement list; returns the open tails after it.
    fn block(
        &mut self,
        stmts: &[FlowStmt],
        mut tails: Vec<Tail>,
        depth: usize,
    ) -> Option<Vec<Tail>> {
        if depth > MAX_DEPTH {
            return None;
        }
        for stmt in stmts {
            tails = match stmt {
                FlowStmt::Node(n) => self.node(n, tails)?,
                FlowStmt::If(i) => {
                    let decision = match tails.as_slice() {
                        [t] if t.kind == FlowEdgeKind::Sequence && t.span.is_none() => t.from,
                        _ => return None,
                    };
                    if self.nodes.get(decision.0)?.kind != FlowNodeKind::Decision {
                        return None;
                    }
                    let yes = Tail {
                        from: decision,
                        kind: FlowEdgeKind::Yes,
                        label: Some("Yes".to_string()),
                        span: Some(i.yes_span),
                    };
                    let no = Tail {
                        from: decision,
                        kind: FlowEdgeKind::No,
                        label: Some("No".to_string()),
                        span: Some(i.else_span),
                    };
                    let mut open = self.block(&i.yes_block.stmts, vec![yes], depth + 1)?;
                    open.extend(self.block(&i.else_block.stmts, vec![no], depth + 1)?);
                    open
                }
            };
        }
        Some(tails)
    }

    fn node(&mut self, n: &ast::FlowNode, tails: Vec<Tail>) -> Option<Vec<Tail>> {
        // Only the very first node may have no incoming tails.
        if tails.is_empty() && !self.nodes.is_empty() {
            return None;
        }
        let (kind, label) = match n.kind {
            ast::FlowNodeKind::Start => (FlowNodeKind::Start, "Start".to_string()),
            ast::FlowNodeKind::End => (FlowNodeKind::End, "End".to_string()),
            ast::FlowNodeKind::Process => (FlowNodeKind::Process, n.label.clone()?),
            ast::FlowNodeKind::Event => (FlowNodeKind::Event, n.label.clone()?),
            ast::FlowNodeKind::Decision => (FlowNodeKind::Decision, n.label.clone()?),
            ast::FlowNodeKind::InputOutput => (FlowNodeKind::InputOutput, n.label.clone()?),
            ast::FlowNodeKind::Subprocess => (FlowNodeKind::Subprocess, n.label.clone()?),
            ast::FlowNodeKind::Database => (FlowNodeKind::Database, n.label.clone()?),
        };
        let id = NodeId(self.nodes.len());
        self.nodes.push(FlowNode {
            id,
            source_id: n.id.as_ref().map(|s| s.value.clone()),
            label,
            kind,
            classes: n.classes.iter().map(|c| c.value.clone()).collect(),
            span: n.span,
            hidden: n.hidden,
        });
        for t in tails {
            let eid = EdgeId(self.edges.len());
            self.edges.push(FlowEdge {
                id: eid,
                from: t.from,
                to: id,
                kind: t.kind,
                label: t.label,
                span: t.span.unwrap_or(n.span),
            });
        }
        if kind == FlowNodeKind::End {
            return Some(Vec::new());
        }
        Some(vec![Tail {
            from: id,
            kind: FlowEdgeKind::Sequence,
            label: None,
            span: None,
        }])
    }
}
