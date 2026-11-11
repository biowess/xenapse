//! FLOW model: a directed graph with typed node and edge indices.

use super::{EdgeId, NodeId};
use crate::lang::Direction;
use crate::source::Span;

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

impl FlowNodeKind {
    /// Built-in classes matched by this kind (D4). `InputOutput` matches both
    /// `.input` and `.output`.
    pub(crate) fn builtin_classes(&self) -> &'static [&'static str] {
        match self {
            FlowNodeKind::Start => &["start"],
            FlowNodeKind::End => &["end"],
            FlowNodeKind::Process => &["process"],
            FlowNodeKind::Event => &["event"],
            FlowNodeKind::Decision => &["decision"],
            FlowNodeKind::InputOutput => &["input", "output"],
            FlowNodeKind::Subprocess => &["subprocess"],
            FlowNodeKind::Database => &["database"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlowEdgeKind {
    Sequence,
    Yes,
    No,
}

impl FlowEdgeKind {
    /// Built-in edge class (D4): `yes` / `no`; sequence edges have none.
    pub(crate) fn builtin_class(&self) -> Option<&'static str> {
        match self {
            FlowEdgeKind::Sequence => None,
            FlowEdgeKind::Yes => Some("yes"),
            FlowEdgeKind::No => Some("no"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowNode {
    pub(crate) id: NodeId,
    pub(crate) source_id: Option<String>,
    pub(crate) label: String,
    pub(crate) kind: FlowNodeKind,
    pub(crate) classes: Vec<String>,
    pub(crate) span: Span,
    pub(crate) hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowEdge {
    pub(crate) id: EdgeId,
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    pub(crate) kind: FlowEdgeKind,
    pub(crate) label: Option<String>,
    pub(crate) span: Span,
}

/// A directed graph. Node and edge ids are positions in `nodes` / `edges`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowDiagram {
    pub(crate) nodes: Vec<FlowNode>,
    pub(crate) edges: Vec<FlowEdge>,
    pub(crate) start: NodeId,
    pub(crate) direction: Direction,
    successors: Vec<Vec<EdgeId>>,
    predecessors: Vec<Vec<EdgeId>>,
}

impl FlowDiagram {
    /// Builds the adjacency lists, in edge order. Returns `None` if `start`
    /// or any edge endpoint is not a valid index into `nodes`.
    pub(crate) fn new(
        nodes: Vec<FlowNode>,
        edges: Vec<FlowEdge>,
        start: NodeId,
        direction: Direction,
    ) -> Option<FlowDiagram> {
        let n = nodes.len();
        if start.0 >= n {
            return None;
        }
        let mut successors = vec![Vec::new(); n];
        let mut predecessors = vec![Vec::new(); n];
        for (i, e) in edges.iter().enumerate() {
            if e.from.0 >= n || e.to.0 >= n {
                return None;
            }
            successors.get_mut(e.from.0)?.push(EdgeId(i));
            predecessors.get_mut(e.to.0)?.push(EdgeId(i));
        }
        Some(FlowDiagram {
            nodes,
            edges,
            start,
            direction,
            successors,
            predecessors,
        })
    }

    pub(crate) fn node(&self, id: NodeId) -> Option<&FlowNode> {
        self.nodes.get(id.0)
    }

    pub(crate) fn edge(&self, id: EdgeId) -> Option<&FlowEdge> {
        self.edges.get(id.0)
    }

    /// Outgoing edge ids of `id` (empty if out of range).
    pub(crate) fn successors(&self, id: NodeId) -> &[EdgeId] {
        self.successors.get(id.0).map_or(&[], Vec::as_slice)
    }

    /// Incoming edge ids of `id` (empty if out of range).
    pub(crate) fn predecessors(&self, id: NodeId) -> &[EdgeId] {
        self.predecessors.get(id.0).map_or(&[], Vec::as_slice)
    }
}
