//! FLOW semantic validation (decision D4, blueprint §4.1). Collects every
//! problem in one pass; builds no model.

use crate::ast::{FlowNodeKind, FlowStmt, Header};
use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::source::Span;

use super::IdClassChecker;

pub(crate) fn validate_flow(header: &Header, body: &[FlowStmt]) -> Diagnostics {
    let mut v = Validator {
        diags: Diagnostics::new(),
        checker: IdClassChecker::new(),
        starts_seen: 0,
        has_end: false,
    };
    if body.is_empty() {
        v.missing_start(header.import_span);
        v.missing_end(header.import_span);
        return v.diags;
    }
    v.block(body, true, false, true);
    if v.starts_seen == 0 {
        v.missing_start(header.import_span);
    }
    if !v.has_end {
        v.missing_end(header.import_span);
    }
    v.diags
}

struct Validator {
    diags: Diagnostics,
    checker: IdClassChecker,
    starts_seen: usize,
    has_end: bool,
}

fn stmt_span(stmt: &FlowStmt) -> Span {
    match stmt {
        FlowStmt::Node(n) => n.span,
        FlowStmt::If(i) => i.span,
    }
}

/// Whether control never continues past this statement.
fn stmt_terminates(stmt: &FlowStmt) -> bool {
    match stmt {
        FlowStmt::Node(n) => n.kind == FlowNodeKind::End,
        FlowStmt::If(i) => {
            stmts_terminate(&i.yes_block.stmts) && stmts_terminate(&i.else_block.stmts)
        }
    }
}

/// A block terminates if its first terminating statement exists; anything
/// after it is unreachable and reported separately.
fn stmts_terminate(stmts: &[FlowStmt]) -> bool {
    stmts.iter().any(stmt_terminates)
}

impl Validator {
    fn missing_start(&mut self, span: Span) {
        self.diags.push(
            Diagnostic::error(
                codes::FLOW_MISSING_START,
                Origin::Source,
                span,
                "A FLOW diagram needs a START node.",
            )
            .with_help("Add `START` as the first statement."),
        );
    }

    fn missing_end(&mut self, span: Span) {
        self.diags.push(
            Diagnostic::error(
                codes::FLOW_MISSING_END,
                Origin::Source,
                span,
                "A FLOW diagram needs at least one END node.",
            )
            .with_help("Add `END` where a path finishes."),
        );
    }

    /// `top`: this is the document body. `flows_on`: normal completion of
    /// this block continues into a following statement. `reachable`: false
    /// inside code already reported as unreachable (suppresses dead-end noise).
    fn block(&mut self, stmts: &[FlowStmt], top: bool, flows_on: bool, reachable: bool) {
        let mut first_term: Option<usize> = None;
        for (i, stmt) in stmts.iter().enumerate() {
            let after_term = first_term.is_some();
            if first_term.is_none() && stmt_terminates(stmt) {
                first_term = Some(i);
            }
            let next_exists = i + 1 < stmts.len();
            match stmt {
                FlowStmt::Node(node) => {
                    self.checker
                        .check(node.id.as_ref(), &node.classes, &mut self.diags);
                    match node.kind {
                        FlowNodeKind::Start => {
                            self.starts_seen += 1;
                            if !(top && i == 0 && self.starts_seen == 1) {
                                self.diags.push(
                                    Diagnostic::error(
                                        codes::FLOW_START_MISPLACED,
                                        Origin::Source,
                                        node.span,
                                        "START must be the first top-level statement and appear only once.",
                                    )
                                    .with_help("Keep a single `START` at the top of the diagram."),
                                );
                            }
                        }
                        FlowNodeKind::End => self.has_end = true,
                        FlowNodeKind::Decision
                            if !matches!(stmts.get(i + 1), Some(FlowStmt::If(_))) =>
                        {
                            self.diags.push(
                                Diagnostic::error(
                                    codes::FLOW_DECISION_WITHOUT_IF,
                                    Origin::Source,
                                    node.span,
                                    "A decision must be immediately followed by an IF.",
                                )
                                .with_help(
                                    "Add `IF YES { ... } ELSE { ... }` right after the decision.",
                                ),
                            );
                        }
                        _ => {}
                    }
                }
                FlowStmt::If(f) => {
                    let after_decision = i > 0
                        && matches!(
                            stmts.get(i - 1),
                            Some(FlowStmt::Node(n)) if n.kind == FlowNodeKind::Decision
                        );
                    if !after_decision {
                        self.diags.push(
                            Diagnostic::error(
                                codes::FLOW_IF_WITHOUT_DECISION,
                                Origin::Source,
                                f.span,
                                "IF must immediately follow a decision node.",
                            )
                            .with_help("Put a decision node `< ... >` directly before the IF."),
                        );
                    }
                    let branch_flows = next_exists || flows_on;
                    let live = reachable && !after_term;
                    self.block(&f.yes_block.stmts, false, branch_flows, live);
                    self.block(&f.else_block.stmts, false, branch_flows, live);
                }
            }
        }

        if let Some(t) = first_term {
            if let Some(next) = stmts.get(t + 1) {
                self.diags.push(
                    Diagnostic::error(
                        codes::FLOW_UNREACHABLE,
                        Origin::Source,
                        stmt_span(next),
                        "This statement is unreachable.",
                    )
                    .with_help("Remove it, or move it before the END that ends this path."),
                );
            }
        } else if reachable && !flows_on {
            // Non-terminating block that does not flow into anything. A
            // trailing IF reports through its own branches.
            if let Some(FlowStmt::Node(last)) = stmts.last() {
                self.diags.push(
                    Diagnostic::warning(
                        codes::FLOW_DEAD_END,
                        Origin::Source,
                        last.span,
                        "This path does not reach an END.",
                    )
                    .with_help("Finish the path with `END`."),
                );
            }
        }
    }
}
