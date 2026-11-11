//! Header parsing: `IMPORT` and directives (D3).

use super::{describe, Parser};
use crate::ast::{Header, Spanned};
use crate::diagnostics::codes;
use crate::lang::{Direction, LayoutMode, Module};
use crate::lexer::{Keyword, TokenKind};
use crate::source::Span;

const IMPORT_HELP: &str = "write `IMPORT FLOW` or `IMPORT MINDMAP`";

fn direction_value(k: &TokenKind) -> Option<Direction> {
    match k {
        TokenKind::Keyword(Keyword::Tb) => Some(Direction::TopToBottom),
        TokenKind::Keyword(Keyword::Lr) => Some(Direction::LeftToRight),
        _ => None,
    }
}

fn layout_value(k: &TokenKind) -> Option<LayoutMode> {
    match k {
        TokenKind::Keyword(Keyword::Radial) => Some(LayoutMode::Radial),
        TokenKind::Keyword(Keyword::Tree) => Some(LayoutMode::Tree),
        TokenKind::Keyword(Keyword::Horizontal) => Some(LayoutMode::Horizontal),
        _ => None,
    }
}

/// Keywords that can only be the value of a directive or `IMPORT`.
fn is_value_keyword(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Keyword(
            Keyword::Tb
                | Keyword::Lr
                | Keyword::Radial
                | Keyword::Tree
                | Keyword::Horizontal
                | Keyword::Flow
                | Keyword::Mindmap
        )
    )
}

fn is_any_keyword(k: &TokenKind) -> bool {
    matches!(k, TokenKind::Keyword(_))
}

/// After a bad directive: drop a stray value keyword, then skip to the next
/// keyword. The directive keyword itself was already consumed.
fn recover_directive(p: &mut Parser) {
    if is_value_keyword(&p.peek().kind) {
        p.bump();
    }
    p.skip_until(is_any_keyword);
}

/// Parses one enum-valued directive argument, diagnosing and recovering on
/// failure.
fn directive_value<T>(
    p: &mut Parser,
    name: &str,
    allowed: &str,
    conv: fn(&TokenKind) -> Option<T>,
) -> Option<Spanned<T>> {
    if let Some(value) = conv(&p.peek().kind) {
        let span = p.bump().span;
        return Some(Spanned { value, span });
    }
    let tok = p.peek();
    let span = tok.span;
    let msg = format!("invalid value for {name}: found {}", describe(&tok.kind));
    p.error(
        codes::SYN_INVALID_DIRECTIVE_VALUE,
        span,
        msg,
        Some(format!("allowed values: {allowed}")),
    );
    recover_directive(p);
    None
}

/// Stores `new` in `slot`, or reports a duplicate (keeping the first).
fn store<T>(
    p: &mut Parser,
    name: &str,
    slot: &mut Option<(Span, Spanned<T>)>,
    kw_span: Span,
    new: Option<Spanned<T>>,
) {
    let Some(new) = new else { return };
    match slot {
        None => *slot = Some((kw_span, new)),
        Some((first_kw, _)) => {
            let first = *first_kw;
            let d = crate::diagnostics::Diagnostic::error(
                codes::SYN_DUPLICATE_DIRECTIVE,
                crate::diagnostics::Origin::Source,
                kw_span,
                format!("duplicate {name} directive"),
            )
            .with_help(format!(
                "each directive may appear at most once; remove this {name}"
            ))
            .with_related(first, format!("first {name} directive"));
            p.report(d);
        }
    }
}

/// Parses `IMPORT <module> directive*`. Returns `None` (with diagnostics)
/// when no valid `IMPORT` opens the document. Stops at the first token that
/// is not a directive keyword; the body parser continues from there.
pub(crate) fn parse_header(p: &mut Parser) -> Option<Header> {
    let first = p.peek().clone();
    if first.kind != TokenKind::Keyword(Keyword::Import) {
        p.error(
            codes::SYN_MISSING_IMPORT,
            first.span,
            "document must begin with IMPORT FLOW or IMPORT MINDMAP".to_owned(),
            Some(IMPORT_HELP.to_owned()),
        );
        return None;
    }
    p.bump();

    let module_tok = p.peek().clone();
    let module = match module_tok.kind {
        TokenKind::Keyword(Keyword::Flow) => Module::Flow,
        TokenKind::Keyword(Keyword::Mindmap) => Module::Mindmap,
        ref other => {
            p.error(
                codes::SYN_EXPECTED_MODULE,
                module_tok.span,
                format!(
                    "expected FLOW or MINDMAP after IMPORT, found {}",
                    describe(other)
                ),
                Some("allowed values: FLOW, MINDMAP".to_owned()),
            );
            return None;
        }
    };
    p.bump();
    let import_span = first.span.merge(module_tok.span);

    let mut stylesheet = None;
    let mut direction = None;
    let mut layout = None;

    loop {
        let kw_tok = p.peek().clone();
        let kw = match kw_tok.kind {
            TokenKind::Keyword(
                k @ (Keyword::Stylesheet | Keyword::Direction | Keyword::Layout),
            ) => k,
            _ => break,
        };
        p.bump();
        let ks = kw_tok.span;
        match kw {
            Keyword::Stylesheet => {
                let value = if let TokenKind::Str(s) = &p.peek().kind {
                    let value = s.clone();
                    let span = p.bump().span;
                    Some(Spanned { value, span })
                } else {
                    let tok = p.peek();
                    let span = tok.span;
                    let msg = format!(
                        "invalid value for STYLESHEET: expected a quoted path, found {}",
                        describe(&tok.kind)
                    );
                    p.error(
                        codes::SYN_INVALID_DIRECTIVE_VALUE,
                        span,
                        msg,
                        Some("write STYLESHEET \"path/to/file.xcss\"".to_owned()),
                    );
                    recover_directive(p);
                    None
                };
                store(p, "STYLESHEET", &mut stylesheet, ks, value);
            }
            Keyword::Direction => {
                if module != Module::Flow {
                    p.error(
                        codes::SYN_DIRECTIVE_WRONG_MODULE,
                        ks,
                        "DIRECTION is only allowed in FLOW documents".to_owned(),
                        Some(
                            "MINDMAP documents choose a layout with LAYOUT RADIAL|TREE|HORIZONTAL"
                                .to_owned(),
                        ),
                    );
                    recover_directive(p);
                    continue;
                }
                let value = directive_value(p, "DIRECTION", "TB, LR", direction_value);
                store(p, "DIRECTION", &mut direction, ks, value);
            }
            _ => {
                if module != Module::Mindmap {
                    p.error(
                        codes::SYN_DIRECTIVE_WRONG_MODULE,
                        ks,
                        "LAYOUT is only allowed in MINDMAP documents".to_owned(),
                        Some("FLOW documents choose a direction with DIRECTION TB|LR".to_owned()),
                    );
                    recover_directive(p);
                    continue;
                }
                let value = directive_value(p, "LAYOUT", "RADIAL, TREE, HORIZONTAL", layout_value);
                store(p, "LAYOUT", &mut layout, ks, value);
            }
        }
    }

    Some(Header {
        module,
        import_span,
        stylesheet: stylesheet.map(|(_, v)| v),
        direction: direction.map(|(_, v)| v),
        layout: layout.map(|(_, v)| v),
    })
}

/// For body parsers: if the current token is `IMPORT` at statement position,
/// reports a duplicate (related span at `first_import`), consumes it and a
/// following `FLOW`/`MINDMAP`, and returns `true`. Otherwise does nothing.
pub(crate) fn report_duplicate_import(p: &mut Parser, first_import: Span) -> bool {
    if !p.at_keyword(Keyword::Import) {
        return false;
    }
    let mut span = p.bump().span;
    if matches!(
        p.peek().kind,
        TokenKind::Keyword(Keyword::Flow | Keyword::Mindmap)
    ) {
        span = span.merge(p.bump().span);
    }
    let d = crate::diagnostics::Diagnostic::error(
        codes::SYN_DUPLICATE_IMPORT,
        crate::diagnostics::Origin::Source,
        span,
        "duplicate IMPORT: a document has exactly one IMPORT",
    )
    .with_help("remove this IMPORT; the module is set by the first line")
    .with_related(first_import, "first IMPORT");
    p.report(d);
    true
}
