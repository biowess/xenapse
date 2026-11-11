//! Normalized diagram model: meaning only, no syntax and no coordinates.
//! FLOW is a directed graph; MINDMAP is a rooted tree (decisions D4, D5).

pub(crate) mod flow;
pub(crate) mod lower_flow;
pub(crate) mod lower_mindmap;
pub(crate) mod mindmap;

pub(crate) use flow::FlowDiagram;
pub(crate) use mindmap::MindmapDiagram;

/// Typed index of a node, in source (pre-order) order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(pub(crate) usize);

/// Typed index of an edge, in source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EdgeId(pub(crate) usize);

/// A lowered document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Diagram {
    Flow(FlowDiagram),
    Mindmap(MindmapDiagram),
}
