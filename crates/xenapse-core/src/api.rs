//! Public compile entry point (blueprint §7.4, decision D10). Runs the
//! pipeline: parse, validate, lower, resolve styles, layout, render SVG.

use crate::ast::Body;
use crate::diagnostics::{Diagnostic, Origin, Severity};
use crate::lang::Module;
use crate::layout::{layout, ApproxMeasurer, TextMeasurer};
use crate::lexer::lex;
use crate::model::{lower_flow::lower_flow, lower_mindmap::lower_mindmap, Diagram};
use crate::parser::{parse, parse_header, Parser};
use crate::render::{render_svg, ElementInfo};
use crate::source::LineIndex;
use crate::style::{parse_stylesheet, resolve_styles, validate_stylesheet, ValidatedStylesheet};
use crate::validate::{flow::validate_flow, mindmap::validate_mindmap};

/// Version of the public compile API.
pub const API_VERSION: u32 = 1;

/// Host-supplied options for [`compile`].
pub struct CompileOptions {
    /// File name used when rendering source diagnostics.
    pub source_name: String,
    /// File name used when rendering stylesheet diagnostics.
    pub stylesheet_name: String,
    /// Host text measurer; `None` uses the built-in deterministic approximation.
    pub measurer: Option<Box<dyn TextMeasurer>>,
}

impl CompileOptions {
    /// Sets the host text measurer used for layout.
    pub fn with_measurer(mut self, measurer: Box<dyn TextMeasurer>) -> CompileOptions {
        self.measurer = Some(measurer);
        self
    }
}

impl std::fmt::Debug for CompileOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileOptions")
            .field("source_name", &self.source_name)
            .field("stylesheet_name", &self.stylesheet_name)
            .field("measurer", &self.measurer.as_ref().map(|_| "<custom>"))
            .finish()
    }
}

impl Default for CompileOptions {
    fn default() -> CompileOptions {
        CompileOptions {
            source_name: "input.xnp".to_owned(),
            stylesheet_name: "style.xcss".to_owned(),
            measurer: None,
        }
    }
}

/// Outcome of [`compile`].
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub api_version: u32,
    pub module: Option<Module>,
    pub diagnostics: Vec<Diagnostic>,
    /// The SVG document; `None` when any error diagnostic exists.
    pub svg: Option<String>,
    /// Width of the SVG in user units (as written in the SVG).
    pub width: Option<f64>,
    /// Height of the SVG in user units (as written in the SVG).
    pub height: Option<f64>,
    /// Every node and edge of the diagram with span and bounding box; empty when no SVG.
    pub elements: Vec<ElementInfo>,
    source_name: String,
    stylesheet_name: String,
    source_index: LineIndex,
    stylesheet_index: Option<LineIndex>,
}

impl CompileResult {
    /// `true` if any diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Renders every diagnostic (D6 format), each followed by a newline.
    pub fn format_diagnostics(&self) -> String {
        let mut out = String::new();
        for d in &self.diagnostics {
            let rendered = match (d.origin, &self.stylesheet_index) {
                (Origin::Stylesheet, Some(index)) => d.render(&self.stylesheet_name, index),
                (Origin::Stylesheet, None) => d.render(&self.stylesheet_name, &self.source_index),
                (Origin::Source, _) => d.render(&self.source_name, &self.source_index),
            };
            out.push_str(&rendered);
            out.push('\n');
        }
        out
    }
}

/// Parses, validates and lowers `source`; parses and validates `stylesheet` if given.
/// Semantic stages run only when lexing and parsing produced no errors.
pub fn compile(source: &str, stylesheet: Option<&str>, options: &CompileOptions) -> CompileResult {
    let parsed = parse(source);
    let syntax_ok = !parsed.diagnostics.has_errors();
    let mut diags = parsed.diagnostics;
    let mut diagram = None;

    if syntax_ok {
        if let Some(doc) = &parsed.document {
            match &doc.body {
                Body::Flow(body) => {
                    let semantic = validate_flow(&doc.header, body);
                    if !semantic.has_errors() {
                        diagram = lower_flow(&doc.header, body).map(Diagram::Flow);
                    }
                    diags.extend(semantic.into_vec());
                }
                Body::Mindmap(root) => {
                    let semantic = validate_mindmap(root);
                    if !semantic.has_errors() {
                        diagram = Some(Diagram::Mindmap(lower_mindmap(&doc.header, root)));
                    }
                    diags.extend(semantic.into_vec());
                }
            }
        }
    }

    let mut validated = None;
    let stylesheet_index = stylesheet.map(|css| {
        let (raw, mut style_diags) = parse_stylesheet(css);
        let (typed, validation_diags) = validate_stylesheet(&raw);
        style_diags.extend(validation_diags.into_vec());
        validated = Some(typed);
        diags.extend(style_diags.into_vec().into_iter().map(|mut d| {
            d.origin = Origin::Stylesheet;
            d
        }));
        LineIndex::new(css)
    });

    let mut styles = None;
    if let Some(d) = &diagram {
        if !diags.has_errors() {
            let empty = ValidatedStylesheet::default();
            let (resolved, style_diags) = resolve_styles(d, validated.as_ref().unwrap_or(&empty));
            diags.extend(style_diags.into_vec());
            styles = Some(resolved);
        }
    }

    let (mut svg, mut width, mut height) = (None, None, None);
    let mut elements = Vec::new();
    if let (Some(d), Some(s)) = (&diagram, &styles) {
        if !diags.has_errors() {
            let approx = ApproxMeasurer;
            let measurer: &dyn TextMeasurer = options.measurer.as_deref().unwrap_or(&approx);
            let laid_out = layout(d, s, measurer);
            let (text, w, h, infos) = render_svg(d, s, &laid_out);
            elements = infos;
            svg = Some(text);
            width = Some(w);
            height = Some(h);
        }
    }

    CompileResult {
        api_version: API_VERSION,
        module: parsed.module,
        diagnostics: diags.into_sorted_vec(),
        svg,
        width,
        height,
        elements,
        source_name: options.source_name.clone(),
        stylesheet_name: options.stylesheet_name.clone(),
        source_index: LineIndex::new(source),
        stylesheet_index,
    }
}

/// The path from the `STYLESHEET` directive, if the header declares one.
/// Only the header is parsed; diagnostics are ignored.
pub fn declared_stylesheet(source: &str) -> Option<String> {
    let mut parser = Parser::new(lex(source).tokens);
    parse_header(&mut parser)?.stylesheet.map(|s| s.value)
}
