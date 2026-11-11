//! Raw stylesheet model. Nothing here is validated: selector meaning,
//! property names and values are checked by a later stage. All spans are byte
//! offsets into the stylesheet text.

use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// `selector { declarations }`; the span runs from the selector start through
/// the closing `}` (or the end of the last token if the rule is unclosed).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Rule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Selector {
    pub parts: Vec<SelectorPart>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectorPartKind {
    Ident,
    Class,
    Id,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SelectorPart {
    pub kind: SelectorPartKind,
    /// The name without its `.` / `#` prefix; the span includes the prefix.
    pub text: String,
    pub span: Span,
    /// Whitespace or a comment separates this part from the previous one
    /// (`node text` is true, `edge.yes` is false). Always false for the first part.
    pub space_before: bool,
}

/// `property: value`; the span covers the property through the last value part
/// (not the `;`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Declaration {
    pub property: String,
    pub property_span: Span,
    pub value: Vec<ValuePart>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ValueKind {
    Ident(String),
    Number {
        value: f64,
        unit: Option<String>,
    },
    /// The characters after `#` (not checked to be hex digits here).
    Hex(String),
    /// A string with escapes resolved.
    Str(String),
    Comma,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValuePart {
    pub kind: ValueKind,
    pub span: Span,
}
