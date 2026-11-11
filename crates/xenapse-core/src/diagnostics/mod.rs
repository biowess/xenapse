pub mod codes;

use crate::source::{LineIndex, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Where a diagnostic's span points. `Source` sorts before `Stylesheet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    Source,
    Stylesheet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Related {
    pub span: Span,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub help: Option<String>,
    pub origin: Origin,
    pub span: Span,
    pub related: Option<Related>,
}

impl Diagnostic {
    fn new(
        severity: Severity,
        code: &'static str,
        origin: Origin,
        span: Span,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic {
            code,
            severity,
            message: message.into(),
            help: None,
            origin,
            span,
            related: None,
        }
    }

    pub fn error(
        code: &'static str,
        origin: Origin,
        span: Span,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::new(Severity::Error, code, origin, span, message)
    }

    pub fn warning(
        code: &'static str,
        origin: Origin,
        span: Span,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::new(Severity::Warning, code, origin, span, message)
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Diagnostic {
        self.help = Some(help.into());
        self
    }

    pub fn with_related(mut self, span: Span, label: impl Into<String>) -> Diagnostic {
        self.related = Some(Related {
            span,
            label: label.into(),
        });
        self
    }

    /// Renders the D6 format. Lines are joined with `\n`; there is no
    /// trailing newline. `index` must be built from the text `span` refers to.
    pub(crate) fn render(&self, file_name: &str, index: &LineIndex) -> String {
        let kind = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let pos = index.position(self.span.start);
        let mut out = format!(
            "{}:{}:{}: {}[{}]\n{}",
            file_name, pos.line, pos.column, kind, self.code, self.message
        );
        if let Some(help) = &self.help {
            out.push_str("\nhelp: ");
            out.push_str(help);
        }
        if let Some(rel) = &self.related {
            let p = index.position(rel.span.start);
            out.push_str(&format!(
                "\nnote: {} at {}:{}:{}",
                rel.label, file_name, p.line, p.column
            ));
        }
        out
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Diagnostics {
        Diagnostics::default()
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(other);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }

    /// Stable sort by origin (Source first), then span start.
    pub fn into_sorted_vec(self) -> Vec<Diagnostic> {
        let mut v = self.items;
        v.sort_by_key(|d| (d.origin, d.span.start));
        v
    }
}
