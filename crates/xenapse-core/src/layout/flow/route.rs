//! FLOW edge routing, label placement and orientation (blueprint §8.1 steps
//! 4-5; decision D8). Routing happens in the abstract top-to-bottom space of
//! `place_layers`; LR diagrams are transposed at the very end.

use super::layers::{ChainStop, LayerItem, Layers};
use super::position::PlacedGraph;
use crate::geometry::{Point, Rect};
use crate::lang::Direction;
use crate::layout::{
    measure_edge_label, nonneg, EdgeLayout, LabelLayout, Layout, NodeLayout, TextAnchor,
    TextMeasurer,
};
use crate::model::flow::{FlowDiagram, FlowEdgeKind, FlowNodeKind};
use crate::model::{EdgeId, NodeId};
use crate::style::ResolvedStyles;

/// A decision exits sideways only if the next stop is off-centre by more.
const EPS: f64 = 0.5;
/// Space between a label box and the segment it sits beside.
const LABEL_GAP: f64 = 2.0;

/// Vertical extent (`top`, `bottom`) of every layer in abstract space.
fn layer_extents(placed: &PlacedGraph) -> Vec<(f64, f64)> {
    placed
        .layers
        .iter()
        .map(|row| {
            let top = row.iter().map(|p| p.rect.y).fold(f64::INFINITY, f64::min);
            let bottom = row
                .iter()
                .map(|p| p.rect.bottom())
                .fold(f64::NEG_INFINITY, f64::max);
            if top.is_finite() && bottom.is_finite() {
                (top, bottom)
            } else {
                (0.0, 0.0)
            }
        })
        .collect()
}

fn stop_rect(placed: &PlacedGraph, stop: ChainStop) -> Option<Rect> {
    placed
        .layers
        .get(stop.layer)?
        .get(stop.index)
        .map(|p| p.rect)
}

/// Snaps near-collinear coordinates, removes duplicate points and redundant
/// collinear points or micro-jogs, guaranteeing clean orthogonal geometry.
fn normalize_path(pts: Vec<Point>) -> Vec<Point> {
    if pts.len() < 2 {
        return pts;
    }
    // Snap consecutive coordinates within 1.0px (e.g. sub-pixel rounding differences)
    let mut snapped: Vec<Point> = Vec::with_capacity(pts.len());
    for p in pts {
        if let Some(prev) = snapped.last_mut() {
            if (p.x - prev.x).abs() < 1.0 {
                let x = prev.x;
                snapped.push(Point::new(x, p.y));
                continue;
            } else if (p.y - prev.y).abs() < 1.0 {
                let y = prev.y;
                snapped.push(Point::new(p.x, y));
                continue;
            }
        }
        snapped.push(p);
    }

    let same = |a: Point, b: Point| (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6;
    let mut dedup: Vec<Point> = Vec::with_capacity(snapped.len());
    for p in snapped {
        if !dedup.last().is_some_and(|&l| same(l, p)) {
            dedup.push(p);
        }
    }

    let mut out: Vec<Point> = Vec::with_capacity(dedup.len());
    for p in dedup {
        while let [.., a, b] = out[..] {
            let straight_x = (a.x - b.x).abs() < 1e-6 && (b.x - p.x).abs() < 1e-6;
            let straight_y = (a.y - b.y).abs() < 1e-6 && (b.y - p.y).abs() < 1e-6;
            if straight_x || straight_y {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }

    // Eliminate micro-jogs (< 1.5px)
    let mut cleaned: Vec<Point> = Vec::with_capacity(out.len());
    let mut i = 0;
    while i < out.len() {
        if i + 3 < out.len() {
            let a = out[i];
            let b = out[i + 1];
            let c = out[i + 2];
            let d = out[i + 3];
            if (a.x - b.x).abs() < 1e-6
                && (b.y - c.y).abs() < 1e-6
                && (c.x - d.x).abs() < 1e-6
                && (b.x - c.x).abs() < 1.5
            {
                cleaned.push(a);
                cleaned.push(Point::new(a.x, d.y));
                i += 4;
                continue;
            }
            if (a.y - b.y).abs() < 1e-6
                && (b.x - c.x).abs() < 1e-6
                && (c.y - d.y).abs() < 1e-6
                && (b.y - c.y).abs() < 1.5
            {
                cleaned.push(a);
                cleaned.push(Point::new(d.x, a.y));
                i += 4;
                continue;
            }
        }
        cleaned.push(out[i]);
        i += 1;
    }

    let mut final_out: Vec<Point> = Vec::with_capacity(cleaned.len());
    for p in cleaned {
        if final_out.last().is_some_and(|&l| same(l, p)) {
            continue;
        }
        while let [.., a, b] = final_out[..] {
            let straight_x = (a.x - b.x).abs() < 1e-6 && (b.x - p.x).abs() < 1e-6;
            let straight_y = (a.y - b.y).abs() < 1e-6 && (b.y - p.y).abs() < 1e-6;
            if straight_x || straight_y {
                final_out.pop();
            } else {
                break;
            }
        }
        final_out.push(p);
    }
    final_out
}

/// Orthogonal route of one edge in abstract space.
fn route_edge(
    flow: &FlowDiagram,
    layers: &Layers,
    placed: &PlacedGraph,
    ext: &[(f64, f64)],
    edge: EdgeId,
    stub: f64,
    target_port_x: f64,
) -> Vec<Point> {
    let chain = layers.chain(edge);
    let Some(src) = chain.first().and_then(|&s| stop_rect(placed, s)) else {
        return Vec::new();
    };
    let from_decision = flow
        .edge(edge)
        .and_then(|e| flow.node(e.from))
        .is_some_and(|n| n.kind == FlowNodeKind::Decision);
    let next_x = chain
        .get(1)
        .and_then(|&s| stop_rect(placed, s))
        .map_or(src.center().x, |r| r.center().x);
    let c = src.center();
    let dx = next_x - c.x;

    let mut pts: Vec<Point> = Vec::new();
    let mut cur_x;
    if from_decision && dx < -EPS {
        pts.push(src.left_mid());
        if next_x <= src.x + 1.0 {
            cur_x = if (next_x - src.x).abs() < 1.0 {
                src.x
            } else {
                next_x
            };
            pts.push(Point::new(cur_x, c.y));
        } else {
            // Target lies under the diamond: step outside it first.
            cur_x = src.x - stub;
            pts.push(Point::new(cur_x, c.y));
        }
    } else if from_decision && dx > EPS {
        pts.push(src.right_mid());
        if next_x >= src.right() - 1.0 {
            cur_x = if (next_x - src.right()).abs() < 1.0 {
                src.right()
            } else {
                next_x
            };
            pts.push(Point::new(cur_x, c.y));
        } else {
            cur_x = src.right() + stub;
            pts.push(Point::new(cur_x, c.y));
        }
    } else {
        pts.push(src.bottom_mid());
        cur_x = c.x;
    }

    for (k, stop) in chain.iter().enumerate().skip(1) {
        let Some(r) = stop_rect(placed, *stop) else {
            return Vec::new();
        };
        let above = stop
            .layer
            .checked_sub(1)
            .and_then(|l| ext.get(l))
            .map(|e| e.1);
        let below = ext.get(stop.layer).map(|e| e.0);
        let (Some(a), Some(b)) = (above, below) else {
            return Vec::new();
        };
        let jog = (a + b) / 2.0;
        let is_last = k + 1 == chain.len();
        let mut tx = if is_last { target_port_x } else { r.center().x };
        if (tx - cur_x).abs() < 1.0 {
            tx = cur_x;
        }
        if (tx - cur_x).abs() > 1e-6 {
            pts.push(Point::new(cur_x, jog));
            pts.push(Point::new(tx, jog));
            cur_x = tx;
        }
        if is_last {
            pts.push(Point::new(tx, r.y));
        }
    }
    normalize_path(pts)
}

/// Centre of a label box of abstract size `aw` x `ah`, beside the first
/// segment: above a horizontal one, right of a downward one.
fn label_center(pts: &[Point], aw: f64, ah: f64) -> Option<Point> {
    let p0 = *pts.first()?;
    let horizontal = pts
        .get(1)
        .is_some_and(|p1| (p1.y - p0.y).abs() < 1e-9 && (p1.x - p0.x).abs() > 1e-9);
    if horizontal {
        let dir = if pts.get(1).is_some_and(|p1| p1.x < p0.x) {
            -1.0
        } else {
            1.0
        };
        Some(Point::new(
            p0.x + dir * (LABEL_GAP + aw / 2.0),
            p0.y - LABEL_GAP - ah / 2.0,
        ))
    } else {
        Some(Point::new(
            p0.x + LABEL_GAP + aw / 2.0,
            p0.y + LABEL_GAP + ah / 2.0,
        ))
    }
}

fn fin(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Routes every edge, places labels, applies the orientation and diagram
/// padding, and returns the final layout.
pub(super) fn route_and_transform(
    flow: &FlowDiagram,
    layers: &Layers,
    placed: &PlacedGraph,
    nodes: Vec<NodeLayout>,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
) -> Layout {
    let swap = flow.direction == Direction::LeftToRight;
    let ext = layer_extents(placed);
    let stub = nonneg(styles.diagram.layout.node_gap) / 2.0;
    let pad = nonneg(styles.diagram.layout.diagram_padding);
    let tp = |p: Point| {
        if swap {
            Point::new(fin(p.y), fin(p.x))
        } else {
            Point::new(fin(p.x), fin(p.y))
        }
    };

    // Target ports for edges entering nodes from above
    let mut target_ports = vec![0.0_f64; flow.edges.len()];
    for nid in 0..flow.nodes.len() {
        let preds = flow.predecessors(NodeId(nid));
        if preds.is_empty() {
            continue;
        }
        let Some(first_chain) = layers.chain(preds[0]).last() else {
            continue;
        };
        let Some(dst_rect) = stop_rect(placed, *first_chain) else {
            continue;
        };
        let dst_cx = dst_rect.center().x;
        let dst_w = dst_rect.width;

        if preds.len() == 1 {
            target_ports[preds[0].0] = dst_cx;
        } else {
            let mut inc: Vec<(f64, EdgeId)> = preds
                .iter()
                .map(|&eid| {
                    let chain = layers.chain(eid);
                    let px = chain
                        .get(chain.len().saturating_sub(2))
                        .and_then(|&s| stop_rect(placed, s))
                        .map_or(dst_cx, |r| r.center().x);
                    (px, eid)
                })
                .collect();
            inc.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1 .0.cmp(&b.1 .0)));

            let m = inc.len();
            if m == 2 {
                let spacing = 16.0_f64.min(dst_w * 0.25).max(8.0);
                target_ports[inc[0].1 .0] = dst_cx - spacing;
                target_ports[inc[1].1 .0] = dst_cx + spacing;
            } else {
                let span = (dst_w * 0.6).min((m - 1) as f64 * 24.0);
                let step = span / (m - 1) as f64;
                let start = dst_cx - span / 2.0;
                for (rank, (_, eid)) in inc.iter().enumerate() {
                    target_ports[eid.0] = start + rank as f64 * step;
                }
            }
        }
    }

    // Node rects: abstract (swapped sizes for LR) -> real.
    let mut rects: Vec<Rect> = nodes.iter().map(|n| n.rect).collect();
    for row in &placed.layers {
        for p in row {
            if let LayerItem::Node(id) = p.item {
                if let Some(r) = rects.get_mut(id.0) {
                    let a = p.rect;
                    *r = if swap {
                        Rect::new(fin(a.y), fin(a.x), fin(a.height), fin(a.width))
                    } else {
                        Rect::new(fin(a.x), fin(a.y), fin(a.width), fin(a.height))
                    };
                }
            }
        }
    }

    // Routes and labels in real (transposed) space, before padding.
    struct Raw {
        points: Vec<Point>,
        label: Option<(String, Point, f64, f64)>,
    }
    let mut raws: Vec<Raw> = Vec::with_capacity(flow.edges.len());
    for (i, e) in flow.edges.iter().enumerate() {
        let abs_pts = route_edge(
            flow,
            layers,
            placed,
            &ext,
            EdgeId(i),
            stub,
            target_ports.get(i).copied().unwrap_or(0.0),
        );
        let text = e.label.clone().or_else(|| match e.kind {
            FlowEdgeKind::Yes => Some("Yes".to_string()),
            FlowEdgeKind::No => Some("No".to_string()),
            FlowEdgeKind::Sequence => None,
        });
        let label = match (text, styles.edge(EdgeId(i))) {
            (Some(t), Some(style)) => {
                let s = measure_edge_label(&t, style, measurer);
                let (lw, lh) = (nonneg(s.width), nonneg(s.height));
                let (aw, ah) = if swap { (lh, lw) } else { (lw, lh) };
                label_center(&abs_pts, aw, ah).map(|c| (t, tp(c), lw, lh))
            }
            _ => None,
        };
        raws.push(Raw {
            points: abs_pts.iter().map(|&p| tp(p)).collect(),
            label,
        });
    }

    // Extents of everything drawn, then shift so the content starts at `pad`.
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut grow = |x0: f64, y0: f64, x1: f64, y1: f64| {
        min_x = min_x.min(x0);
        min_y = min_y.min(y0);
        max_x = max_x.max(x1);
        max_y = max_y.max(y1);
    };
    for (i, r) in rects.iter().enumerate() {
        if flow.nodes.get(i).is_some_and(|n| n.hidden) {
            continue;
        }
        grow(r.x, r.y, r.right(), r.bottom());
    }
    for (i, raw) in raws.iter().enumerate() {
        if flow.edges.get(i).is_some_and(|e| {
            flow.nodes.get(e.from.0).is_some_and(|n| n.hidden)
                || flow.nodes.get(e.to.0).is_some_and(|n| n.hidden)
        }) {
            continue;
        }
        for p in &raw.points {
            grow(p.x, p.y, p.x, p.y);
        }
        if let Some((_, c, w, h)) = &raw.label {
            grow(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0);
        }
    }
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        (min_x, min_y, max_x, max_y) = (0.0, 0.0, 0.0, 0.0);
    }
    let (dx, dy) = (pad - min_x, pad - min_y);
    let shift = |p: Point| Point::new(p.x + dx, p.y + dy);

    let out_nodes: Vec<NodeLayout> = nodes
        .into_iter()
        .zip(rects)
        .map(|(mut n, r)| {
            n.rect = Rect::new(r.x + dx, r.y + dy, r.width, r.height);
            n
        })
        .collect();
    let out_edges: Vec<EdgeLayout> = raws
        .into_iter()
        .map(|raw| EdgeLayout {
            points: raw.points.into_iter().map(shift).collect(),
            label: raw.label.map(|(text, c, _, _)| LabelLayout {
                text,
                position: shift(c),
                anchor: TextAnchor::Middle,
            }),
        })
        .collect();

    Layout {
        width: fin(max_x - min_x + 2.0 * pad),
        height: fin(max_y - min_y + 2.0 * pad),
        nodes: out_nodes,
        edges: out_edges,
    }
}
