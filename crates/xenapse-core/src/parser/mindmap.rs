//! MINDMAP body parsing (D3): `ROOT string suffix [block]` and nested
//! `string suffix [block]` children.

use super::{describe, report_duplicate_import, Parser};
use crate::ast::{MindNode, Spanned};
use crate::diagnostics::{codes, Diagnostic, Origin};
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::Span;

/// Parses the MINDMAP body up to `Eof`. Always returns a node; when the
/// source has no `ROOT` it is an empty placeholder and an error was reported.
pub(crate) fn parse_mindmap_body(p: &mut Parser) -> MindNode {
    let first_import = p.first_import_span();
    let mut root: Option<MindNode> = None;
    let mut missing_reported = false;
    loop {
        let before = p.pos;
        let tok = p.peek().clone();
        match &tok.kind {
            TokenKind::Eof => break,
            TokenKind::RBrace => {
                p.error(
                    codes::SYN_STRAY_RBRACE,
                    tok.span,
                    "unmatched `}`: there is no open block to close".to_owned(),
                    Some("remove this `}` or add the `{` that opens its block".to_owned()),
                );
                p.bump();
            }
            TokenKind::Keyword(Keyword::Root) => {
                p.bump();
                let node = parse_root_rest(p, tok.span);
                match root.as_ref().map(|r| r.span) {
                    None => root = Some(node),
                    Some(first) => {
                        let d = Diagnostic::error(
                            codes::MIND_SECOND_ROOT,
                            Origin::Source,
                            tok.span,
                            "a MINDMAP has exactly one ROOT",
                        )
                        .with_help(
                            "remove this ROOT or make it a child inside the first ROOT's block",
                        )
                        .with_related(first, "first ROOT");
                        p.report(d);
                    }
                }
            }
            _ if common(p, &tok, first_import) => {}
            kind => {
                if root.is_none() && !missing_reported {
                    missing_reported = true;
                    report_missing_root(p, tok.span);
                } else {
                    p.error(
                        codes::MIND_EXTRA_CONTENT,
                        tok.span,
                        format!(
                            "unexpected {} after the ROOT: a MINDMAP has a single root",
                            describe(kind)
                        ),
                        Some("put other nodes inside the ROOT's `{ ... }` block".to_owned()),
                    );
                }
                if matches!(kind, TokenKind::Str(_)) {
                    // Consume the whole node (and its block) to stay in sync.
                    parse_node(p);
                } else {
                    recover(p, true);
                }
            }
        }
        if p.pos == before && !p.at_eof() {
            p.bump();
        }
    }
    if root.is_none() && !missing_reported {
        let span = p.peek().span;
        report_missing_root(p, span);
    }
    root.unwrap_or_else(|| {
        let end = Span::new(p.peek().span.start, p.peek().span.start);
        MindNode {
            label: String::new(),
            label_span: end,
            id: None,
            classes: Vec::new(),
            children: Vec::new(),
            span: end,
        }
    })
}

fn report_missing_root(p: &mut Parser, span: Span) {
    p.error(
        codes::MIND_MISSING_ROOT,
        span,
        "a MINDMAP must begin with ROOT".to_owned(),
        Some(
            "write `ROOT \"Title\"` (optionally followed by a `{ ... }` block of children)"
                .to_owned(),
        ),
    );
}

fn is_flow_construct(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Keyword(
            Keyword::Start
                | Keyword::StartStar
                | Keyword::End
                | Keyword::EndStar
                | Keyword::If
                | Keyword::Else
                | Keyword::Yes
                | Keyword::Database
        ) | TokenKind::Shape { .. }
    )
}

/// Handles statement-position tokens that are errors in both body positions:
/// duplicate `IMPORT`, directives, and FLOW constructs. Returns `true` if the
/// token was handled (and consumed).
fn common(p: &mut Parser, tok: &Token, first_import: Span) -> bool {
    match &tok.kind {
        TokenKind::Keyword(Keyword::Import) => {
            report_duplicate_import(p, first_import);
            true
        }
        TokenKind::Keyword(Keyword::Stylesheet | Keyword::Direction | Keyword::Layout) => {
            p.error(
                codes::SYN_DIRECTIVE_IN_BODY,
                tok.span,
                "directives must come before the diagram body".to_owned(),
                Some("move STYLESHEET, DIRECTION and LAYOUT directly under IMPORT".to_owned()),
            );
            recover(p, true);
            true
        }
        k if is_flow_construct(k) => {
            p.error(
                codes::MIND_FLOW_CONSTRUCT,
                tok.span,
                "FLOW construct not allowed in MINDMAP".to_owned(),
                Some(format!(
                    "{} is a FLOW construct; MINDMAP nodes are quoted labels such as `\"Topic\"`",
                    describe(k)
                )),
            );
            recover(p, true);
            true
        }
        _ => false,
    }
}

fn starts_node(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Str(_)
            | TokenKind::Word(_)
            | TokenKind::Shape { .. }
            | TokenKind::Keyword(
                Keyword::Root
                    | Keyword::Start
                    | Keyword::StartStar
                    | Keyword::End
                    | Keyword::EndStar
                    | Keyword::If
                    | Keyword::Database,
            )
    )
}

/// Skips a bad item. With `force`, the current token is consumed even if it
/// could start a node. Stops (without consuming) at the next node starter, a
/// `}` closing the enclosing block, or `Eof`; braces opened while skipping
/// are matched so nested blocks stay in sync.
fn recover(p: &mut Parser, mut force: bool) {
    let mut depth = 0usize;
    loop {
        let (eof, rbrace, lbrace, starter) = {
            let k = &p.peek().kind;
            (
                *k == TokenKind::Eof,
                *k == TokenKind::RBrace,
                *k == TokenKind::LBrace,
                starts_node(k),
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

/// Reads a label. A quoted string is the normal case; a bare word is
/// diagnosed and used as the label so parsing can continue. Anything else is
/// diagnosed and yields an empty label without consuming a token.
fn take_label(p: &mut Parser, what: &str) -> (String, Span) {
    let tok = p.peek().clone();
    match &tok.kind {
        TokenKind::Str(s) => {
            p.bump();
            (s.clone(), tok.span)
        }
        TokenKind::Word(w) => {
            p.bump();
            p.error(
                codes::MIND_UNQUOTED_LABEL,
                tok.span,
                "MINDMAP labels must be quoted".to_owned(),
                Some(format!("write \"{w}\" with double quotes")),
            );
            (w.clone(), tok.span)
        }
        other => {
            p.error(
                codes::MIND_EXPECTED_LABEL,
                tok.span,
                format!("{what} requires a quoted label, found {}", describe(other)),
                Some("write a label in double quotes, e.g. `\"Topic\"`".to_owned()),
            );
            (String::new(), Span::new(tok.span.start, tok.span.start))
        }
    }
}

/// Parses the rest of a root node; the `ROOT` keyword (`head`) is consumed.
fn parse_root_rest(p: &mut Parser, head: Span) -> MindNode {
    let (label, label_span) = take_label(p, "`ROOT`");
    finish_node(p, head, label, label_span)
}

/// Parses a child node starting at its label token (a string or bare word).
fn parse_node(p: &mut Parser) -> MindNode {
    let head = p.peek().span;
    let (label, label_span) = take_label(p, "a node");
    finish_node(p, head, label, label_span)
}

/// Parses `[#id] (.class)* [block]` and builds the node.
fn finish_node(p: &mut Parser, head: Span, label: String, label_span: Span) -> MindNode {
    let mut id: Option<Spanned<String>> = None;
    let mut classes: Vec<Spanned<String>> = Vec::new();
    let mut span = head.merge(label_span);
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
                            Some("write the `#id` first, then the `.class` names, e.g. `\"Topic\" #id .class`".to_owned()),
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
    let mut children = Vec::new();
    if p.peek().kind == TokenKind::LBrace {
        let (kids, block_span) = parse_block(p);
        children = kids;
        span = span.merge(block_span);
    }
    MindNode {
        label,
        label_span,
        id,
        classes,
        children,
        span,
    }
}

/// Parses `{ node+ }`; the current token is the `{`. Returns the children and
/// the span of the block (just the `{` when it was skipped or unclosed and
/// empty).
fn parse_block(p: &mut Parser) -> (Vec<MindNode>, Span) {
    let open = p.bump().span;
    if !p.enter_block() {
        p.leave_block();
        skip_block_rest(p);
        return (Vec::new(), open);
    }
    let (children, saw_any) = parse_children(p);
    p.leave_block();

    if p.peek().kind == TokenKind::RBrace {
        let span = open.merge(p.bump().span);
        if !saw_any {
            p.error(
                codes::SYN_EMPTY_BLOCK,
                span,
                "this block is empty".to_owned(),
                Some(
                    "a block needs at least one child label; remove the braces for a leaf"
                        .to_owned(),
                ),
            );
        }
        (children, span)
    } else {
        p.error(
            codes::SYN_UNCLOSED_BLOCK,
            open,
            "missing `}`: this block is never closed".to_owned(),
            Some("add a `}` to close the block opened here".to_owned()),
        );
        let end = children.last().map_or(open, |c| c.span);
        (children, open.merge(end))
    }
}

/// Parses children up to the closing `}` or `Eof` (neither consumed). The
/// flag says whether any item, valid or not, was seen.
fn parse_children(p: &mut Parser) -> (Vec<MindNode>, bool) {
    let mut out = Vec::new();
    let mut saw_any = false;
    let first_import = p.first_import_span();
    loop {
        let before = p.pos;
        let tok = p.peek().clone();
        match &tok.kind {
            TokenKind::Eof | TokenKind::RBrace => break,
            TokenKind::Str(_) | TokenKind::Word(_) => {
                saw_any = true;
                out.push(parse_node(p));
            }
            TokenKind::Keyword(Keyword::Root) => {
                saw_any = true;
                p.bump();
                p.error(
                    codes::MIND_SECOND_ROOT,
                    tok.span,
                    "ROOT is only allowed once, at the top level".to_owned(),
                    Some("write children as quoted labels such as `\"Topic\"`".to_owned()),
                );
                parse_root_rest(p, tok.span);
            }
            _ if common(p, &tok, first_import) => saw_any = true,
            kind => {
                saw_any = true;
                p.error(
                    codes::MIND_EXPECTED_LABEL,
                    tok.span,
                    format!("expected a quoted label, found {}", describe(kind)),
                    Some("write a child node as `\"Label\"`, optionally followed by `#id`, `.class` and a block".to_owned()),
                );
                recover(p, true);
            }
        }
        if p.pos == before && !p.at_eof() {
            p.bump();
        }
    }
    (out, saw_any)
}
