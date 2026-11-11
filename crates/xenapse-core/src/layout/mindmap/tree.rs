//! TREE / HORIZONTAL MINDMAP layout: a contour-based tidy tree
//! (Reingold-Tilford style) with elbow connectors (blueprint §8.2; D5, D8).
//!
//! The algorithm works in an abstract top-to-bottom space. HORIZONTAL runs it
//! with swapped node sizes and transposes the result. Nodes are in pre-order,
//! so every child has a larger index than its parent: a reverse pass handles
//! children before parents and a forward pass places parents before children.
//! No recursion is used.

use crate::geometry::{Point, Rect};
use crate::layout::{measure_nodes, nonneg, EdgeLayout, Layout, TextMeasurer};
use crate::model::Diagram;
use crate::style::ResolvedStyles;

/// Left and right extent of a subtree per relative depth, measured from the
/// subtree root's centre line.
struct Contour {
    left: Vec<f64>,
    right: Vec<f64>,
}

pub(crate) fn layout_tree(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
    horizontal: bool,
) -> Layout {
    let mut nodes = measure_nodes(diagram, styles, measurer);
    let Diagram::Mindmap(map) = diagram else {
        return Layout {
            width: 0.0,
            height: 0.0,
            nodes,
            edges: Vec::new(),
        };
    };
    let n = nodes.len().min(map.nodes.len());
    let gaps = &styles.diagram.layout;
    let node_gap = nonneg(gaps.node_gap);
    let rank_gap = nonneg(gaps.rank_gap);
    let padding = nonneg(gaps.diagram_padding);

    // Abstract sizes (width along the sibling axis, height along the depth axis).
    let size: Vec<(f64, f64)> = nodes
        .iter()
        .take(n)
        .map(|nl| {
            let (w, h) = (nonneg(nl.rect.width), nonneg(nl.rect.height));
            if horizontal {
                (h, w)
            } else {
                (w, h)
            }
        })
        .collect();

    // Valid children per node (always later in pre-order).
    let kids: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            map.nodes
                .get(i)
                .map(|m| {
                    m.children
                        .iter()
                        .map(|c| c.0)
                        .filter(|&c| c > i && c < n)
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    // Pass 1 (children before parents): offsets of children from their parent.
    let mut dx = vec![0.0_f64; n];
    let mut contours: Vec<Option<Contour>> = (0..n).map(|_| None).collect();
    for i in (0..n).rev() {
        let half = size.get(i).map_or(0.0, |s| s.0) / 2.0;
        let mut left = vec![-half];
        let mut right = vec![half];
        let mut placed: Vec<(usize, f64)> = Vec::new();
        let mut acc_left: Vec<f64> = Vec::new();
        let mut acc_right: Vec<f64> = Vec::new();
        for &c in kids.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            let Some(cont) = contours.get_mut(c).and_then(Option::take) else {
                continue;
            };
            let shift = if placed.is_empty() {
                0.0
            } else {
                let s = acc_right
                    .iter()
                    .zip(cont.left.iter())
                    .map(|(r, l)| r + node_gap - l)
                    .fold(f64::NEG_INFINITY, f64::max);
                if s.is_finite() {
                    s
                } else {
                    0.0
                }
            };
            for (d, (&l, &r)) in cont.left.iter().zip(cont.right.iter()).enumerate() {
                if let Some(slot) = acc_right.get_mut(d) {
                    *slot = r + shift;
                } else {
                    acc_left.push(l + shift);
                    acc_right.push(r + shift);
                }
            }
            placed.push((c, shift));
        }
        if let (Some(first), Some(last)) = (placed.first(), placed.last()) {
            // Parent centred between its first and last child.
            let centre = (first.1 + last.1) / 2.0;
            for &(c, off) in &placed {
                if let Some(slot) = dx.get_mut(c) {
                    *slot = off - centre;
                }
            }
            left.extend(acc_left.iter().map(|v| v - centre));
            right.extend(acc_right.iter().map(|v| v - centre));
        }
        if let Some(slot) = contours.get_mut(i) {
            *slot = Some(Contour { left, right });
        }
    }

    // Pass 2 (parents before children): absolute centre x and depth.
    let mut cx = vec![0.0_f64; n];
    let mut depth = vec![0usize; n];
    for i in 0..n {
        let (px, pd) = (
            cx.get(i).copied().unwrap_or(0.0),
            depth.get(i).copied().unwrap_or(0),
        );
        for &c in kids.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            if let (Some(x), Some(d)) = (cx.get_mut(c), depth.get_mut(c)) {
                *x = px + dx.get(c).copied().unwrap_or(0.0);
                *d = pd + 1;
            }
        }
    }

    // Levels: one band per depth, sized by its tallest node.
    let levels = depth.iter().copied().max().map_or(0, |d| d + 1);
    let mut level_h = vec![0.0_f64; levels];
    for (i, &d) in depth.iter().enumerate() {
        if let (Some(slot), Some(s)) = (level_h.get_mut(d), size.get(i)) {
            *slot = slot.max(s.1);
        }
    }
    let mut level_top = Vec::with_capacity(levels);
    let mut y = 0.0_f64;
    for &h in &level_h {
        level_top.push(y);
        y += h + rank_gap;
    }

    // Abstract rects (nodes centred in their band).
    let mut abs: Vec<Rect> = Vec::with_capacity(n);
    for i in 0..n {
        let (w, h) = size.get(i).copied().unwrap_or((0.0, 0.0));
        let d = depth.get(i).copied().unwrap_or(0);
        let top = level_top.get(d).copied().unwrap_or(0.0);
        let band = level_h.get(d).copied().unwrap_or(0.0);
        abs.push(Rect::new(
            cx.get(i).copied().unwrap_or(0.0) - w / 2.0,
            top + (band - h) / 2.0,
            w,
            h,
        ));
    }

    // Elbow connectors, one per non-root node, in abstract space.
    let mut abs_edges: Vec<Vec<Point>> = Vec::new();
    for e in 0..map.edge_count() {
        let pts = match map.edge(e) {
            Some((p, c)) if p.0 < n && c.0 < n => {
                let (pr, cr) = (
                    abs.get(p.0).copied().unwrap_or_default(),
                    abs.get(c.0).copied().unwrap_or_default(),
                );
                let pd = depth.get(p.0).copied().unwrap_or(0);
                let band_bottom = level_top.get(pd).copied().unwrap_or(0.0)
                    + level_h.get(pd).copied().unwrap_or(0.0);
                let mid = band_bottom + rank_gap / 2.0;
                let (a, b) = (pr.bottom_mid(), cr.top_mid());
                if (a.x - b.x).abs() < 1e-9 {
                    vec![a, b]
                } else {
                    vec![a, Point::new(a.x, mid), Point::new(b.x, mid), b]
                }
            }
            _ => Vec::new(),
        };
        abs_edges.push(pts);
    }

    // Transpose for HORIZONTAL.
    let flip = |p: Point| if horizontal { Point::new(p.y, p.x) } else { p };
    let mut rects: Vec<Rect> = abs
        .iter()
        .map(|r| {
            if horizontal {
                Rect::new(r.y, r.x, r.height, r.width)
            } else {
                *r
            }
        })
        .collect();
    let mut edge_pts: Vec<Vec<Point>> = abs_edges
        .iter()
        .map(|v| v.iter().map(|&p| flip(p)).collect())
        .collect();

    // Normalise to non-negative coordinates with diagram padding.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for r in &rects {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_x = max_x.max(r.right());
        max_y = max_y.max(r.bottom());
    }
    for p in edge_pts.iter().flatten() {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        (min_x, min_y, max_x, max_y) = (0.0, 0.0, 0.0, 0.0);
    }
    let (ox, oy) = (padding - min_x, padding - min_y);
    for r in &mut rects {
        r.x += ox;
        r.y += oy;
    }
    for p in edge_pts.iter_mut().flatten() {
        p.x += ox;
        p.y += oy;
    }
    for (nl, r) in nodes.iter_mut().zip(rects) {
        nl.rect = r;
    }

    Layout {
        width: max_x - min_x + 2.0 * padding,
        height: max_y - min_y + 2.0 * padding,
        nodes,
        edges: edge_pts
            .into_iter()
            .map(|points| EdgeLayout {
                points,
                label: None,
            })
            .collect(),
    }
}
