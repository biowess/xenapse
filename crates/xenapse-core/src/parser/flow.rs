//! FLOW body parsing (D3): statements, suffixes and nested `IF YES { } ELSE { }`.

use super::{describe, report_duplicate_import, Parser};
use crate::ast::{Block, FlowIf, FlowNode, FlowNodeKind, FlowStmt, Spanned};
use crate::diagnostics::codes;
use crate::lexer::{Keyword, ShapeKind, TokenKind};
use crate::source::Span;

/// Parses the FLOW body: zero or more statements up to `Eof`. An empty body
/// is allowed here; semantic validation reports the missing `START`.
pub(crate) fn parse_flow_body(p: &mut Parser) -> Vec<FlowStmt> {
    parse_stmts(p, false)
}

fn starts_stmt(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Keyword(
            Keyword::Start
                | Keyword::StartStar
                | Keyword::End
                | Keyword::EndStar
                | Keyword::If
                | Keyword::Database
        ) | TokenKind::Shape { .. }
    )
}

/// Skips a bad statement. With `force`, the current token is consumed even if
/// it could start a statement. Stops (without consuming) at the next
/// statement starter, a `}` that closes the enclosing block, or `Eof`;
/// braces opened while skipping are matched so nested blocks stay in sync.
fn recover(p: &mut Parser, mut force: bool) {
    let mut depth = 0usize;
    loop {
        let (eof, rbrace, lbrace, starter) = {
            let k = &p.peek().kind;
            (
                *k == TokenKind::Eof,
                *k == TokenKind::RBrace,
                *k == TokenKind::LBrace,
                starts_stmt(k),
            )
        };
        if eof {
            break;
        }
        if depth == 0 && !force && (rbrace || starter) {
            break;
        }
        force = false;
        if lbrace {
            depth += 1;
        } else if rbrace {
            depth = depth.saturating_sub(1);
        }
        p.bump();
    }
}

/// Skips to just past the `}` matching an already-consumed `{`.
fn skip_block_rest(p: &mut Parser) {
    let mut depth = 1usize;
    while depth > 0 && !p.at_eof() {
        match p.bump().kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => depth -= 1,
            _ => {}
        }
    }
}

fn parse_stmts(p: &mut Parser, in_block: bool) -> Vec<FlowStmt> {
    let mut out = Vec::new();
    let first_import = p.first_import_span();
    loop {
        let before = p.pos;
        let tok = p.peek().clone();
        match &tok.kind {
            TokenKind::Eof => break,
            TokenKind::RBrace => {
                if in_block {
                    break;
                }
                p.error(
                    codes::SYN_STRAY_RBRACE,
                    tok.span,
                    "unmatched `}`: there is no open block to close".to_owned(),
                    Some("remove this `}` or add the `{` that opens its block".to_owned()),
                );
                p.bump();
            }
            TokenKind::Keyword(
                Keyword::Start | Keyword::StartStar | Keyword::End | Keyword::EndStar,
            ) => {
                if let Some(n) = parse_start_end(p) {
                    out.push(FlowStmt::Node(n));
                }
            }
            TokenKind::Keyword(Keyword::Database) => {
                if let Some(n) = parse_database(p) {
                    out.push(FlowStmt::Node(n));
                }
            }
            TokenKind::Shape { kind, label } => {
                p.bump();
                let node = finish_node(p, shape_kind(*kind), Some(label.clone()), tok.span);
                out.push(FlowStmt::Node(node));
            }
            TokenKind::Keyword(Keyword::If) => {
                if let Some(f) = parse_if(p) {
                    out.push(FlowStmt::If(f));
                }
            }
            TokenKind::Keyword(Keyword::Else) => {
                p.error(
                    codes::SYN_ELSE_WITHOUT_IF,
                    tok.span,
                    "`ELSE` without a matching `IF YES` block".to_owned(),
                    Some(
                        "`ELSE` must directly follow the closing `}` of an `IF YES { ... }` block"
                            .to_owned(),
                    ),
                );
                recover(p, true);
            }
            TokenKind::Keyword(Keyword::Root) => {
                p.error(
                    codes::SYN_MODULE_ISOLATION,
                    tok.span,
                    "`ROOT` is a MINDMAP construct and is not allowed in FLOW".to_owned(),
                    Some("use `IMPORT MINDMAP` for mind maps; FLOW starts with `START`".to_owned()),
                );
                recover(p, true);
            }
            TokenKind::Str(_) => {
                p.error(
                    codes::SYN_MODULE_ISOLATION,
                    tok.span,
                    "a bare string is a MINDMAP construct and is not allowed in FLOW".to_owned(),
                    Some(
                        "write a FLOW node such as `[ text ]`, `( text )`, `< text >`, `/ text /`, `| text |` or `DATABASE \"text\"`"
                            .to_owned(),
                    ),
                );
                recover(p, true);
            }
            TokenKind::Keyword(Keyword::Import) => {
                report_duplicate_import(p, first_import);
            }
            TokenKind::Keyword(Keyword::Stylesheet | Keyword::Direction | Keyword::Layout) => {
                p.error(
                    codes::SYN_DIRECTIVE_IN_BODY,
                    tok.span,
                    "directives must come before the diagram body".to_owned(),
                    Some("move STYLESHEET, DIRECTION and LAYOUT directly under IMPORT".to_owned()),
                );
                recover(p, true);
            }
            other => {
                p.error(
                    codes::SYN_EXPECTED_FLOW_STMT,
                    tok.span,
                    format!("expected a FLOW statement, found {}", describe(other)),
                    Some("a statement is START, END, a shape, DATABASE \"...\" or IF YES { ... } ELSE { ... }".to_owned()),
                );
                recover(p, true);
            }
        }
        // Progress guard: never loop on a token that nothing consumed.
        if p.pos == before && !p.at_eof() {
            p.bump();
        }
    }
    out
}

fn shape_kind(k: ShapeKind) -> FlowNodeKind {
    match k {
        ShapeKind::Process => FlowNodeKind::Process,
        ShapeKind::Event => FlowNodeKind::Event,
        ShapeKind::Decision => FlowNodeKind::Decision,
        ShapeKind::InputOutput => FlowNodeKind::InputOutput,
        ShapeKind::Subprocess => FlowNodeKind::Subprocess,
    }
}

/// Parses the optional `#id` then `.class`* suffix and builds the node. The
/// head token has already been consumed; `head` is its span.
fn finish_node(p: &mut Parser, kind: FlowNodeKind, label: Option<String>, head: Span) -> FlowNode {
    let mut id: Option<Spanned<String>> = None;
    let mut classes: Vec<Spanned<String>> = Vec::new();
    let mut span = head;
    loop {
        match &p.peek().kind {
            TokenKind::Id(name) => {
                let name = name.clone();
                let tok_span = p.bump().span;
                span = span.merge(tok_span);
                if id.is_some() {
                    p.error(
                        codes::SYN_DUPLICATE_ID,
                        tok_span,
                        "a node can have only one ID".to_owned(),
                        Some("remove this extra `#id`".to_owned()),
                    );
                } else {
                    if !classes.is_empty() {
                        p.error(
                            codes::SYN_ID_AFTER_CLASS,
                            tok_span,
                            "ID must come before classes".to_owned(),
                            Some("write the `#id` first, then the `.class` names, e.g. `[ Step ] #id .class`".to_owned()),
                        );
                    }
                    id = Some(Spanned {
                        value: name,
                        span: tok_span,
                    });
                }
            }
            TokenKind::Class(name) => {
                let name = name.clone();
                let tok_span = p.bump().span;
                span = span.merge(tok_span);
                classes.push(Spanned {
                    value: name,
                    span: tok_span,
                });
            }
            _ => break,
        }
    }
    FlowNode {
        kind,
        label,
        id,
        classes,
        span,
        hidden: false,
    }
}

fn parse_start_end(p: &mut Parser) -> Option<FlowNode> {
    let tok = p.bump();
    let (kind, name, hidden) = match tok.kind {
        TokenKind::Keyword(Keyword::Start) => (FlowNodeKind::Start, "START", false),
        TokenKind::Keyword(Keyword::StartStar) => (FlowNodeKind::Start, "START*", true),
        TokenKind::Keyword(Keyword::End) => (FlowNodeKind::End, "END", false),
        TokenKind::Keyword(Keyword::EndStar) => (FlowNodeKind::End, "END*", true),
        _ => unreachable!(),
    };
    if let TokenKind::Str(_) = p.peek().kind {
        let span = p.bump().span;
        p.error(
            codes::SYN_UNEXPECTED_LABEL,
            span,
            format!("`{name}` takes no label"),
            Some(format!(
                "write `{name}` alone, or use a shape such as `[ text ]` for a labelled step"
            )),
        );
    }
    let mut node = finish_node(p, kind, None, tok.span);
    node.hidden = hidden;
    Some(node)
}

fn parse_database(p: &mut Parser) -> Option<FlowNode> {
    let tok = p.bump();
    let label = if let TokenKind::Str(s) = &p.peek().kind {
        let s = s.clone();
        p.bump();
        s
    } else {
        let found = describe(&p.peek().kind);
        let span = p.peek().span;
        p.error(
            codes::SYN_EXPECTED_LABEL,
            span,
            format!("`DATABASE` requires a quoted label, found {found}"),
            Some("write DATABASE \"Patient records\"".to_owned()),
        );
        recover(p, false);
        return None;
    };
    Some(finish_node(
        p,
        FlowNodeKind::Database,
        Some(label),
        tok.span,
    ))
}

fn parse_if(p: &mut Parser) -> Option<FlowIf> {
    let if_tok = p.bump();
    let mut yes_span = if_tok.span;
    let next = p.peek().clone();
    match &next.kind {
        TokenKind::Keyword(Keyword::Yes) => {
            yes_span = yes_span.merge(p.bump().span);
        }
        TokenKind::Word(w) if w == "NO" => {
            p.bump();
            p.error(
                codes::SYN_IF_NO,
                next.span,
                "`IF NO` is not supported".to_owned(),
                Some(
                    "write `IF YES { ... } ELSE { ... }`; the ELSE block is the NO branch"
                        .to_owned(),
                ),
            );
            yes_span = yes_span.merge(next.span);
        }
        other => {
            p.error(
                codes::SYN_IF_EXPECTED_YES,
                next.span,
                format!("expected `YES` after `IF`, found {}", describe(other)),
                Some("write `IF YES { ... } ELSE { ... }`".to_owned()),
            );
            if *other != TokenKind::LBrace {
                recover(p, false);
                return None;
            }
        }
    }

    let Some(yes_block) = parse_block(p, "YES") else {
        recover(p, false);
        return None;
    };

    let else_tok = p.peek().clone();
    if else_tok.kind != TokenKind::Keyword(Keyword::Else) {
        p.error(
            codes::SYN_MISSING_ELSE,
            else_tok.span,
            format!(
                "`IF YES` block must be followed by `ELSE`, found {}",
                describe(&else_tok.kind)
            ),
            Some("both branches are required: `IF YES { ... } ELSE { ... }`".to_owned()),
        );
        return None;
    }
    p.bump();

    let Some(else_block) = parse_block(p, "ELSE") else {
        recover(p, false);
        return None;
    };

    Some(FlowIf {
        span: if_tok.span.merge(else_block.span),
        yes_span,
        else_span: else_tok.span,
        yes_block,
        else_block,
    })
}

/// Parses `{ stmt+ }`. Returns `None` if there is no `{` or the nesting limit
/// was hit (both already diagnosed).
fn parse_block(p: &mut Parser, branch: &str) -> Option<Block> {
    let open = if p.peek().kind == TokenKind::LBrace {
        p.bump().span
    } else {
        let found = describe(&p.peek().kind);
        let span = p.peek().span;
        p.error(
            codes::SYN_UNEXPECTED_TOKEN,
            span,
            format!("expected `{{` to open the {branch} branch, found {found}"),
            None,
        );
        return None;
    };

    if !p.enter_block() {
        p.leave_block();
        skip_block_rest(p);
        return None;
    }
    let stmts = parse_stmts(p, true);
    p.leave_block();

    if p.peek().kind == TokenKind::RBrace {
        let close = p.bump().span;
        let span = open.merge(close);
        if stmts.is_empty() {
            p.error(
                codes::SYN_EMPTY_BLOCK,
                span,
                format!("the {branch} branch is empty"),
                Some("a block needs at least one statement".to_owned()),
            );
        }
        Some(Block { stmts, span })
    } else {
        p.error(
            codes::SYN_UNCLOSED_BLOCK,
            open,
            "missing `}`: this block is never closed".to_owned(),
            Some("add a `}` to close the block opened here".to_owned()),
        );
        let end = stmts.last().map_or(open, stmt_span);
        Some(Block {
            stmts,
            span: open.merge(end),
        })
    }
}

fn stmt_span(s: &FlowStmt) -> Span {
    match s {
        FlowStmt::Node(n) => n.span,
        FlowStmt::If(f) => f.span,
    }
}
