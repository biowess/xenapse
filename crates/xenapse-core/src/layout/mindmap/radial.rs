//! RADIAL MINDMAP layout (blueprint §8.2; decisions D5, D8).
//!
//! Each subtree gets an angular sector proportional to its leaf count. The
//! root takes the full circle, starting at the top and moving clockwise, and
//! children take consecutive sub-sectors in source order. A node sits at the
//! centre angle of its sector. The radius per depth starts from the previous
//! ring plus the rank gap and the node extents, then grows in bounded steps
//! until neighbouring boxes on the ring no longer overlap. Connectors are
//! straight segments clipped to the node boxes.
//!
//! Nodes are in pre-order, so every child has a larger index than its parent:
//! a reverse pass handles children before parents and a forward pass places
//! parents before children. No recursion is used.

use crate::geometry::{Point, Rect};
use crate::layout::{measure_nodes, nonneg, EdgeLayout, Layout, TextMeasurer};
use crate::model::Diagram;
use crate::style::ResolvedStyles;

/// Upper bound on growth steps per ring.
const MAX_GROW_STEPS: usize = 4096;
/// The growth step increases by one base step every this many iterations.
const RAMP: usize = 64;

/// Distance from a rectangle's centre to its border along `(dx, dy)`, as a
/// multiple of that vector. Zero for a zero vector.
fn border_scale(w: f64, h: f64, dx: f64, dy: f64) -> f64 {
    let tx = if dx.abs() > 1e-12 {
        (w / 2.0) / dx.abs()
    } else {
        f64::INFINITY
    };
    let ty = if dy.abs() > 1e-12 {
        (h / 2.0) / dy.abs()
    } else {
        f64::INFINITY
    };
    let t = tx.min(ty);
    if t.is_finite() {
        t
    } else {
        0.0
    }
}

fn overlaps(a: &Rect, b: &Rect, gap: f64) -> bool {
    a.x < b.right() + gap
        && b.x < a.right() + gap
        && a.y < b.bottom() + gap
        && b.y < a.bottom() + gap
}

pub(crate) fn layout_radial(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
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

    let size: Vec<(f64, f64)> = nodes
        .iter()
        .take(n)
        .map(|nl| (nonneg(nl.rect.width), nonneg(nl.rect.height)))
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

    // Pass 1 (children before parents): leaf counts, minimum one.
    let mut leaves = vec![1usize; n];
    for i in (0..n).rev() {
        let ks = kids.get(i).map(Vec::as_slice).unwrap_or(&[]);
        if !ks.is_empty() {
            let total = ks.iter().fold(0usize, |acc, &c| {
                acc.saturating_add(leaves.get(c).copied().unwrap_or(1))
            });
            if let Some(slot) = leaves.get_mut(i) {
                *slot = total.max(1);
            }
        }
    }

    // Pass 2 (parents before children): depth and sector per node. Angles are
    // measured clockwise from the top.
    let mut depth = vec![0usize; n];
    let mut sector = vec![(0.0_f64, 0.0_f64); n];
    if let Some(root) = sector.first_mut() {
        *root = (0.0, std::f64::consts::TAU);
    }
    for i in 0..n {
        let (start, end) = sector.get(i).copied().unwrap_or((0.0, 0.0));
        let d = depth.get(i).copied().unwrap_or(0);
        let ks = kids.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let total = leaves.get(i).copied().unwrap_or(1).max(1) as f64;
        let mut acc = 0usize;
        for &c in ks {
            let from = start + (end - start) * (acc as f64) / total;
            acc = acc.saturating_add(leaves.get(c).copied().unwrap_or(1));
            let to = start + (end - start) * (acc as f64) / total;
            if let (Some(s), Some(dep)) = (sector.get_mut(c), depth.get_mut(c)) {
                *s = (from, to);
                *dep = d + 1;
            }
        }
    }
    let angle: Vec<f64> = sector.iter().map(|&(a, b)| (a + b) / 2.0).collect();
    // Unit direction of each node from the origin (clockwise from the top).
    let dir: Vec<(f64, f64)> = angle.iter().map(|&a| (a.sin(), -a.cos())).collect();

    // Nodes per depth, in index order (= source order = angular order).
    let levels = depth.iter().copied().max().map_or(0, |d| d + 1);
    let mut rings: Vec<Vec<usize>> = vec![Vec::new(); levels];
    for (i, &d) in depth.iter().enumerate() {
        if let Some(r) = rings.get_mut(d) {
            r.push(i);
        }
    }

    // Centre-to-border distance of each node along its own ray.
    let reach: Vec<f64> = (0..n)
        .map(|i| {
            let (w, h) = size.get(i).copied().unwrap_or((0.0, 0.0));
            let (dx, dy) = dir.get(i).copied().unwrap_or((0.0, 1.0));
            border_scale(w, h, dx, dy)
        })
        .collect();
    let max_reach = |ring: &[usize]| {
        ring.iter()
            .map(|&i| reach.get(i).copied().unwrap_or(0.0))
            .fold(0.0_f64, f64::max)
    };

    // Radii, outwards: each ring starts from the previous one, so any growth
    // propagates to all deeper rings.
    let base_step = (node_gap / 2.0).max(4.0);
    let mut radius = vec![0.0_f64; levels];
    for d in 1..levels {
        let ring = rings.get(d).map(Vec::as_slice).unwrap_or(&[]);
        let prev = rings.get(d - 1).map(Vec::as_slice).unwrap_or(&[]);
        let mut r = radius.get(d - 1).copied().unwrap_or(0.0)
            + rank_gap
            + max_reach(prev)
            + max_reach(ring);
        let rect_at = |i: usize, r: f64| -> Rect {
            let (w, h) = size.get(i).copied().unwrap_or((0.0, 0.0));
            let (dx, dy) = dir.get(i).copied().unwrap_or((0.0, 1.0));
            Rect::new(dx * r - w / 2.0, dy * r - h / 2.0, w, h)
        };
        if ring.len() > 1 {
            for step in 0..MAX_GROW_STEPS {
                // Adjacent pairs, including the wrap-around pair.
                let clash = (0..ring.len()).any(|k| {
                    let a = ring.get(k).copied();
                    let b = ring.get((k + 1) % ring.len()).copied();
                    match (a, b) {
                        (Some(a), Some(b)) if a != b => {
                            overlaps(&rect_at(a, r), &rect_at(b, r), node_gap)
                        }
                        _ => false,
                    }
                });
                if !clash {
                    break;
                }
                r += base_step * (1 + step / RAMP) as f64;
            }
        }
        if let Some(slot) = radius.get_mut(d) {
            *slot = r;
        }
    }

    // Final rectangles around the origin.
    let mut rects: Vec<Rect> = Vec::with_capacity(n);
    for i in 0..n {
        let (w, h) = size.get(i).copied().unwrap_or((0.0, 0.0));
        let (dx, dy) = dir.get(i).copied().unwrap_or((0.0, 1.0));
        let r = depth
            .get(i)
            .and_then(|&d| radius.get(d))
            .copied()
            .unwrap_or(0.0);
        rects.push(Rect::new(dx * r - w / 2.0, dy * r - h / 2.0, w, h));
    }

    // Straight connectors clipped to both boxes.
    let mut edge_pts: Vec<Vec<Point>> = Vec::new();
    for e in 0..map.edge_count() {
        let pts = match map.edge(e) {
            Some((p, c)) if p.0 < n && c.0 < n => {
                let pr = rects.get(p.0).copied().unwrap_or_default();
                let cr = rects.get(c.0).copied().unwrap_or_default();
                let (a, b) = (pr.center(), cr.center());
                let (vx, vy) = (b.x - a.x, b.y - a.y);
                let dist = (vx * vx + vy * vy).sqrt();
                let tp = border_scale(pr.width, pr.height, vx, vy);
                let tc = border_scale(cr.width, cr.height, vx, vy);
                if dist > 1e-9 && tp + tc < 1.0 {
                    vec![
                        Point::new(a.x + vx * tp, a.y + vy * tp),
                        Point::new(b.x - vx * tc, b.y - vy * tc),
                    ]
                } else {
                    vec![a, b]
                }
            }
            _ => Vec::new(),
        };
        edge_pts.push(pts);
    }

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
