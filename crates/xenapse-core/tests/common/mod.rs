#![allow(unused, dead_code)]

use xenapse_core::{
    compile, declared_stylesheet, CompileOptions, CompileResult, Diagnostic, ElementInfo,
    ElementKind, FontSpec, Module, Severity, Span, TextMeasurer, API_VERSION,
};

pub fn compile_src(src: &str) -> CompileResult {
    compile(src, None, &CompileOptions::default())
}

pub fn compile_with_css(src: &str, css: &str) -> CompileResult {
    compile(src, Some(css), &CompileOptions::default())
}

pub fn has_diag(diags: &[Diagnostic], code: &str, severity: Severity) -> bool {
    diags
        .iter()
        .any(|d| d.code == code && d.severity == severity)
}

pub fn has_code(diags: &[Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}
