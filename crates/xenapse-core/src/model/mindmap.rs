//! MINDMAP model: a rooted tree stored as a pre-order arena.

use super::NodeId;
use crate::lang::LayoutMode;
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MindNode {
    pub(crate) id: NodeId,
    pub(crate) source_id: Option<String>,
    pub(crate) label: String,
    pub(crate) classes: Vec<String>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) depth: usize,
    pub(crate) span: Span,
}

/// `nodes` is in pre-order; the root is index 0. Edge `i` is the implicit
/// parent→child edge ending at node `i + 1` (D5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MindmapDiagram {
    pub(crate) nodes: Vec<MindNode>,
    pub(crate) layout: LayoutMode,
}

impl MindmapDiagram {
    pub(crate) fn edge_count(&self) -> usize {
        self.nodes.len().saturating_sub(1)
    }

    /// `(parent, child)` of edge `i`, or `None` if out of range or the child
    /// has no parent.
    pub(crate) fn edge(&self, i: usize) -> Option<(NodeId, NodeId)> {
        let child = self.nodes.get(i.checked_add(1)?)?;
        Some((child.parent?, child.id))
    }

    /// Source span of edge `i`: that of its child node.
    pub(crate) fn edge_span(&self, i: usize) -> Option<Span> {
        Some(self.nodes.get(i.checked_add(1)?)?.span)
    }
}
