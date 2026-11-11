//! SVG rendering (blueprint §9, decision D9) and element metadata (blueprint §7.4).

mod edge;
mod node;
pub(crate) mod svg;

use crate::geometry::Rect;
use crate::layout::Layout;
use crate::model::Diagram;
use crate::source::Span;
use crate::style::ResolvedStyles;
use svg::{round_num, SvgWriter};

/// Whether an [`ElementInfo`] describes a node or an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Node,
    Edge,
}

/// Host-facing metadata of one rendered node or edge. Coordinates are SVG user
/// units; an edge's box is the bounding box of its route.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementInfo {
    /// Id of the element's `<g>` in the SVG (`xn-n{i}` / `xn-e{i}`).
    pub svg_id: String,
    pub kind: ElementKind,
    /// Explicit `#id` from the source (nodes only).
    pub source_id: Option<String>,
    pub label: Option<String>,
    /// Classes as written in the SVG `class` attribute.
    pub classes: Vec<String>,
    /// Byte span of the element in the source.
    pub span: Span,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn info(
    svg_id: String,
    kind: ElementKind,
    source_id: Option<String>,
    label: Option<String>,
    classes: Vec<String>,
    span: Span,
    r: &Rect,
) -> ElementInfo {
    ElementInfo {
        svg_id,
        kind,
        source_id,
        label,
        classes,
        span,
        x: round_num(r.x),
        y: round_num(r.y),
        width: round_num(r.width),
        height: round_num(r.height),
    }
}

/// Renders the document. Returns the SVG, its (rounded) width and height, and
/// the node and edge metadata (nodes first, then edges, in id order).
pub(crate) fn render_svg(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    layout: &Layout,
) -> (String, f64, f64, Vec<ElementInfo>) {
    let width = round_num(layout.width.max(0.0));
    let height = round_num(layout.height.max(0.0));
    let (kind_name, kind_label) = match diagram {
        Diagram::Flow(_) => ("FLOW", "Xenapse FLOW diagram"),
        Diagram::Mindmap(_) => ("MINDMAP", "Xenapse MINDMAP diagram"),
    };

    let mut w = SvgWriter::new();
    w.begin("svg");
    w.attr("xmlns", "http://www.w3.org/2000/svg");
    w.attr(
        "viewBox",
        &format!("0 0 {} {}", svg::fmt_num(width), svg::fmt_num(height)),
    );
    w.attr_num("width", width);
    w.attr_num("height", height);
    w.attr("role", "img");
    w.attr("aria-labelledby", "xn-title");
    w.attr("aria-describedby", "xn-desc");
    w.open_end();
    w.newline();

    w.begin("title");
    w.attr("id", "xn-title");
    w.open_end();
    w.text(kind_label);
    w.close("title");
    w.newline();
    w.begin("desc");
    w.attr("id", "xn-desc");
    w.open_end();
    w.text(&format!(
        "{kind_name} diagram with {} nodes and {} edges.",
        layout.nodes.len(),
        layout.edges.len()
    ));
    w.close("desc");
    w.newline();

    let mut elements = Vec::new();

    w.begin("g");
    w.attr("class", "xn-edges");
    w.open_end();
    w.newline();
    for (i, el) in layout.edges.iter().enumerate() {
        if edge::edge_is_hidden(diagram, i) {
            continue;
        }
        edge::render_edge(&mut w, diagram, styles, i, el);
        if let Some((span, builtin, label)) = edge::edge_meta(diagram, i) {
            let mut classes = vec!["xn-edge".to_owned()];
            classes.extend(builtin.map(|b| format!("xn-{b}")));
            elements.push(info(
                format!("xn-e{i}"),
                ElementKind::Edge,
                None,
                label,
                classes,
                span,
                &edge::edge_bbox(el),
            ));
        }
    }
    w.close("g");
    w.newline();

    w.begin("g");
    w.attr("class", "xn-nodes");
    w.open_end();
    w.newline();
    let mut node_infos = Vec::new();
    for (i, nl) in layout.nodes.iter().enumerate() {
        if node::node_is_hidden(diagram, i) {
            continue;
        }
        node::render_node(&mut w, diagram, styles, i, nl);
        if let Some((source_id, label, span)) = node::node_meta(diagram, i) {
            node_infos.push(info(
                format!("xn-n{i}"),
                ElementKind::Node,
                source_id,
                Some(label),
                node::node_classes(diagram, i),
                span,
                &nl.rect,
            ));
        }
    }
    w.close("g");
    w.newline();

    w.close("svg");
    w.newline();
    node_infos.extend(elements);
    (w.finish(), width, height, node_infos)
}
