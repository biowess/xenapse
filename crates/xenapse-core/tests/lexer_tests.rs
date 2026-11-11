mod common;
use common::*;
use xenapse_core::Severity;

#[test]
fn lexer_recognizes_all_flow_shapes() {
    let src = "IMPORT FLOW\nSTART\n[ Process Step ]\n( Event Occurred )\n< Is Valid? >\nIF YES { [ OK ] } ELSE { [ Bad ] }\n/ Read Data /\n| Sub Process |\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    assert!(res.svg.is_some());
}

#[test]
fn lexer_collapses_whitespace_in_raw_labels() {
    let src = "IMPORT FLOW\nSTART\n[   hello    world \t test   ]\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let node = res.elements.iter().find(|e| e.svg_id == "xn-n1").unwrap();
    assert_eq!(node.label.as_deref(), Some("hello world test"));
}

#[test]
fn lexer_handles_quoted_labels_inside_shapes() {
    let src = "IMPORT FLOW\nSTART\n[ \"Process with [brackets] and // slashes\" ]\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let node = res.elements.iter().find(|e| e.svg_id == "xn-n1").unwrap();
    assert_eq!(
        node.label.as_deref(),
        Some("Process with [brackets] and // slashes")
    );
}

#[test]
fn lexer_distinguishes_comment_from_input_output_shape() {
    let src = "IMPORT FLOW\n// this is a comment\nSTART\n/ IO Node /\nEND // trailing comment\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let node = res.elements.iter().find(|e| e.svg_id == "xn-n1").unwrap();
    assert_eq!(node.label.as_deref(), Some("IO Node"));
}

#[test]
fn lexer_accepts_valid_string_escapes() {
    let src =
        "IMPORT MINDMAP\nROOT \"Title with \\\\ backslash, \\\" quotes, \\n newline, \\t tab\"\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let root = res.elements.iter().find(|e| e.svg_id == "xn-n0").unwrap();
    assert_eq!(
        root.label.as_deref(),
        Some("Title with \\ backslash, \" quotes, \n newline, \t tab")
    );
}

#[test]
fn lexer_rejects_unknown_characters_with_xn_lex_001() {
    let src = "IMPORT FLOW\n$START\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-001", Severity::Error));
}

#[test]
fn lexer_rejects_unterminated_string_with_xn_lex_002() {
    let src = "IMPORT MINDMAP\nROOT \"Unclosed string\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    // In Xenapse, a newline inside an unclosed string triggers either XN-LEX-003 or at EOF XN-LEX-002
    let src_eof = "IMPORT MINDMAP\nROOT \"Unclosed string at eof";
    let res_eof = compile_src(src_eof);
    assert!(has_diag(
        &res_eof.diagnostics,
        "XN-LEX-002",
        Severity::Error
    ));
}

#[test]
fn lexer_rejects_raw_newline_in_string_with_xn_lex_003() {
    let src = "IMPORT MINDMAP\nROOT \"Line 1\nLine 2\"\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-003", Severity::Error));
}

#[test]
fn lexer_rejects_invalid_escape_with_xn_lex_004() {
    let src = "IMPORT MINDMAP\nROOT \"Invalid \\q escape\"\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-004", Severity::Error));
}

#[test]
fn lexer_rejects_invalid_identifier_with_xn_lex_005() {
    let src = "IMPORT FLOW\nSTART #123invalid\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-005", Severity::Error));

    let src2 = "IMPORT FLOW\nSTART .123bad\nEND\n";
    let res2 = compile_src(src2);
    assert!(has_diag(&res2.diagnostics, "XN-LEX-005", Severity::Error));
}

#[test]
fn lexer_rejects_unterminated_shape_with_xn_lex_006() {
    let src = "IMPORT FLOW\nSTART\n[ Process without closer\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-006", Severity::Error));
}

#[test]
fn lexer_rejects_empty_shape_label_with_xn_lex_007() {
    let src = "IMPORT FLOW\nSTART\n[   ]\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-007", Severity::Error));
}

#[test]
fn lexer_rejects_flow_shape_in_mindmap_with_xn_lex_008() {
    let src = "IMPORT MINDMAP\nROOT \"My Map\" {\n  [ Flow Shape ]\n}\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-LEX-008", Severity::Error));
}
