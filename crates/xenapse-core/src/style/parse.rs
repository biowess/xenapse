//! The one XCSS parser (D7). Produces a raw stylesheet; it never validates
//! selector meaning, property names or values.

use super::lex::{err, is_ident_start, lex, Tok, Token};
use super::stylesheet::{
    Declaration, Rule, Selector, SelectorPart, SelectorPartKind, Stylesheet, ValueKind, ValuePart,
};
use crate::diagnostics::{codes, Diagnostic, Diagnostics};
use crate::source::Span;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    eof: Token,
    diags: Diagnostics,
}

/// Parses XCSS text into a raw stylesheet. Spans are byte offsets into `src`.
/// Rules with a bad selector and declarations with a bad value are dropped
/// (their errors are reported); parsing always continues.
pub(crate) fn parse_stylesheet(src: &str) -> (Stylesheet, Diagnostics) {
    let (tokens, diags) = lex(src);
    let eof = Token {
        kind: Tok::Eof,
        span: Span::new(src.len(), src.len()),
    };
    let mut p = Parser {
        tokens,
        pos: 0,
        eof,
        diags,
    };
    let mut rules = Vec::new();
    loop {
        let tok = p.peek().clone();
        match tok.kind {
            Tok::Eof => break,
            Tok::RBrace => {
                p.diags.push(err(
                    codes::CSS_STRAY_RBRACE,
                    tok.span,
                    "'}' without a matching '{'",
                ));
                p.bump();
            }
            _ => {
                if let Some(rule) = p.rule() {
                    rules.push(rule);
                }
            }
        }
    }
    (Stylesheet { rules }, p.diags)
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.eof)
    }

    fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn prev_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map_or(0, |t| t.span.end)
    }

    fn rule(&mut self) -> Option<Rule> {
        let (selector, bad) = self.selector();
        if bad {
            self.recover_rule();
            return None;
        }
        let open = self.peek().span;
        self.bump();
        let mut declarations = Vec::new();
        let end;
        loop {
            let tok = self.peek().clone();
            match tok.kind {
                Tok::RBrace => {
                    self.bump();
                    end = tok.span.end;
                    break;
                }
                Tok::Eof => {
                    self.diags.push(err(
                        codes::CSS_UNCLOSED_RULE,
                        open,
                        "missing '}' to close this rule",
                    ));
                    end = self.prev_end();
                    break;
                }
                Tok::Semi => self.bump(),
                Tok::Ident(name) => {
                    self.bump();
                    if let Some(d) = self.declaration(name, tok.span) {
                        declarations.push(d);
                    }
                }
                Tok::Invalid => self.recover_decl(),
                Tok::Punct('@') => {
                    self.diags.push(at_rule_error(tok.span));
                    self.recover_decl();
                }
                _ => {
                    self.diags.push(err(
                        codes::CSS_EXPECTED_PROPERTY,
                        tok.span,
                        "expected a property name",
                    ));
                    self.recover_decl();
                }
            }
        }
        let span = Span::new(selector.span.start, end);
        Some(Rule {
            selector,
            declarations,
            span,
        })
    }

    /// Parses up to (not including) `{`. On success the current token is `{`.
    fn selector(&mut self) -> (Selector, bool) {
        let mut parts: Vec<SelectorPart> = Vec::new();
        let mut bad = false;
        loop {
            let tok = self.peek().clone();
            let part = match &tok.kind {
                Tok::LBrace => {
                    if parts.is_empty() && !bad {
                        self.diags.push(err(
                            codes::CSS_EXPECTED_SELECTOR,
                            tok.span,
                            "expected a selector before '{'",
                        ));
                        bad = true;
                    }
                    break;
                }
                Tok::Semi | Tok::RBrace | Tok::Eof => {
                    if !bad {
                        let (code, msg) = if parts.is_empty() {
                            (codes::CSS_EXPECTED_SELECTOR, "expected a selector")
                        } else {
                            (
                                codes::CSS_EXPECTED_OPEN_BRACE,
                                "expected '{' after the selector",
                            )
                        };
                        self.diags.push(err(code, tok.span, msg));
                    }
                    bad = true;
                    break;
                }
                Tok::Ident(s) => Some((SelectorPartKind::Ident, s.clone())),
                Tok::Class(s) => Some((SelectorPartKind::Class, s.clone())),
                Tok::Hash(s) => {
                    if s.starts_with(is_ident_start) {
                        Some((SelectorPartKind::Id, s.clone()))
                    } else {
                        if !bad {
                            self.diags.push(err(
                                codes::CSS_EXPECTED_SELECTOR,
                                tok.span,
                                "an id must start with a letter or '_'",
                            ));
                        }
                        None
                    }
                }
                Tok::Invalid => None,
                other => {
                    if !bad {
                        self.diags.push(selector_error(other, tok.span));
                    }
                    None
                }
            };
            match part {
                Some((kind, text)) => {
                    let space_before = !parts.is_empty() && tok.span.start > self.prev_end();
                    parts.push(SelectorPart {
                        kind,
                        text,
                        span: tok.span,
                        space_before,
                    });
                }
                None => bad = true,
            }
            self.bump();
        }
        let span = match (parts.first(), parts.last()) {
            (Some(a), Some(b)) => Span::new(a.span.start, b.span.end),
            _ => self.peek().span,
        };
        (Selector { parts, span }, bad)
    }

    /// After a bad selector: skip the rule's block, or the terminating token.
    fn recover_rule(&mut self) {
        let tok = self.peek().clone();
        match tok.kind {
            Tok::LBrace => {
                self.bump();
                let mut depth = 1usize;
                loop {
                    let t = self.peek().clone();
                    match t.kind {
                        Tok::Eof => {
                            self.diags.push(err(
                                codes::CSS_UNCLOSED_RULE,
                                tok.span,
                                "missing '}' to close this rule",
                            ));
                            return;
                        }
                        Tok::LBrace => depth += 1,
                        Tok::RBrace => depth -= 1,
                        _ => {}
                    }
                    self.bump();
                    if depth == 0 {
                        return;
                    }
                }
            }
            Tok::Semi | Tok::RBrace => self.bump(),
            _ => {}
        }
    }

    /// Skips to just past the next `;`, or stops before the rule's `}` / EOF.
    /// Nested `{ }` are skipped as a unit.
    fn recover_decl(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.peek().kind {
                Tok::Eof => return,
                Tok::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                Tok::RBrace if depth == 0 => return,
                Tok::RBrace => depth -= 1,
                Tok::LBrace => depth += 1,
                _ => {}
            }
            self.bump();
        }
    }

    /// The property identifier has been consumed.
    fn declaration(&mut self, property: String, property_span: Span) -> Option<Declaration> {
        let colon = self.peek().clone();
        if colon.kind != Tok::Colon {
            if colon.kind != Tok::Invalid {
                self.diags.push(err(
                    codes::CSS_EXPECTED_COLON,
                    colon.span,
                    "expected ':' after the property name",
                ));
            }
            self.recover_decl();
            return None;
        }
        self.bump();
        let mut value: Vec<ValuePart> = Vec::new();
        let mut bad = false;
        loop {
            let tok = self.peek().clone();
            let kind = match tok.kind {
                Tok::Semi => {
                    self.bump();
                    break;
                }
                Tok::RBrace | Tok::Eof => break,
                Tok::Ident(s) => ValueKind::Ident(s),
                Tok::Number { value, unit } => ValueKind::Number { value, unit },
                Tok::Hash(s) => ValueKind::Hex(s),
                Tok::Str(s) => ValueKind::Str(s),
                Tok::Comma => ValueKind::Comma,
                Tok::Invalid => {
                    bad = true;
                    self.bump();
                    continue;
                }
                Tok::Colon => {
                    if !bad {
                        self.diags.push(
                            err(
                                codes::CSS_EXPECTED_SEMICOLON,
                                tok.span,
                                "expected ';' before the next declaration",
                            )
                            .with_help("a ':' cannot appear inside a value"),
                        );
                    }
                    self.recover_decl();
                    return None;
                }
                Tok::Punct('@') => {
                    if !bad {
                        self.diags.push(at_rule_error(tok.span));
                    }
                    self.recover_decl();
                    return None;
                }
                _ => {
                    if !bad {
                        self.diags.push(err(
                            codes::CSS_UNEXPECTED_TOKEN,
                            tok.span,
                            "unexpected token in a value",
                        ));
                    }
                    self.recover_decl();
                    return None;
                }
            };
            value.push(ValuePart {
                kind,
                span: tok.span,
            });
            self.bump();
        }
        if bad {
            return None;
        }
        let Some(last) = value.last() else {
            self.diags.push(err(
                codes::CSS_EXPECTED_VALUE,
                colon.span,
                "expected a value after ':'",
            ));
            return None;
        };
        let span = Span::new(property_span.start, last.span.end);
        Some(Declaration {
            property,
            property_span,
            value,
            span,
        })
    }
}

fn at_rule_error(span: Span) -> Diagnostic {
    err(
        codes::CSS_AT_RULE,
        span,
        "'@' rules are not supported in XCSS",
    )
}

fn selector_error(tok: &Tok, span: Span) -> Diagnostic {
    let msg = match tok {
        Tok::Punct('@') => return at_rule_error(span),
        Tok::Colon => "pseudo-classes are not supported",
        Tok::Punct('[') | Tok::Punct(']') => "attribute selectors are not supported",
        Tok::Punct('>') | Tok::Punct('+') | Tok::Punct('~') => "combinators are not supported",
        Tok::Comma => "selector lists are not supported; write one rule per selector",
        Tok::Punct('*') => "the universal selector is not supported",
        _ => {
            return err(
                codes::CSS_UNEXPECTED_TOKEN,
                span,
                "unexpected token in a selector",
            )
        }
    };
    err(codes::CSS_UNSUPPORTED_SELECTOR, span, msg)
}
