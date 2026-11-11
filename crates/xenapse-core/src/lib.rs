#![forbid(unsafe_code)]

mod api;
mod ast;
mod diagnostics;
mod geometry;
mod lang;
mod layout;
mod lexer;
mod model;
mod parser;
mod render;
mod source;
mod style;
mod validate;

pub use api::{compile, declared_stylesheet, CompileOptions, CompileResult, API_VERSION};
pub use diagnostics::{Diagnostic, Origin, Related, Severity};
pub use lang::Module;
pub use layout::{FontSpec, TextMeasurer};
pub use render::{ElementInfo, ElementKind};
pub use source::Span;
