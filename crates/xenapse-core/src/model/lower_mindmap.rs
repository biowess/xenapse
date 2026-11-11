//! Lowers a validated MINDMAP AST into a pre-order `MindmapDiagram` (D5).

use super::mindmap::{MindNode, MindmapDiagram};
use super::NodeId;
use crate::ast::{self, Header};
use crate::lang::LayoutMode;

/// Iterative, so tree depth cannot overflow the call stack.
pub(crate) fn lower_mindmap(header: &Header, root: &ast::MindNode) -> MindmapDiagram {
    let mut nodes: Vec<MindNode> = Vec::new();
    // (AST node, parent index, depth); children are pushed in reverse so they
    // pop in source order, which yields pre-order ids.
    let mut stack: Vec<(&ast::MindNode, Option<NodeId>, usize)> = vec![(root, None, 0)];
    while let Some((node, parent, depth)) = stack.pop() {
        let id = NodeId(nodes.len());
        if let Some(p) = parent {
            if let Some(parent_node) = nodes.get_mut(p.0) {
                parent_node.children.push(id);
            }
        }
        nodes.push(MindNode {
            id,
            source_id: node.id.as_ref().map(|i| i.value.clone()),
            label: node.label.clone(),
            classes: node.classes.iter().map(|c| c.value.clone()).collect(),
            parent,
            children: Vec::new(),
            depth,
            span: node.span,
        });
        for child in node.children.iter().rev() {
            stack.push((child, Some(id), depth.saturating_add(1)));
        }
    }
    let layout = header
        .layout
        .as_ref()
        .map_or(LayoutMode::Radial, |l| l.value);
    MindmapDiagram { nodes, layout }
}
