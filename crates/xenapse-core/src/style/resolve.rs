//! Style resolution (blueprint §6, decision D7): built-in default theme, then
//! every matching rule in ascending (specificity, source order). Layout-affecting
//! values and paint-only values live in separate types. No geometry is computed.

use super::model::{
    Arrowhead, Dash, Paint, Property, Rgba, Selector, TextAlign, TypedDeclaration,
    ValidatedStylesheet, Value,
};
use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::model::{Diagram, EdgeId, NodeId};

/// Font properties; all of them affect layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FontStyle {
    pub family: Vec<String>,
    pub size: f64,
    pub weight: u16,
}

/// Node values that must be known before geometry is computed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeLayoutStyle {
    pub font: FontStyle,
    pub padding: f64,
    pub min_width: f64,
    pub max_width: f64,
}

/// Node values that only affect painting.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodePaintStyle {
    pub fill: Paint,
    pub stroke: Paint,
    pub stroke_width: f64,
    pub color: Rgba,
    pub text_align: TextAlign,
    /// Only meaningful for Process, Subprocess and MINDMAP topics (D7).
    pub border_radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextLayoutStyle {
    pub font: FontStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextPaintStyle {
    pub color: Rgba,
    pub align: TextAlign,
}

/// Effective text style of a node: its node-level color/font/align values,
/// overridden by matching `text` rules.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextStyle {
    pub layout: TextLayoutStyle,
    pub paint: TextPaintStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeStyle {
    pub layout: NodeLayoutStyle,
    pub paint: NodePaintStyle,
    pub text: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeLayoutStyle {
    pub font_family: Vec<String>,
    pub font_size: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgePaintStyle {
    pub stroke: Paint,
    pub stroke_width: f64,
    pub dash: Dash,
    pub arrowhead: Arrowhead,
    /// Label color.
    pub color: Rgba,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeStyle {
    pub layout: EdgeLayoutStyle,
    pub paint: EdgePaintStyle,
}

/// The `diagram` selector only has layout properties; there is no paint part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DiagramLayoutStyle {
    pub node_gap: f64,
    pub rank_gap: f64,
    pub branch_gap: f64,
    pub diagram_padding: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DiagramStyle {
    pub layout: DiagramLayoutStyle,
}

/// Final styles, indexed by `NodeId` / `EdgeId` (positions in the model).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedStyles {
    pub nodes: Vec<NodeStyle>,
    pub edges: Vec<EdgeStyle>,
    pub diagram: DiagramStyle,
}

impl ResolvedStyles {
    pub(crate) fn node(&self, id: NodeId) -> Option<&NodeStyle> {
        self.nodes.get(id.0)
    }

    pub(crate) fn edge(&self, id: EdgeId) -> Option<&EdgeStyle> {
        self.edges.get(id.0)
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

fn default_family() -> Vec<String> {
    ["IBM Plex Sans", "Helvetica Neue", "Arial", "sans-serif"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

fn default_node_style() -> NodeStyle {
    let font = FontStyle {
        family: default_family(),
        size: 14.0,
        weight: 400,
    };
    let color = rgb(0x20, 0x29, 0x38);
    NodeStyle {
        layout: NodeLayoutStyle {
            font: font.clone(),
            padding: 12.0,
            min_width: 72.0,
            max_width: 220.0,
        },
        paint: NodePaintStyle {
            fill: Paint::Color(rgb(0xFF, 0xFF, 0xFF)),
            stroke: Paint::Color(rgb(0x34, 0x40, 0x54)),
            stroke_width: 1.5,
            color,
            text_align: TextAlign::Center,
            border_radius: 8.0,
        },
        text: TextStyle {
            layout: TextLayoutStyle { font },
            paint: TextPaintStyle {
                color,
                align: TextAlign::Center,
            },
        },
    }
}

fn default_edge_style() -> EdgeStyle {
    EdgeStyle {
        layout: EdgeLayoutStyle {
            font_family: default_family(),
            font_size: 12.0,
        },
        paint: EdgePaintStyle {
            stroke: Paint::Color(rgb(0x66, 0x70, 0x85)),
            stroke_width: 1.5,
            dash: Dash::None,
            arrowhead: Arrowhead::Triangle,
            color: rgb(0x47, 0x54, 0x67),
        },
    }
}

fn default_diagram_style() -> DiagramStyle {
    DiagramStyle {
        layout: DiagramLayoutStyle {
            node_gap: 32.0,
            rank_gap: 48.0,
            branch_gap: 40.0,
            diagram_padding: 24.0,
        },
    }
}

fn apply_node(s: &mut NodeStyle, d: &TypedDeclaration) {
    match (d.property, &d.value) {
        (Property::Fill, Value::Paint(p)) => s.paint.fill = *p,
        (Property::Stroke, Value::Paint(p)) => s.paint.stroke = *p,
        (Property::StrokeWidth, Value::Number(n)) => s.paint.stroke_width = *n,
        (Property::Color, Value::Color(c)) => s.paint.color = *c,
        (Property::TextAlign, Value::TextAlign(a)) => s.paint.text_align = *a,
        (Property::BorderRadius, Value::Length(n)) => s.paint.border_radius = *n,
        (Property::FontFamily, Value::FontFamily(f)) => s.layout.font.family = f.clone(),
        (Property::FontSize, Value::Length(n)) => s.layout.font.size = *n,
        (Property::FontWeight, Value::FontWeight(w)) => s.layout.font.weight = *w,
        (Property::Padding, Value::Length(n)) => s.layout.padding = *n,
        (Property::MinWidth, Value::Length(n)) => s.layout.min_width = *n,
        (Property::MaxWidth, Value::Length(n)) => s.layout.max_width = *n,
        _ => {}
    }
}

fn apply_text(t: &mut TextStyle, d: &TypedDeclaration) {
    match (d.property, &d.value) {
        (Property::Color, Value::Color(c)) => t.paint.color = *c,
        (Property::TextAlign, Value::TextAlign(a)) => t.paint.align = *a,
        (Property::FontFamily, Value::FontFamily(f)) => t.layout.font.family = f.clone(),
        (Property::FontSize, Value::Length(n)) => t.layout.font.size = *n,
        (Property::FontWeight, Value::FontWeight(w)) => t.layout.font.weight = *w,
        _ => {}
    }
}

fn apply_edge(s: &mut EdgeStyle, d: &TypedDeclaration) {
    match (d.property, &d.value) {
        (Property::Stroke, Value::Paint(p)) => s.paint.stroke = *p,
        (Property::StrokeWidth, Value::Number(n)) => s.paint.stroke_width = *n,
        (Property::StrokeDasharray, Value::Dash(v)) => s.paint.dash = v.clone(),
        (Property::Arrowhead, Value::Arrowhead(a)) => s.paint.arrowhead = *a,
        (Property::Color, Value::Color(c)) => s.paint.color = *c,
        (Property::FontFamily, Value::FontFamily(f)) => s.layout.font_family = f.clone(),
        (Property::FontSize, Value::Length(n)) => s.layout.font_size = *n,
        _ => {}
    }
}

fn apply_diagram(s: &mut DiagramStyle, d: &TypedDeclaration) {
    if let Value::Length(n) = d.value {
        match d.property {
            Property::NodeGap => s.layout.node_gap = n,
            Property::RankGap => s.layout.rank_gap = n,
            Property::BranchGap => s.layout.branch_gap = n,
            Property::DiagramPadding => s.layout.diagram_padding = n,
            _ => {}
        }
    }
}

/// What selectors can match on for one node.
struct NodeInfo<'a> {
    source_id: Option<&'a str>,
    /// User classes plus the kind's built-in classes.
    classes: Vec<&'a str>,
}

fn node_matches(info: &NodeInfo<'_>, class: Option<&str>, id: Option<&str>) -> bool {
    let class_ok = match class {
        Some(c) => info.classes.contains(&c),
        None => true,
    };
    let id_ok = match id {
        Some(i) => info.source_id == Some(i),
        None => true,
    };
    class_ok && id_ok
}

/// Resolves every element's style. Warns (XN-CSS, stylesheet origin) for each
/// rule whose selector matches no element.
pub(crate) fn resolve_styles(
    diagram: &Diagram,
    sheet: &ValidatedStylesheet,
) -> (ResolvedStyles, Diagnostics) {
    let (infos, edge_classes): (Vec<NodeInfo<'_>>, Vec<Option<&'static str>>) = match diagram {
        Diagram::Flow(f) => (
            f.nodes
                .iter()
                .map(|n| {
                    let mut classes: Vec<&str> = n.classes.iter().map(String::as_str).collect();
                    classes.extend(n.kind.builtin_classes().iter().copied());
                    NodeInfo {
                        source_id: n.source_id.as_deref(),
                        classes,
                    }
                })
                .collect(),
            f.edges.iter().map(|e| e.kind.builtin_class()).collect(),
        ),
        Diagram::Mindmap(m) => (
            m.nodes
                .iter()
                .map(|n| NodeInfo {
                    source_id: n.source_id.as_deref(),
                    classes: n.classes.iter().map(String::as_str).collect(),
                })
                .collect(),
            vec![None; m.edge_count()],
        ),
    };

    // Ascending (specificity, order): later entries override earlier ones.
    let mut sorted: Vec<usize> = (0..sheet.rules.len()).collect();
    sorted.sort_by_key(|&i| {
        sheet
            .rules
            .get(i)
            .map_or((0, 0), |r| (r.specificity, r.order))
    });

    let mut matched = vec![false; sheet.rules.len()];
    let mut diagram_style = default_diagram_style();
    let mut nodes = Vec::with_capacity(infos.len());
    let mut edges = Vec::with_capacity(edge_classes.len());

    for &i in &sorted {
        if let Some(r) = sheet.rules.get(i) {
            if matches!(r.selector, Selector::Diagram) {
                if let Some(m) = matched.get_mut(i) {
                    *m = true;
                }
                for d in &r.declarations {
                    apply_diagram(&mut diagram_style, d);
                }
            }
        }
    }

    for info in &infos {
        let mut style = default_node_style();
        // Node-level rules first; the text style is then derived from them.
        for &i in &sorted {
            let Some(r) = sheet.rules.get(i) else {
                continue;
            };
            if let Selector::Node {
                class,
                id,
                text: false,
            } = &r.selector
            {
                if node_matches(info, class.as_deref(), id.as_deref()) {
                    if let Some(m) = matched.get_mut(i) {
                        *m = true;
                    }
                    for d in &r.declarations {
                        apply_node(&mut style, d);
                    }
                }
            }
        }
        style.text = TextStyle {
            layout: TextLayoutStyle {
                font: style.layout.font.clone(),
            },
            paint: TextPaintStyle {
                color: style.paint.color,
                align: style.paint.text_align,
            },
        };
        for &i in &sorted {
            let Some(r) = sheet.rules.get(i) else {
                continue;
            };
            if let Selector::Node {
                class,
                id,
                text: true,
            } = &r.selector
            {
                if node_matches(info, class.as_deref(), id.as_deref()) {
                    if let Some(m) = matched.get_mut(i) {
                        *m = true;
                    }
                    for d in &r.declarations {
                        apply_text(&mut style.text, d);
                    }
                }
            }
        }
        nodes.push(style);
    }

    for edge_class in &edge_classes {
        let mut style = default_edge_style();
        for &i in &sorted {
            let Some(r) = sheet.rules.get(i) else {
                continue;
            };
            if let Selector::Edge { class } = &r.selector {
                let applies = match class.as_deref() {
                    Some(c) => *edge_class == Some(c),
                    None => true,
                };
                if applies {
                    if let Some(m) = matched.get_mut(i) {
                        *m = true;
                    }
                    for d in &r.declarations {
                        apply_edge(&mut style, d);
                    }
                }
            }
        }
        edges.push(style);
    }

    let mut diags = Diagnostics::new();
    for (r, was_matched) in sheet.rules.iter().zip(matched.iter()) {
        if *was_matched {
            continue;
        }
        let (what, help) = match r.selector {
            Selector::Edge { .. } => (
                "edges",
                "edge classes are `yes` and `no` (FLOW decision branches); MINDMAP edges have no classes",
            ),
            _ => (
                "nodes",
                "check the class or id; classes are the user classes plus the built-in kind classes (FLOW only)",
            ),
        };
        diags.push(
            Diagnostic::warning(
                codes::CSS_UNMATCHED_SELECTOR,
                Origin::Stylesheet,
                r.selector_span,
                format!(
                    "selector matches no {} in this diagram; rule has no effect",
                    what
                ),
            )
            .with_help(help),
        );
    }

    (
        ResolvedStyles {
            nodes,
            edges,
            diagram: diagram_style,
        },
        diags,
    )
}
