mod common;
use common::*;
use xenapse_core::Severity;

#[test]
fn style_accepts_valid_rules_and_diagram_properties() {
    let src = "IMPORT FLOW\nSTART\n[ Process Step ] #step .primary\nEND\n";
    let css = r#"
        /* Diagram layout */
        diagram {
            node-gap: 40px;
            rank-gap: 50px;
            branch-gap: 30px;
            diagram-padding: 20px;
        }

        node {
            fill: #f0f4f8;
            stroke: #102a43;
            stroke-width: 2px;
            color: #102a43;
            font-family: "IBM Plex Sans", Arial;
            font-size: 16px;
            font-weight: bold;
            text-align: center;
            padding: 14px;
            min-width: 80px;
            max-width: 200px;
            border-radius: 6px;
        }

        .primary {
            fill: #334e68;
            color: #ffffff;
        }

        #step text {
            color: #f7fafc;
        }

        edge {
            stroke: #627d98;
            stroke-width: 2px;
            stroke-dasharray: 4, 4;
            arrowhead: triangle;
        }
    "#;

    let res = compile_with_css(src, css);
    assert!(!res.has_errors());
    assert!(res.svg.is_some());
}

#[test]
fn style_rejects_unterminated_comment_with_xn_css_002() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = "node { fill: #fff; } /* unclosed comment";
    let res = compile_with_css(src, css);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-002", Severity::Error));
}

#[test]
fn style_rejects_unterminated_string_with_xn_css_003() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = "node { font-family: \"IBM Plex Sans; }";
    let res = compile_with_css(src, css);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-003", Severity::Error));
}

#[test]
fn style_rejects_at_rule_with_xn_css_007() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = "@import \"other.css\";";
    let res = compile_with_css(src, css);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-007", Severity::Error));
}

#[test]
fn style_rejects_unsupported_selector_syntax_with_xn_css_008() {
    let src = "IMPORT FLOW\nSTART\nEND\n";

    // Selector lists with comma
    let res1 = compile_with_css(src, "node, edge { stroke: #000; }");
    assert!(has_diag(&res1.diagnostics, "XN-CSS-008", Severity::Error));

    // Combinators
    let res2 = compile_with_css(src, "node > text { color: #000; }");
    assert!(has_diag(&res2.diagnostics, "XN-CSS-008", Severity::Error));

    // Attribute selectors
    let res3 = compile_with_css(src, "node[class=foo] { stroke: #000; }");
    assert!(has_diag(&res3.diagnostics, "XN-CSS-008", Severity::Error));

    // Universal selector
    let res4 = compile_with_css(src, "* { fill: none; }");
    assert!(has_diag(&res4.diagnostics, "XN-CSS-008", Severity::Error));
}

#[test]
fn style_warns_on_unknown_selector_with_xn_css_018() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = "sidebar { fill: #fff; }";
    let res = compile_with_css(src, css);
    // Unknown selector is a warning, rule is skipped
    assert!(!res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-018", Severity::Warning));
}

#[test]
fn style_warns_on_unsupported_property_with_xn_css_019() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = "node { display: flex; }";
    let res = compile_with_css(src, css);
    assert!(!res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-019", Severity::Warning));
}

#[test]
fn style_warns_on_property_target_mismatch_with_xn_css_020() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    // arrowhead is an edge property, not valid for node
    let css = "node { arrowhead: triangle; }";
    let res = compile_with_css(src, css);
    assert!(!res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-020", Severity::Warning));
}

#[test]
fn style_rejects_invalid_value_with_xn_css_021() {
    let src = "IMPORT FLOW\nSTART\nEND\n";

    // Invalid color
    let res1 = compile_with_css(src, "node { fill: notacolor; }");
    assert!(res1.has_errors());
    assert!(has_diag(&res1.diagnostics, "XN-CSS-021", Severity::Error));

    // Negative padding
    let res2 = compile_with_css(src, "node { padding: -10px; }");
    assert!(res2.has_errors());
    assert!(has_diag(&res2.diagnostics, "XN-CSS-021", Severity::Error));

    // Unsupported unit
    let res3 = compile_with_css(src, "node { font-size: 2em; }");
    assert!(res3.has_errors());
    assert!(has_diag(&res3.diagnostics, "XN-CSS-021", Severity::Error));
}

#[test]
fn style_warns_on_unmatched_selector_with_xn_css_022() {
    let src = "IMPORT FLOW\nSTART\nEND\n";
    let css = ".nonexistent_class { fill: #ff0000; }";
    let res = compile_with_css(src, css);
    assert!(!res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-CSS-022", Severity::Warning));
}

#[test]
fn style_applies_builtin_classes_and_inout_dual_match() {
    let src = "IMPORT FLOW\nSTART\n/ IO Step /\nEND\n";
    // decision D11 confirms: / / matches BOTH .input and .output
    let css = ".input { stroke: #111111; } .output { fill: #eeeeee; }";
    let res = compile_with_css(src, css);
    assert!(!res.has_errors());
    // Both rules matched, neither should trigger XN-CSS-022 unmatched warning!
    assert!(!has_code(&res.diagnostics, "XN-CSS-022"));
}
