//! XCSS: the single stylesheet lexer/parser, the raw (unvalidated) model, and
//! validation into typed rules.

mod lex;
mod model;
mod parse;
mod resolve;
mod stylesheet;
mod validate;

pub(crate) use model::ValidatedStylesheet;
pub(crate) use model::{Arrowhead, Dash, Paint, Rgba, TextAlign};
pub(crate) use parse::parse_stylesheet;
pub(crate) use resolve::resolve_styles;
pub(crate) use resolve::ResolvedStyles;
pub(crate) use resolve::{DiagramStyle, EdgeStyle, FontStyle, NodeStyle};
pub(crate) use validate::validate_stylesheet;
