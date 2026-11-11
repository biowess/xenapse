//! MINDMAP semantic validation (decision D5, blueprint §4.2). The parser
//! already enforces the single ROOT and non-empty blocks; this checks IDs
//! and class names over every node in pre-order.

use crate::ast::MindNode;
use crate::diagnostics::Diagnostics;

use super::IdClassChecker;

pub(crate) fn validate_mindmap(root: &MindNode) -> Diagnostics {
    let mut diags = Diagnostics::new();
    let mut checker = IdClassChecker::new();
    // Explicit stack: children are pushed in reverse so they pop in source order.
    let mut stack: Vec<&MindNode> = vec![root];
    while let Some(node) = stack.pop() {
        checker.check(node.id.as_ref(), &node.classes, &mut diags);
        stack.extend(node.children.iter().rev());
    }
    diags
}
