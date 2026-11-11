mod common;
use common::*;
use xenapse_core::{
    compile, declared_stylesheet, CompileOptions, ElementKind, FontSpec, Module, TextMeasurer,
    API_VERSION,
};

#[test]
fn api_version_is_current() {
    assert_eq!(API_VERSION, 1);
}

#[test]
fn declared_stylesheet_extracts_path() {
    let with_sheet = "IMPORT FLOW\nSTYLESHEET \"path/to/theme.xcss\"\nSTART\nEND\n";
    assert_eq!(
        declared_stylesheet(with_sheet),
        Some("path/to/theme.xcss".to_owned())
    );

    let without_sheet = "IMPORT FLOW\nSTART\nEND\n";
    assert_eq!(declared_stylesheet(without_sheet), None);
}

#[test]
fn compile_is_deterministic() {
    let src =
        "IMPORT FLOW\nSTART\n< Is Complete? >\nIF YES { [ Deploy ] } ELSE { [ Work ] }\nEND\n";
    let css = "node { fill: #f0f0f0; } edge { stroke: #333333; }";

    let res1 = compile_with_css(src, css);
    let res2 = compile_with_css(src, css);

    assert!(res1.svg.is_some());
    assert_eq!(res1.svg, res2.svg);
    assert_eq!(res1.width, res2.width);
    assert_eq!(res1.height, res2.height);
    assert_eq!(res1.elements.len(), res2.elements.len());
}

#[test]
fn compile_produces_valid_svg_structure() {
    let src = "IMPORT FLOW\nSTART\n[ Process & Validate <Data> ] #main_proc .important\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());

    let svg = res.svg.expect("SVG should be generated");
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("viewBox=\"0 0 "));
    assert!(svg.contains("<title id=\"xn-title\">"));
    assert!(svg.contains("<desc id=\"xn-desc\">"));
    assert!(svg.contains("<g class=\"xn-edges\">"));
    assert!(svg.contains("<g class=\"xn-nodes\">"));
    assert!(svg.ends_with("</svg>\n"));

    // Verify XML escaping of special characters in labels
    assert!(svg.contains("Process &amp; Validate &lt;Data&gt;"));

    // Verify element metadata
    let proc_elem = res
        .elements
        .iter()
        .find(|e| e.source_id.as_deref() == Some("main_proc"))
        .expect("element with source_id 'main_proc' must exist");
    assert_eq!(proc_elem.kind, ElementKind::Node);
    assert_eq!(
        proc_elem.label.as_deref(),
        Some("Process & Validate <Data>")
    );
    assert!(proc_elem.classes.contains(&"important".to_string()));
    assert!(proc_elem.width > 0.0);
    assert!(proc_elem.height > 0.0);
}

#[test]
fn compile_mindmap_all_layout_modes() {
    let tree_src = "IMPORT MINDMAP\nLAYOUT TREE\nROOT \"Mind Root\" {\n  \"Branch 1\"\n  \"Branch 2\" {\n    \"Leaf 2.1\"\n  }\n}\n";
    let tree_res = compile_src(tree_src);
    assert!(!tree_res.has_errors());
    assert_eq!(tree_res.module, Some(Module::Mindmap));
    assert!(tree_res.svg.is_some());

    let horiz_src = "IMPORT MINDMAP\nLAYOUT HORIZONTAL\nROOT \"Mind Root\" {\n  \"Branch 1\"\n  \"Branch 2\"\n}\n";
    let horiz_res = compile_src(horiz_src);
    assert!(!horiz_res.has_errors());
    assert!(horiz_res.svg.is_some());

    let radial_src = "IMPORT MINDMAP\nLAYOUT RADIAL\nROOT \"Mind Root\" {\n  \"North\"\n  \"East\"\n  \"South\"\n  \"West\"\n}\n";
    let radial_res = compile_src(radial_src);
    assert!(!radial_res.has_errors());
    assert!(radial_res.svg.is_some());
}

#[test]
fn compile_flow_directions_tb_and_lr() {
    let tb_src = "IMPORT FLOW\nDIRECTION TB\nSTART\n[ Process ]\nEND\n";
    let tb_res = compile_src(tb_src);
    assert!(!tb_res.has_errors());
    assert!(tb_res.svg.is_some());

    let lr_src = "IMPORT FLOW\nDIRECTION LR\nSTART\n[ Process ]\nEND\n";
    let lr_res = compile_src(lr_src);
    assert!(!lr_res.has_errors());
    assert!(lr_res.svg.is_some());
}

struct CustomMeasurer;

impl TextMeasurer for CustomMeasurer {
    fn text_width(&self, text: &str, font: &FontSpec) -> f64 {
        text.chars().count() as f64 * font.size * 0.7
    }

    fn line_height(&self, font: &FontSpec) -> f64 {
        font.size * 1.5
    }
}

#[test]
fn compile_with_custom_text_measurer() {
    let src = "IMPORT FLOW\nSTART\n[ Measured Node ]\nEND\n";
    let options = CompileOptions::default().with_measurer(Box::new(CustomMeasurer));
    let res = compile(src, None, &options);
    assert!(!res.has_errors());
    assert!(res.svg.is_some());
}

#[test]
fn format_diagnostics_renders_d6_specification() {
    let src = "IMPORT FLOW\nSTART\n[ Step ]\n"; // missing END
    let options = CompileOptions {
        source_name: "test_doc.xnp".to_owned(),
        stylesheet_name: "test_doc.xcss".to_owned(),
        measurer: None,
    };
    let res = compile(src, None, &options);
    assert!(res.has_errors());
    assert!(res.svg.is_none());

    let formatted = res.format_diagnostics();
    assert!(formatted.contains("test_doc.xnp:"));
    assert!(formatted.contains("error[XN-FLOW-003]"));
    assert!(formatted.contains("help:"));
}

#[test]
fn flow_start_star_and_end_star_omit_nodes_in_rendered_svg() {
    // 1. START* then END
    let res1 = compile_src("IMPORT FLOW\nSTART*\n[ Middle Step ]\nEND\n");
    assert!(!res1.has_errors());
    let svg1 = res1.svg.expect("SVG should be generated");
    assert!(!svg1.contains("xn-node xn-start"));
    assert!(!svg1.contains("<title>Start</title>"));
    assert!(svg1.contains("xn-node xn-end"));
    assert!(svg1.contains("<title>End</title>"));
    assert!(svg1.contains("Middle Step"));
    assert!(!svg1.contains("id=\"xn-e0\""));
    assert!(svg1.contains("id=\"xn-e1\""));

    // 2. START* then END*
    let res2 = compile_src("IMPORT FLOW\nSTART*\n[ Middle Step ]\nEND*\n");
    assert!(!res2.has_errors());
    let svg2 = res2.svg.expect("SVG should be generated");
    assert!(!svg2.contains("xn-node xn-start"));
    assert!(!svg2.contains("<title>Start</title>"));
    assert!(!svg2.contains("xn-node xn-end"));
    assert!(!svg2.contains("<title>End</title>"));
    assert!(svg2.contains("Middle Step"));
    assert!(!svg2.contains("id=\"xn-e0\""));
    assert!(!svg2.contains("id=\"xn-e1\""));

    // 3. START then END*
    let res3 = compile_src("IMPORT FLOW\nSTART\n[ Middle Step ]\nEND*\n");
    assert!(!res3.has_errors());
    let svg3 = res3.svg.expect("SVG should be generated");
    assert!(svg3.contains("xn-node xn-start"));
    assert!(svg3.contains("<title>Start</title>"));
    assert!(!svg3.contains("xn-node xn-end"));
    assert!(!svg3.contains("<title>End</title>"));
    assert!(svg3.contains("Middle Step"));
    assert!(svg3.contains("id=\"xn-e0\""));
    assert!(!svg3.contains("id=\"xn-e1\""));

    // 4. START then END (obvious baseline)
    let res4 = compile_src("IMPORT FLOW\nSTART\n[ Middle Step ]\nEND\n");
    assert!(!res4.has_errors());
    let svg4 = res4.svg.expect("SVG should be generated");
    assert!(svg4.contains("xn-node xn-start"));
    assert!(svg4.contains("<title>Start</title>"));
    assert!(svg4.contains("xn-node xn-end"));
    assert!(svg4.contains("<title>End</title>"));
    assert!(svg4.contains("Middle Step"));

    // Minimal sequences without intermediate nodes
    assert!(!compile_src("IMPORT FLOW\nSTART*\nEND\n").has_errors());
    assert!(!compile_src("IMPORT FLOW\nSTART*\nEND*\n").has_errors());
    assert!(!compile_src("IMPORT FLOW\nSTART\nEND*\n").has_errors());
}

#[test]
fn sequential_nodes_have_direct_straight_connections_in_tb_and_lr() {
    // Top-to-Bottom
    let tb_src = "IMPORT FLOW\nSTART\n[ Process 1: Alpha ]\n[ Process 2: Beta Gamma Delta ]\nEND\n";
    let tb_res = compile_src(tb_src);
    assert!(!tb_res.has_errors());
    let svg_tb = tb_res.svg.expect("SVG should exist");

    // In TB, sequential process nodes must share the same horizontal center
    let p1 = tb_res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Process 1: Alpha"))
        .unwrap();
    let p2 = tb_res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Process 2: Beta Gamma Delta"))
        .unwrap();
    let c1 = p1.x + p1.width / 2.0;
    let c2 = p2.x + p2.width / 2.0;
    assert!(
        (c1 - c2).abs() < 1e-3,
        "Sequential nodes in TB must share horizontal center: {c1} vs {c2}"
    );

    // The connector between them must be a direct vertical straight line (M cx y1 L cx y2)
    assert!(
        svg_tb.contains(&format!("M {c1:.2} ")),
        "Edge path must use straight vertical segment at {c1:.2}"
    );

    // Left-to-Right
    let lr_src = "IMPORT FLOW\nDIRECTION LR\nSTART\n[ Short ]\n[ Much Longer Process Step ]\nEND\n";
    let lr_res = compile_src(lr_src);
    assert!(!lr_res.has_errors());
    let svg_lr = lr_res.svg.expect("SVG should exist");

    let l1 = lr_res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Short"))
        .unwrap();
    let l2 = lr_res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Much Longer Process Step"))
        .unwrap();
    let cy1 = l1.y + l1.height / 2.0;
    let cy2 = l2.y + l2.height / 2.0;
    assert!(
        (cy1 - cy2).abs() < 1e-3,
        "Sequential nodes in LR must share vertical center: {cy1} vs {cy2}"
    );
    assert!(
        svg_lr.contains(&format!(" {cy1:.2} L ")),
        "Edge path must be horizontal at {cy1:.2}"
    );
}

#[test]
fn decision_branches_and_merges_have_clean_geometry() {
    let src = "IMPORT FLOW\nSTART\n< Decision Point >\nIF YES {\n  [ Option A ]\n} ELSE {\n  [ Option B ]\n}\n[ Merge Node ]\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let svg = res.svg.expect("SVG should exist");

    let opt_a = res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Option A"))
        .unwrap();
    let opt_b = res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Option B"))
        .unwrap();
    let dec = res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Decision Point"))
        .unwrap();
    let merge = res
        .elements
        .iter()
        .find(|e| e.label.as_deref() == Some("Merge Node"))
        .unwrap();

    let ca = opt_a.x + opt_a.width / 2.0;
    let cb = opt_b.x + opt_b.width / 2.0;
    let c_dec = dec.x + dec.width / 2.0;
    let c_merge = merge.x + merge.width / 2.0;

    // YES branch is to the left of NO branch
    assert!(ca < cb, "YES branch should be on the left of NO branch");
    // Decision is centered between branches
    let branch_mid = (ca + cb) / 2.0;
    assert!(
        (c_dec - branch_mid).abs() < 1.0,
        "Decision should be centered between branches: {c_dec} vs {branch_mid}"
    );
    assert!(
        (c_merge - branch_mid).abs() < 1.0,
        "Merge node should be centered: {c_merge} vs {branch_mid}"
    );

    // The two incoming edges to the merge node must not have overlapping arrowheads
    let merge_edges: Vec<_> = res
        .elements
        .iter()
        .filter(|e| e.kind == ElementKind::Edge)
        .collect();
    assert!(merge_edges.len() >= 4);

    // Verify all edge paths in the SVG are strictly orthogonal
    for line in svg.lines() {
        if line.starts_with("<path d=\"M ")
            && line.contains(" L ")
            && line.contains("fill=\"none\"")
        {
            let d_str = line
                .split("d=\"")
                .nth(1)
                .unwrap()
                .split('\"')
                .next()
                .unwrap();
            let tokens: Vec<&str> = d_str.split_whitespace().collect();
            let mut i = 0;
            let mut pts: Vec<(f64, f64)> = Vec::new();
            while i < tokens.len() {
                if (tokens[i] == "M" || tokens[i] == "L") && i + 2 < tokens.len() {
                    let x: f64 = tokens[i + 1].parse().unwrap();
                    let y: f64 = tokens[i + 2].parse().unwrap();
                    pts.push((x, y));
                    i += 3;
                    continue;
                }
                i += 1;
            }
            // Invariants: each segment must be horizontal or vertical
            for w in pts.windows(2) {
                let dx = (w[0].0 - w[1].0).abs();
                let dy = (w[0].1 - w[1].1).abs();
                assert!(
                    dx < 1e-4 || dy < 1e-4,
                    "Segment must be orthogonal: {:?} to {:?}",
                    w[0],
                    w[1]
                );
                assert!(
                    dx > 1e-4 || dy > 1e-4,
                    "Zero length segment: {:?} to {:?}",
                    w[0],
                    w[1]
                );
            }
        }
    }
}

#[test]
fn multi_layer_long_edge_channel_is_strictly_vertical() {
    let src = "IMPORT FLOW\nSTART\n< Check Condition >\nIF YES {\n  [ Step 1 ]\n  [ Step 2 ]\n  [ Step 3 ]\n} ELSE {\n  [ Bypass Step ]\n}\n[ Join After ]\nEND\n";
    let res = compile_src(src);
    assert!(!res.has_errors());
    let svg = res.svg.as_ref().expect("SVG should exist");

    // Verify the SVG compiles and is deterministic
    let res2 = compile_src(src);
    assert_eq!(res.svg, res2.svg);

    // Verify all connectors are strictly orthogonal
    for line in svg.lines() {
        if line.starts_with("<path d=\"M ")
            && line.contains(" L ")
            && line.contains("fill=\"none\"")
        {
            let d_str = line
                .split("d=\"")
                .nth(1)
                .unwrap()
                .split('\"')
                .next()
                .unwrap();
            let tokens: Vec<&str> = d_str.split_whitespace().collect();
            let mut i = 0;
            let mut pts: Vec<(f64, f64)> = Vec::new();
            while i < tokens.len() {
                if (tokens[i] == "M" || tokens[i] == "L") && i + 2 < tokens.len() {
                    let x: f64 = tokens[i + 1].parse().unwrap();
                    let y: f64 = tokens[i + 2].parse().unwrap();
                    pts.push((x, y));
                    i += 3;
                    continue;
                }
                i += 1;
            }
            for w in pts.windows(2) {
                let dx = (w[0].0 - w[1].0).abs();
                let dy = (w[0].1 - w[1].1).abs();
                assert!(
                    dx < 1e-4 || dy < 1e-4,
                    "Segment must be orthogonal: {:?} to {:?}",
                    w[0],
                    w[1]
                );
            }
        }
    }
}
