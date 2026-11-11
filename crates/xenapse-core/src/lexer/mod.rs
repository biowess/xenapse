//! Hand-written lexer for `.xnp` sources.

use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::lang::Module;
use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Import,
    Flow,
    Mindmap,
    Stylesheet,
    Direction,
    Layout,
    Tb,
    Lr,
    Radial,
    Tree,
    Horizontal,
    Root,
    Start,
    StartStar,
    End,
    EndStar,
    If,
    Yes,
    Else,
    Database,
}

impl Keyword {
    /// Exact, case-sensitive match against the D2 keyword list.
    pub fn from_word(word: &str) -> Option<Keyword> {
        Some(match word {
            "IMPORT" => Keyword::Import,
            "FLOW" => Keyword::Flow,
            "MINDMAP" => Keyword::Mindmap,
            "STYLESHEET" => Keyword::Stylesheet,
            "DIRECTION" => Keyword::Direction,
            "LAYOUT" => Keyword::Layout,
            "TB" => Keyword::Tb,
            "LR" => Keyword::Lr,
            "RADIAL" => Keyword::Radial,
            "TREE" => Keyword::Tree,
            "HORIZONTAL" => Keyword::Horizontal,
            "ROOT" => Keyword::Root,
            "START" => Keyword::Start,
            "START*" => Keyword::StartStar,
            "END" => Keyword::End,
            "END*" => Keyword::EndStar,
            "IF" => Keyword::If,
            "YES" => Keyword::Yes,
            "ELSE" => Keyword::Else,
            "DATABASE" => Keyword::Database,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Process,
    Event,
    Decision,
    InputOutput,
    Subprocess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    /// `#ident` (name without the `#`).
    Id(String),
    /// `.ident` (name without the `.`).
    Class(String),
    /// A string literal with escapes resolved.
    Str(String),
    LBrace,
    RBrace,
    Comment,
    /// A whole FLOW shape (opener through closer) with its label.
    Shape {
        kind: ShapeKind,
        label: String,
    },
    /// Any other bare word; the parser reports it.
    Word(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LexResult {
    /// Always ends with an `Eof` token.
    pub tokens: Vec<Token>,
    pub module: Option<Module>,
    pub diagnostics: Diagnostics,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Detect {
    /// No `IMPORT` seen yet.
    Idle,
    /// First `IMPORT` seen; the next non-comment token decides.
    Awaiting,
    Done,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diags: Diagnostics,
    module: Option<Module>,
    detect: Detect,
    last_unquoted_shape_on_line: Option<(char, char)>,
}

pub fn lex(source: &str) -> LexResult {
    let mut lx = Lexer {
        src: source,
        pos: 0,
        tokens: Vec::new(),
        diags: Diagnostics::new(),
        module: None,
        detect: Detect::Idle,
        last_unquoted_shape_on_line: None,
    };
    lx.run();
    let end = source.len();
    lx.tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(end, end),
    });
    LexResult {
        tokens: lx.tokens,
        module: lx.module,
        diagnostics: lx.diags,
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn is_word_start(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_word_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos..)?.chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn text(&self, start: usize, end: usize) -> &'a str {
        self.src.get(start..end).unwrap_or("")
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.observe(&kind);
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    /// Module detection (D2): first `IMPORT`, then FLOW or MINDMAP, skipping comments.
    fn observe(&mut self, kind: &TokenKind) {
        match self.detect {
            Detect::Idle => {
                if *kind == TokenKind::Keyword(Keyword::Import) {
                    self.detect = Detect::Awaiting;
                }
            }
            Detect::Awaiting => match kind {
                TokenKind::Comment => {}
                TokenKind::Keyword(Keyword::Flow) => {
                    self.module = Some(Module::Flow);
                    self.detect = Detect::Done;
                }
                TokenKind::Keyword(Keyword::Mindmap) => {
                    self.module = Some(Module::Mindmap);
                    self.detect = Detect::Done;
                }
                _ => self.detect = Detect::Done,
            },
            Detect::Done => {}
        }
    }

    fn error(&mut self, code: &'static str, start: usize, end: usize, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(
            code,
            Origin::Source,
            Span::new(start, end),
            msg,
        ));
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            if c.is_whitespace() {
                if c == '\n' {
                    self.last_unquoted_shape_on_line = None;
                }
                self.bump(c);
                continue;
            }
            match c {
                '{' => {
                    self.bump(c);
                    self.push(TokenKind::LBrace, start);
                }
                '}' => {
                    self.bump(c);
                    self.push(TokenKind::RBrace, start);
                }
                '"' => self.string(start),
                '#' => self.suffix(start, '#'),
                '.' => self.suffix(start, '.'),
                '/' if self.src.get(start..).is_some_and(|r| r.starts_with("//")) => {
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.bump(ch);
                    }
                    self.push(TokenKind::Comment, start);
                }
                '[' | '(' | '<' | '|' | '/' if self.module.is_some() => {
                    if self.module == Some(Module::Flow) {
                        self.shape(start, c);
                    } else {
                        self.bump(c);
                        self.error(
                            codes::LEX_FLOW_SHAPE_IN_MINDMAP,
                            start,
                            self.pos,
                            "FLOW shape syntax is not allowed in MINDMAP",
                        );
                    }
                }
                c if is_word_start(c) => {
                    while let Some(ch) = self.peek() {
                        if !is_word_continue(ch) {
                            break;
                        }
                        self.bump(ch);
                    }
                    if (self.text(start, self.pos) == "START"
                        || self.text(start, self.pos) == "END")
                        && self.peek() == Some('*')
                    {
                        self.bump('*');
                    }
                    let word = self.text(start, self.pos);
                    let kind = match Keyword::from_word(word) {
                        Some(k) => TokenKind::Keyword(k),
                        None => TokenKind::Word(word.to_owned()),
                    };
                    self.push(kind, start);
                }
                _ => {
                    self.bump(c);
                    let mut diag = Diagnostic::error(
                        codes::LEX_UNKNOWN_CHAR,
                        Origin::Source,
                        Span::new(start, self.pos),
                        format!("unexpected character {:?}", c),
                    );
                    if let Some((opener, closer)) = self.last_unquoted_shape_on_line.take() {
                        if c == closer {
                            diag = diag.with_help(format!(
                                "This label contains its own structural delimiter. Wrap the label in double quotes: {} \"...\" {}",
                                opener, closer
                            ));
                        }
                    }
                    self.diags.push(diag);
                }
            }
        }
    }

    /// `#ident` or `.ident`; `marker` has not been consumed yet.
    fn suffix(&mut self, start: usize, marker: char) {
        self.bump(marker);
        let name_start = self.pos;
        let valid = self.peek().is_some_and(is_ident_start);
        while let Some(ch) = self.peek() {
            if !is_ident_continue(ch) {
                break;
            }
            self.bump(ch);
        }
        if valid {
            let name = self.text(name_start, self.pos).to_owned();
            let kind = if marker == '#' {
                TokenKind::Id(name)
            } else {
                TokenKind::Class(name)
            };
            self.push(kind, start);
        } else {
            let what = if marker == '#' { "id" } else { "class" };
            self.diags.push(
                Diagnostic::error(
                    codes::LEX_INVALID_IDENT,
                    Origin::Source,
                    Span::new(start, self.pos),
                    format!("expected an identifier after '{}'", marker),
                )
                .with_help(format!("an {} name matches [A-Za-z_][A-Za-z0-9_-]*", what)),
            );
        }
    }

    fn skip_blanks(&mut self) {
        while let Some(c @ (' ' | '\t')) = self.peek() {
            self.bump(c);
        }
    }

    /// FLOW shape (D2); the opener has not been consumed yet.
    fn shape(&mut self, start: usize, opener: char) {
        let (kind, closer) = match opener {
            '[' => (ShapeKind::Process, ']'),
            '(' => (ShapeKind::Event, ')'),
            '<' => (ShapeKind::Decision, '>'),
            '|' => (ShapeKind::Subprocess, '|'),
            _ => (ShapeKind::InputOutput, '/'),
        };
        self.bump(opener);
        self.skip_blanks();
        let (label, is_quoted) = if self.peek() == Some('"') {
            let (value, ok) = self.string_body(self.pos);
            if !ok {
                // The string diagnostic is already reported; resume at the line break.
                self.last_unquoted_shape_on_line = None;
                return;
            }
            self.skip_blanks();
            if self.peek() != Some(closer) {
                self.unterminated_shape(start, closer);
                return;
            }
            (value, true)
        } else {
            let text_start = self.pos;
            loop {
                match self.peek() {
                    Some(c) if c == closer => break,
                    Some('\n') | None => {
                        self.unterminated_shape(start, closer);
                        return;
                    }
                    Some(c) => self.bump(c),
                }
            }
            (
                self.text(text_start, self.pos)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                false,
            )
        };
        self.bump(closer);
        if label.trim().is_empty() {
            self.last_unquoted_shape_on_line = None;
            self.diags.push(
                Diagnostic::error(
                    codes::LEX_EMPTY_SHAPE_LABEL,
                    Origin::Source,
                    Span::new(start, self.pos),
                    "a shape needs a non-empty label",
                )
                .with_help(format!(
                    "write some label text; use a quoted label if it must contain '{}'",
                    closer
                )),
            );
            return;
        }
        self.push(TokenKind::Shape { kind, label }, start);
        self.last_unquoted_shape_on_line = if is_quoted {
            None
        } else {
            Some((opener, closer))
        };
    }

    /// Reports an unterminated shape from `start` to the end of the line and
    /// leaves the cursor on the line break so lexing resumes on the next line.
    fn unterminated_shape(&mut self, start: usize, closer: char) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.bump(c);
        }
        let help = if let Some((opener, c)) = self.last_unquoted_shape_on_line.take() {
            if c == closer {
                format!(
                    "This label contains its own structural delimiter. Wrap the label in double quotes: {} \"...\" {}",
                    opener, closer
                )
            } else {
                format!(
                    "close the shape on the same line; if the label needs to contain '{}', use a quoted label",
                    closer
                )
            }
        } else {
            format!(
                "close the shape on the same line; if the label needs to contain '{}', use a quoted label",
                closer
            )
        };
        self.diags.push(
            Diagnostic::error(
                codes::LEX_UNTERMINATED_SHAPE,
                Origin::Source,
                Span::new(start, self.pos),
                format!(
                    "unterminated shape: missing closing '{}' on the same line",
                    closer
                ),
            )
            .with_help(help),
        );
    }

    /// String literal token; the opening quote has not been consumed yet.
    fn string(&mut self, start: usize) {
        let (value, _) = self.string_body(start);
        self.push(TokenKind::Str(value), start);
    }

    /// Lexes a string from its opening quote, reporting errors. Returns the
    /// value and whether the closing quote was found.
    fn string_body(&mut self, start: usize) -> (String, bool) {
        self.bump('"');
        let mut value = String::new();
        let mut closed = true;
        loop {
            let Some(c) = self.peek() else {
                self.error(
                    codes::LEX_UNTERMINATED_STRING,
                    start,
                    self.pos,
                    "unterminated string",
                );
                closed = false;
                break;
            };
            match c {
                '"' => {
                    self.bump(c);
                    break;
                }
                '\n' => {
                    self.error(
                        codes::LEX_NEWLINE_IN_STRING,
                        start,
                        self.pos,
                        "a string cannot contain a raw newline",
                    );
                    closed = false;
                    break;
                }
                '\\' => {
                    let esc_start = self.pos;
                    self.bump(c);
                    match self.peek() {
                        // The next iteration reports the end of input / newline.
                        None | Some('\n') => {}
                        Some(e) => {
                            self.bump(e);
                            match e {
                                '\\' => value.push('\\'),
                                '"' => value.push('"'),
                                'n' => value.push('\n'),
                                't' => value.push('\t'),
                                _ => self.diags.push(
                                    Diagnostic::error(
                                        codes::LEX_INVALID_ESCAPE,
                                        Origin::Source,
                                        Span::new(esc_start, self.pos),
                                        format!("invalid escape sequence '\\{}'", e),
                                    )
                                    .with_help("valid escapes are \\\\ \\\" \\n \\t"),
                                ),
                            }
                        }
                    }
                }
                _ => {
                    self.bump(c);
                    value.push(c);
                }
            }
        }
        (value, closed)
    }
}
