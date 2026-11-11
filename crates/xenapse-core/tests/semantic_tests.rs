mod common;
use common::*;
use xenapse_core::Severity;

#[test]
fn semantic_rejects_duplicate_ids_across_document_with_xn_sem_001() {
    let src = "IMPORT FLOW\nSTART #dup\n[ Step ] #dup\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    let diag = res
        .diagnostics
        .iter()
        .find(|d| d.code == "XN-SEM-001")
        .expect("expected XN-SEM-001");
    assert_eq!(diag.severity, Severity::Error);
    assert!(diag.related.is_some());
}

#[test]
fn semantic_rejects_reserved_classes_with_xn_sem_002() {
    let reserved = [
        "process",
        "decision",
        "event",
        "input",
        "output",
        "database",
        "subprocess",
        "start",
        "end",
        "yes",
        "no",
    ];

    for class in reserved {
        let src = format!("IMPORT FLOW\nSTART .{class}\nEND\n");
        let res = compile_src(&src);
        assert!(
            has_diag(&res.diagnostics, "XN-SEM-002", Severity::Error),
            "expected XN-SEM-002 for reserved class .{}",
            class
        );
    }
}

#[test]
fn flow_rejects_missing_start_with_xn_flow_001() {
    let src = "IMPORT FLOW\n[ Step ]\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-001", Severity::Error));
}

#[test]
fn flow_rejects_misplaced_start_with_xn_flow_002() {
    let src1 = "IMPORT FLOW\n[ Step ]\nSTART\nEND\n";
    let res1 = compile_src(src1);
    assert!(has_diag(&res1.diagnostics, "XN-FLOW-002", Severity::Error));

    let src2 = "IMPORT FLOW\nSTART\nSTART\nEND\n";
    let res2 = compile_src(src2);
    assert!(has_diag(&res2.diagnostics, "XN-FLOW-002", Severity::Error));
}

#[test]
fn flow_rejects_missing_end_with_xn_flow_003() {
    let src = "IMPORT FLOW\nSTART\n[ Lonely Step ]\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-003", Severity::Error));
}

#[test]
fn flow_rejects_if_without_decision_with_xn_flow_004() {
    let src = "IMPORT FLOW\nSTART\n[ Process Step ]\nIF YES { [ OK ] } ELSE { [ No ] }\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-004", Severity::Error));
}

#[test]
fn flow_rejects_decision_without_if_with_xn_flow_005() {
    let src = "IMPORT FLOW\nSTART\n< Check Value >\n[ Next Step ]\nEND\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-005", Severity::Error));
}

#[test]
fn flow_rejects_unreachable_statement_after_end_with_xn_flow_006() {
    let src = "IMPORT FLOW\nSTART\nEND\n[ Ghost Step ]\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-006", Severity::Error));
}

#[test]
fn flow_rejects_unreachable_after_both_branches_terminate_with_xn_flow_006() {
    let src = "IMPORT FLOW\nSTART\n< Is Done? >\nIF YES { END } ELSE { END }\n[ Dead Code ]\n";
    let res = compile_src(src);
    assert!(res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-006", Severity::Error));
}

#[test]
fn flow_warns_on_dead_end_path_with_xn_flow_007() {
    let src = "IMPORT FLOW\nSTART\n< Keep Going? >\nIF YES { END } ELSE { [ Hanging Step ] }\n";
    let res = compile_src(src);
    // XN-FLOW-007 is a WARNING, not an error! SVG should still be produced!
    assert!(!res.has_errors());
    assert!(has_diag(&res.diagnostics, "XN-FLOW-007", Severity::Warning));
    assert!(res.svg.is_some());
}
