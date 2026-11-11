//! XCSS lexer. Malformed tokens are reported here and emitted as `Tok::Invalid`.

use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::source::Span;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Tok {
    Ident(String),
    Class(String),
    Hash(String),
    Number {
        value: f64,
        unit: Option<String>,
    },
    Str(String),
    Colon,
    Semi,
    Comma,
    LBrace,
    RBrace,
    /// One of `@ [ ] > + ~ *`; syntax the parser rejects with a specific message.
    Punct(char),
    /// A malformed token whose error the lexer already reported.
    Invalid,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Token {
    pub kind: Tok,
    pub span: Span,
}

pub(super) fn err(code: &'static str, span: Span, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, Origin::Stylesheet, span, msg)
}

pub(super) fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diags: Diagnostics,
}

pub(super) fn lex(src: &str) -> (Vec<Token>, Diagnostics) {
    let mut lx = Lexer {
        src,
        pos: 0,
        tokens: Vec::new(),
        diags: Diagnostics::new(),
    };
    lx.run();
    lx.tokens.push(Token {
        kind: Tok::Eof,
        span: Span::new(src.len(), src.len()),
    });
    (lx.tokens, lx.diags)
}

impl<'a> Lexer<'a> {
    fn rest(&self) -> &'a str {
        self.src.get(self.pos..).unwrap_or("")
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek2(&self) -> Option<char> {
        self.rest().chars().nth(1)
    }

    fn push(&mut self, kind: Tok, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn error(&mut self, code: &'static str, start: usize, msg: impl Into<String>) {
        self.diags.push(err(code, Span::new(start, self.pos), msg));
    }

    fn take_while(&mut self, f: impl Fn(char) -> bool) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.src.get(start..self.pos).unwrap_or("")
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            if c.is_whitespace() {
                self.pos += c.len_utf8();
                continue;
            }
            match c {
                '/' if self.rest().starts_with("/*") => self.comment(),
                '"' => self.string(),
                '{' | '}' | ':' | ';' | ',' => {
                    self.pos += 1;
                    let kind = match c {
                        '{' => Tok::LBrace,
                        '}' => Tok::RBrace,
                        ':' => Tok::Colon,
                        ';' => Tok::Semi,
                        _ => Tok::Comma,
                    };
                    self.push(kind, start);
                }
                '#' => {
                    self.pos += 1;
                    let name = self.take_while(is_ident_continue);
                    if name.is_empty() {
                        self.error(
                            codes::CSS_UNKNOWN_CHAR,
                            start,
                            "'#' must be followed by a name",
                        );
                        self.push(Tok::Invalid, start);
                    } else {
                        self.push(Tok::Hash(name.to_owned()), start);
                    }
                }
                '.' if self.peek2().is_some_and(|n| n.is_ascii_digit()) => self.number(),
                '.' => {
                    self.pos += 1;
                    if self.peek().is_some_and(is_ident_start) {
                        let name = self.take_while(is_ident_continue);
                        self.push(Tok::Class(name.to_owned()), start);
                    } else {
                        self.error(
                            codes::CSS_UNKNOWN_CHAR,
                            start,
                            "'.' must be followed by a class name",
                        );
                        self.push(Tok::Invalid, start);
                    }
                }
                '+' | '-' | '0'..='9' if self.starts_number() => self.number(),
                '+' | '@' | '[' | ']' | '>' | '~' | '*' => {
                    self.pos += 1;
                    self.push(Tok::Punct(c), start);
                }
                _ if is_ident_start(c) => {
                    let name = self.take_while(is_ident_continue);
                    self.push(Tok::Ident(name.to_owned()), start);
                }
                _ => {
                    self.pos += c.len_utf8();
                    self.error(
                        codes::CSS_UNKNOWN_CHAR,
                        start,
                        format!("unknown character '{}'", c),
                    );
                    self.push(Tok::Invalid, start);
                }
            }
        }
    }

    fn starts_number(&self) -> bool {
        let mut it = self.rest().chars();
        let mut c = it.next();
        if matches!(c, Some('+') | Some('-')) {
            c = it.next();
        }
        match c {
            Some(d) if d.is_ascii_digit() => true,
            Some('.') => it.next().is_some_and(|d| d.is_ascii_digit()),
            _ => false,
        }
    }

    fn comment(&mut self) {
        let start = self.pos;
        match self.rest().get(2..).and_then(|r| r.find("*/")) {
            Some(i) => self.pos += 2 + i + 2,
            None => {
                self.pos = self.src.len();
                self.error(
                    codes::CSS_UNTERMINATED_COMMENT,
                    start,
                    "unterminated comment",
                );
            }
        }
    }

    fn number(&mut self) {
        let start = self.pos;
        if matches!(self.peek(), Some('+') | Some('-')) {
            self.pos += 1;
        }
        self.take_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') && self.peek2().is_some_and(|n| n.is_ascii_digit()) {
            self.pos += 1;
            self.take_while(|c| c.is_ascii_digit());
        }
        let text = self.src.get(start..self.pos).unwrap_or("");
        let parsed = text.parse::<f64>();
        let unit = if self.peek() == Some('%') {
            self.pos += 1;
            "%"
        } else {
            self.take_while(|c| c.is_ascii_alphabetic())
        };
        match parsed {
            Ok(v) if v.is_finite() => {
                let value = if v == 0.0 { 0.0 } else { v };
                let unit = if unit.is_empty() {
                    None
                } else {
                    Some(unit.to_owned())
                };
                self.push(Tok::Number { value, unit }, start);
            }
            _ => {
                self.error(
                    codes::CSS_INVALID_NUMBER,
                    start,
                    "number is not a finite value",
                );
                self.push(Tok::Invalid, start);
            }
        }
    }

    /// Same four escapes as D2. Any error makes the token `Invalid`.
    fn string(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let mut value = String::new();
        let mut ok = true;
        loop {
            let Some(c) = self.peek() else {
                self.error(codes::CSS_UNTERMINATED_STRING, start, "unterminated string");
                ok = false;
                break;
            };
            match c {
                '"' => {
                    self.pos += 1;
                    break;
                }
                '\n' => {
                    self.error(
                        codes::CSS_NEWLINE_IN_STRING,
                        start,
                        "a string cannot contain a raw newline",
                    );
                    ok = false;
                    break;
                }
                '\\' => {
                    let esc = self.pos;
                    self.pos += 1;
                    match self.peek() {
                        None | Some('\n') => {}
                        Some(e) => {
                            self.pos += e.len_utf8();
                            match e {
                                '\\' => value.push('\\'),
                                '"' => value.push('"'),
                                'n' => value.push('\n'),
                                't' => value.push('\t'),
                                _ => {
                                    ok = false;
                                    self.diags.push(
                                        err(
                                            codes::CSS_INVALID_ESCAPE,
                                            Span::new(esc, self.pos),
                                            format!("invalid escape sequence '\\{}'", e),
                                        )
                                        .with_help("valid escapes are \\\\ \\\" \\n \\t"),
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {
                    self.pos += c.len_utf8();
                    value.push(c);
                }
            }
        }
        self.push(if ok { Tok::Str(value) } else { Tok::Invalid }, start);
    }
}
