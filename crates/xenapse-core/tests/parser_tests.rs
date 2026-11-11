mod common;
use common::*;
use xenapse_core::{Module, Severity};

#[test]
fn parser_accepts_minimal_flow_and_mindmap() {
    let flow_res = compile_src("IMPORT FLOW\nSTART\nEND\n");
    assert!(!flow_res.has_errors());
    assert_eq!(flow_res.module, Some(Module::Flow));

    let mind_res = compile_src("IMPORT MINDMAP\nROOT \"Central Topic\"\n");
    assert!(!mind_res.has_errors());
    assert_eq!(mind_res.module, Some(Module::Mindmap));
}

#[test]
fn parser_accepts_all_valid_directives() {
    let flow = "IMPORT FLOW\nSTYLESHEET \"theme.xcss\"\nDIRECTION LR\nSTART\nEND\n";
    let res_flow = compile_src(flow);
    assert!(!res_flow.has_errors());

    let mind = "IMPORT MINDMAP\nSTYLESHEET \"style.xcss\"\nLAYOUT TREE\nROOT \"Project\"\n";
    let res_mind = compile_src(mind);
    assert!(!res_mind.has_errors());
}

#[test]
fn parser_rejects_missing_import_with_xn_syn_001() {
    let src = "START\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-SYN-001", Severity::Error));
}

#[test]
fn parser_rejects_invalid_module_with_xn_syn_002() {
    let src = "IMPORT UNKNOWN\nSTART\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-SYN-002", Severity::Error));
}

#[test]
fn parser_rejects_duplicate_import_with_xn_syn_003() {
    let src = "IMPORT FLOW\nIMPORT FLOW\nSTART\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-SYN-003", Severity::Error));
}

#[test]
fn parser_rejects_directive_in_wrong_module_with_xn_syn_004() {
    let flow_with_layout = "IMPORT FLOW\nLAYOUT RADIAL\nSTART\nEND\n";
    let res1 = compile_src(flow_with_layout);
    assert!(has_diag(&res1.diagnostics, "XN-SYN-004", Severity::Error));

    let mind_with_dir = "IMPORT MINDMAP\nDIRECTION TB\nROOT \"Title\"\n";
    let res2 = compile_src(mind_with_dir);
    assert!(has_diag(&res2.diagnostics, "XN-SYN-004", Severity::Error));
}

#[test]
fn parser_rejects_duplicate_directive_with_xn_syn_005() {
    let src = "IMPORT FLOW\nDIRECTION TB\nDIRECTION LR\nSTART\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-005", Severity::Error));
}

#[test]
fn parser_rejects_invalid_directive_values_with_xn_syn_006() {
    let src1 = "IMPORT FLOW\nDIRECTION DIAGONAL\nSTART\nEND\n";
    let res1 = compile_src(src1);
    assert!(has_diag(&res1.diagnostics, "XN-SYN-006", Severity::Error));

    let src2 = "IMPORT FLOW\nSTYLESHEET 12345\nSTART\nEND\n";
    let res2 = compile_src(src2);
    assert!(has_diag(&res2.diagnostics, "XN-SYN-006", Severity::Error));
}

#[test]
fn parser_rejects_nesting_beyond_limit_with_xn_syn_008() {
    let mut nested = String::from("IMPORT FLOW\nSTART\n< D0 >\n");
    let depth = 135;
    for i in 0..depth {
        nested.push_str(&format!("IF YES {{\n< D{} >\n", i + 1));
    }
    nested.push_str("[ Leaf ]\n");
    for _ in 0..depth {
        nested.push_str("} ELSE { [ Fallback ] }\n");
    }
    nested.push_str("END\n");

    let res = compile_src(&nested);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-SYN-008", Severity::Error));
}

#[test]
fn parser_rejects_else_without_if_with_xn_syn_009() {
    let src = "IMPORT FLOW\nSTART\nELSE { [ Step ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-009", Severity::Error));
}

#[test]
fn parser_rejects_if_without_yes_with_xn_syn_010() {
    let src = "IMPORT FLOW\nSTART\n< Check >\nIF MAYBE { [ Step ] } ELSE { [ Other ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-010", Severity::Error));
}

#[test]
fn parser_rejects_if_no_with_xn_syn_011() {
    let src = "IMPORT FLOW\nSTART\n< Check >\nIF NO { [ Step ] } ELSE { [ Other ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-011", Severity::Error));
}

#[test]
fn parser_rejects_missing_else_with_xn_syn_012() {
    let src = "IMPORT FLOW\nSTART\n< Check >\nIF YES { [ Step ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-012", Severity::Error));
}

#[test]
fn parser_rejects_empty_block_with_xn_syn_013() {
    let src = "IMPORT FLOW\nSTART\n< Check >\nIF YES { } ELSE { [ Step ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-013", Severity::Error));
}

#[test]
fn parser_rejects_unclosed_block_with_xn_syn_014() {
    let src = "IMPORT FLOW\nSTART\n< Check >\nIF YES { [ Step ] ELSE { [ Other ] }\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-014", Severity::Error));
}

#[test]
fn parser_rejects_stray_rbrace_with_xn_syn_015() {
    let src = "IMPORT FLOW\nSTART\n}\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-015", Severity::Error));
}

#[test]
fn parser_rejects_cross_module_constructs_with_xn_syn_016() {
    let src_flow_root = "IMPORT FLOW\nSTART\nROOT \"Topic\"\nEND\n";
    let res1 = compile_src(src_flow_root);
    assert!(has_diag(&res1.diagnostics, "XN-SYN-016", Severity::Error));

    let src_flow_str = "IMPORT FLOW\nSTART\n[ Process ]\n\"Bare String\"\nEND\n";
    let res2 = compile_src(src_flow_str);
    assert!(has_diag(&res2.diagnostics, "XN-SYN-016", Severity::Error));
}

#[test]
fn parser_rejects_directive_in_body_with_xn_syn_017() {
    let src = "IMPORT FLOW\nSTART\nDIRECTION LR\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-017", Severity::Error));
}

#[test]
fn parser_rejects_id_after_class_with_xn_syn_019() {
    let src = "IMPORT FLOW\nSTART\n[ Process ] .my_class #my_id\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-019", Severity::Error));
}

#[test]
fn parser_rejects_duplicate_id_on_same_node_with_xn_syn_020() {
    let src = "IMPORT FLOW\nSTART\n[ Process ] #first #second\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-020", Severity::Error));
}

#[test]
fn parser_rejects_unexpected_label_on_start_end_with_xn_syn_021() {
    let src1 = "IMPORT FLOW\nSTART \"Custom Label\"\nEND\n";
    let res1 = compile_src(src1);
    assert!(has_diag(&res1.diagnostics, "XN-SYN-021", Severity::Error));

    let src2 = "IMPORT FLOW\nSTART\nEND \"Custom End\"\n";
    let res2 = compile_src(src2);
    assert!(has_diag(&res2.diagnostics, "XN-SYN-021", Severity::Error));
}

#[test]
fn parser_rejects_database_without_label_with_xn_syn_022() {
    let src = "IMPORT FLOW\nSTART\nDATABASE\nEND\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-SYN-022", Severity::Error));
}

#[test]
fn parser_rejects_mindmap_missing_root_with_xn_mind_001() {
    let src = "IMPORT MINDMAP\n\"Child\"\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-MIND-001", Severity::Error));
}

#[test]
fn parser_rejects_second_root_in_mindmap_with_xn_mind_002() {
    let src = "IMPORT MINDMAP\nROOT \"First\"\nROOT \"Second\"\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-MIND-002", Severity::Error));
}

#[test]
fn parser_rejects_extra_content_after_root_with_xn_mind_003() {
    let src = "IMPORT MINDMAP\nROOT \"Root\"\n\"Unnested Sibling\"\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-MIND-003", Severity::Error));
}

#[test]
fn parser_rejects_unquoted_label_in_mindmap_with_xn_mind_004() {
    let src = "IMPORT MINDMAP\nROOT UnquotedRoot\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-MIND-004", Severity::Error));
}

#[test]
fn parser_rejects_flow_construct_in_mindmap_with_xn_mind_005() {
    let src = "IMPORT MINDMAP\nROOT \"Title\" {\n  START\n}\n";
    let res = compile_src(src);
    assert!(has_diag(&res.diagnostics, "XN-MIND-005", Severity::Error));
}
