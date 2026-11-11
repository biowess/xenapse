//! Recursive-descent parser infrastructure shared by header and body parsers.

mod flow;
mod header;
mod mindmap;

pub(crate) use flow::parse_flow_body;
pub(crate) use header::{parse_header, report_duplicate_import};
pub(crate) use mindmap::parse_mindmap_body;

use crate::ast::{Body, Document};
use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::lang::Module;
use crate::lexer::{lex, Keyword, Token, TokenKind};
use crate::source::Span;

/// Maximum block nesting depth (D3).
pub(crate) const MAX_DEPTH: usize = 128;

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Returned by `peek` if the token vector lacks a trailing `Eof`.
    eof: Token,
    diags: Diagnostics,
    depth: usize,
    depth_reported: bool,
}

pub(crate) fn keyword_text(k: Keyword) -> &'static str {
    match k {
        Keyword::Import => "IMPORT",
        Keyword::Flow => "FLOW",
        Keyword::Mindmap => "MINDMAP",
        Keyword::Stylesheet => "STYLESHEET",
        Keyword::Direction => "DIRECTION",
        Keyword::Layout => "LAYOUT",
        Keyword::Tb => "TB",
        Keyword::Lr => "LR",
        Keyword::Radial => "RADIAL",
        Keyword::Tree => "TREE",
        Keyword::Horizontal => "HORIZONTAL",
        Keyword::Root => "ROOT",
        Keyword::Start => "START",
        Keyword::StartStar => "START*",
        Keyword::End => "END",
        Keyword::EndStar => "END*",
        Keyword::If => "IF",
        Keyword::Yes => "YES",
        Keyword::Else => "ELSE",
        Keyword::Database => "DATABASE",
    }
}

/// Human-readable token description for diagnostics.
pub(crate) fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Keyword(k) => format!("`{}`", keyword_text(*k)),
        TokenKind::Id(s) => format!("`#{s}`"),
        TokenKind::Class(s) => format!("`.{s}`"),
        TokenKind::Str(_) => "a string".to_owned(),
        TokenKind::LBrace => "`{`".to_owned(),
        TokenKind::RBrace => "`}`".to_owned(),
        TokenKind::Comment => "a comment".to_owned(),
        TokenKind::Shape { .. } => "a FLOW shape".to_owned(),
        TokenKind::Word(w) => format!("`{w}`"),
        TokenKind::Eof => "end of input".to_owned(),
    }
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Parser {
        let end = tokens.last().map_or(0, |t| t.span.end);
        let mut p = Parser {
            tokens,
            pos: 0,
            eof: Token {
                kind: TokenKind::Eof,
                span: Span::new(end, end),
            },
            diags: Diagnostics::new(),
            depth: 0,
            depth_reported: false,
        };
        p.skip_comments();
        p
    }

    /// Span of the first `IMPORT` token (empty span at 0 if there is none).
    pub(crate) fn first_import_span(&self) -> Span {
        self.tokens
            .iter()
            .find(|t| t.kind == TokenKind::Keyword(Keyword::Import))
            .map_or(Span::new(0, 0), |t| t.span)
    }

    pub(crate) fn into_diagnostics(self) -> Diagnostics {
        self.diags
    }

    pub(crate) fn report(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    /// Emits an XN-SYN error with an optional help line.
    pub(crate) fn error(
        &mut self,
        code: &'static str,
        span: Span,
        msg: String,
        help: Option<String>,
    ) {
        let mut d = Diagnostic::error(code, Origin::Source, span, msg);
        if let Some(h) = help {
            d = d.with_help(h);
        }
        self.diags.push(d);
    }

    fn skip_comments(&mut self) {
        while let Some(t) = self.tokens.get(self.pos) {
            if t.kind == TokenKind::Comment {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// The current meaningful token (never a comment); `Eof` at the end.
    pub(crate) fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.eof)
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    pub(crate) fn at_keyword(&self, kw: Keyword) -> bool {
        self.peek().kind == TokenKind::Keyword(kw)
    }

    /// Consumes and returns the current token. At `Eof` it returns `Eof`
    /// without advancing.
    pub(crate) fn bump(&mut self) -> Token {
        let tok = self.peek().clone();
        if tok.kind != TokenKind::Eof && self.pos < self.tokens.len() {
            self.pos += 1;
            self.skip_comments();
        }
        tok
    }

    /// Enters a block. Always increments the depth, so every call must be
    /// paired with `leave_block()`. Returns `false` once the limit is
    /// exceeded; the first such overflow emits one diagnostic. Callers that
    /// get `false` must not recurse further.
    pub(crate) fn enter_block(&mut self) -> bool {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            if !self.depth_reported {
                self.depth_reported = true;
                let span = self.peek().span;
                self.error(
                    codes::SYN_NESTING_TOO_DEEP,
                    span,
                    format!("blocks are nested too deeply (limit is {MAX_DEPTH})"),
                    None,
                );
            }
            return false;
        }
        true
    }

    pub(crate) fn leave_block(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Skips tokens until `pred` matches the current token or `Eof` is
    /// reached. May skip nothing. Returns the number of tokens skipped.
    pub(crate) fn skip_until(&mut self, pred: impl Fn(&TokenKind) -> bool) -> usize {
        let mut n = 0;
        while !self.at_eof() && !pred(&self.peek().kind) {
            self.bump();
            n += 1;
        }
        n
    }
}

/// Result of [`parse`].
///
/// `document` is `Some` whenever a header was parsed, even with errors.
/// Callers must not validate or lower when `diagnostics.has_errors()`.
pub(crate) struct ParseResult {
    pub(crate) document: Option<Document>,
    pub(crate) module: Option<Module>,
    pub(crate) diagnostics: Diagnostics,
}

/// Runs the lexer, the header parser and the module's body parser, merging
/// lexer and parser diagnostics. Never panics, for any input.
pub(crate) fn parse(source: &str) -> ParseResult {
    let lexed = lex(source);
    let mut p = Parser::new(lexed.tokens);
    let document = parse_header(&mut p).map(|header| {
        let body = match header.module {
            Module::Flow => Body::Flow(parse_flow_body(&mut p)),
            Module::Mindmap => Body::Mindmap(parse_mindmap_body(&mut p)),
        };
        Document { header, body }
    });
    let module = document.as_ref().map(|d| d.header.module).or(lexed.module);
    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(p.into_diagnostics().into_vec());
    ParseResult {
        document,
        module,
        diagnostics,
    }
}
