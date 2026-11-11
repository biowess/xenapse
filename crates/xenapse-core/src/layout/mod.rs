//! Layout foundation (blueprint §7.3, §8.3; decision D8): shape geometry kinds,
//! the layout output model, and node/edge-label box sizing. No placement
//! algorithms live here.

pub(crate) mod flow;
pub(crate) mod mindmap;
pub(crate) mod text;

use crate::geometry::{Point, Rect, Size};
use crate::model::flow::FlowNodeKind;
use crate::model::{Diagram, NodeId};
use crate::style::{EdgeStyle, NodeStyle, ResolvedStyles};
use text::wrap_text;
pub use text::{ApproxMeasurer, FontSpec, TextMeasurer};

/// Geometric shape drawn for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeGeom {
    Terminator,
    Rect,
    RoundedRect,
    Ellipse,
    Diamond,
    Parallelogram,
    DoubleRect,
    Cylinder,
}

impl ShapeGeom {
    pub(crate) fn from_flow(kind: FlowNodeKind) -> ShapeGeom {
        match kind {
            FlowNodeKind::Start | FlowNodeKind::End => ShapeGeom::Terminator,
            FlowNodeKind::Process => ShapeGeom::Rect,
            FlowNodeKind::Event => ShapeGeom::Ellipse,
            FlowNodeKind::Decision => ShapeGeom::Diamond,
            FlowNodeKind::InputOutput => ShapeGeom::Parallelogram,
            FlowNodeKind::Subprocess => ShapeGeom::DoubleRect,
            FlowNodeKind::Database => ShapeGeom::Cylinder,
        }
    }

    /// MINDMAP: root is an ellipse, every other node a rounded rectangle (D5).
    pub(crate) fn from_mindmap(is_root: bool) -> ShapeGeom {
        if is_root {
            ShapeGeom::Ellipse
        } else {
            ShapeGeom::RoundedRect
        }
    }

    /// `(kx, ky, ex, ey)`: shape box = `k * padded text box + e` per axis.
    pub(crate) fn inflation(self) -> (f64, f64, f64, f64) {
        match self {
            ShapeGeom::Rect | ShapeGeom::RoundedRect => (1.0, 1.0, 0.0, 0.0),
            ShapeGeom::Terminator => (1.0, 1.0, 16.0, 0.0),
            ShapeGeom::Diamond => (2.0, 2.0, 0.0, 0.0),
            ShapeGeom::Ellipse => (1.42, 1.42, 0.0, 0.0),
            ShapeGeom::Parallelogram => (1.0, 1.0, 24.0, 0.0),
            ShapeGeom::DoubleRect => (1.0, 1.0, 16.0, 0.0),
            ShapeGeom::Cylinder => (1.0, 1.0, 0.0, 20.0),
        }
    }
}

/// Lays out a diagram: dispatches to the FLOW or MINDMAP algorithms.
pub(crate) fn layout(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
) -> Layout {
    match diagram {
        Diagram::Flow(_) => flow::layout_flow(diagram, styles, measurer),
        Diagram::Mindmap(_) => mindmap::layout_mindmap(diagram, styles, measurer),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextAnchor {
    Middle,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeLayout {
    pub rect: Rect,
    pub lines: Vec<String>,
    pub line_height: f64,
    pub shape: ShapeGeom,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LabelLayout {
    pub text: String,
    /// Centre of the label box (anchor `Middle`; draw with a middle baseline).
    pub position: Point,
    pub anchor: TextAnchor,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeLayout {
    pub points: Vec<Point>,
    pub label: Option<LabelLayout>,
}

/// Final geometry. `nodes` / `edges` are indexed by `NodeId` / `EdgeId`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layout {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
}

impl From<&crate::style::FontStyle> for FontSpec {
    fn from(f: &crate::style::FontStyle) -> FontSpec {
        FontSpec {
            family: f.family.clone(),
            size: f.size,
            weight: f.weight,
        }
    }
}

/// Padding around an edge label's text box.
const LABEL_PAD: f64 = 4.0;

fn nonneg(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

/// Sizes every node. Boxes are placed at the origin; positions come from the
/// layout algorithms. The text is wrapped so the inflated box fits `max-width`;
/// the box is at least `min-width`. If a single glyph cannot fit, containing
/// the text wins over `max-width`; if `min-width > max-width`, `max-width` wins.
pub(crate) fn measure_nodes(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
) -> Vec<NodeLayout> {
    let items: Vec<(&str, ShapeGeom)> = match diagram {
        Diagram::Flow(f) => f
            .nodes
            .iter()
            .map(|n| (n.label.as_str(), ShapeGeom::from_flow(n.kind)))
            .collect(),
        Diagram::Mindmap(m) => m
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.label.as_str(), ShapeGeom::from_mindmap(i == 0)))
            .collect(),
    };
    items
        .into_iter()
        .enumerate()
        .map(|(i, (label, shape))| match styles.node(NodeId(i)) {
            Some(style) => measure_node(label, shape, style, measurer),
            None => NodeLayout {
                rect: Rect::default(),
                lines: Vec::new(),
                line_height: 0.0,
                shape,
            },
        })
        .collect()
}

fn measure_node(
    label: &str,
    shape: ShapeGeom,
    style: &NodeStyle,
    measurer: &dyn TextMeasurer,
) -> NodeLayout {
    let font = FontSpec::from(&style.text.layout.font);
    let pad = nonneg(style.layout.padding);
    let min_w = nonneg(style.layout.min_width);
    let max_w = nonneg(style.layout.max_width);
    let (kx, ky, ex, ey) = shape.inflation();

    let wrap_w = nonneg((max_w - ex) / kx - 2.0 * pad);
    let lines = wrap_text(label, wrap_w, measurer, &font);
    let line_height = nonneg(measurer.line_height(&font));

    let text_w = lines
        .iter()
        .map(|l| nonneg(measurer.text_width(l, &font)))
        .fold(0.0_f64, f64::max);
    let text_h = line_height * lines.len() as f64;

    let content_w = kx * (text_w + 2.0 * pad) + ex;
    let content_h = ky * (text_h + 2.0 * pad) + ey;
    let width = content_w.max(min_w).min(max_w).max(content_w);
    let height = content_h;

    NodeLayout {
        rect: Rect::new(0.0, 0.0, width, height),
        lines,
        line_height,
        shape,
    }
}

/// Size of an edge label's box: its text plus a small padding.
pub(crate) fn measure_edge_label(
    text: &str,
    style: &EdgeStyle,
    measurer: &dyn TextMeasurer,
) -> Size {
    let font = FontSpec {
        family: style.layout.font_family.clone(),
        size: style.layout.font_size,
        weight: 400,
    };
    let lines = wrap_text(text, f64::INFINITY, measurer, &font);
    let line_height = nonneg(measurer.line_height(&font));
    let w = lines
        .iter()
        .map(|l| nonneg(measurer.text_width(l, &font)))
        .fold(0.0_f64, f64::max);
    Size::new(
        w + 2.0 * LABEL_PAD,
        line_height * lines.len() as f64 + 2.0 * LABEL_PAD,
    )
}
