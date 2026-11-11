//! FLOW coordinate assignment (blueprint §8.1 step 3; decisions D7, D8).
//!
//! Turns layers into rectangles in an abstract top-to-bottom space. For LR
//! diagrams node width/height are swapped here; the transpose, padding and the
//! final transform are applied later.

use std::cmp::Reverse;

use super::layers::{LayerItem, Layers};
use crate::geometry::{Rect, Size};
use crate::lang::Direction;
use crate::layout::{nonneg, NodeLayout};
use crate::model::flow::FlowDiagram;
use crate::model::EdgeId;
use crate::style::DiagramStyle;

/// Fixed width of a virtual item (a long-edge channel).
const VIRTUAL_WIDTH: f64 = 8.0;
/// Number of (down, up) relaxation pass pairs. The last pass is an up pass.
const PASSES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlacedItem {
    pub(crate) item: LayerItem,
    pub(crate) rect: Rect,
}

/// Placed items per layer (left to right, same order as `Layers`) and total
/// bounds. The minimum x is 0 and y starts at 0; no padding is included.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedGraph {
    pub(crate) layers: Vec<Vec<PlacedItem>>,
    pub(crate) bounds: Size,
}

fn get(v: &[f64], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(0.0)
}

fn set(v: &mut [f64], i: usize, val: f64) {
    if let Some(s) = v.get_mut(i) {
        *s = val;
    }
}

/// Flat per-item state; item ids are consecutive per layer.
struct Flat {
    /// `starts[l]..starts[l + 1]` are the ids of layer `l`.
    starts: Vec<usize>,
    w: Vec<f64>,
    x: Vec<f64>,
    /// Minimal left-edge distance from the layer's first item to this item.
    cum: Vec<f64>,
    up: Vec<Vec<usize>>,
    down: Vec<Vec<usize>>,
}

impl Flat {
    fn span(&self, l: usize) -> (usize, usize) {
        let s = self.starts.get(l).copied().unwrap_or(0);
        let e = self.starts.get(l + 1).copied().unwrap_or(s);
        (s, e)
    }

    fn flat_id(&self, layer: usize, index: usize) -> Option<usize> {
        let (s, e) = self.span(layer);
        (index < e - s).then_some(s + index)
    }

    /// Moves each item of layer `l` toward the median x of its neighbours in
    /// the adjacent layer, highest degree first (ties by index), keeping
    /// every separation. Contiguous sibling groups sharing a single parent
    /// are centered symmetrically under that parent.
    fn relax_layer(&mut self, l: usize, from_above: bool) {
        let (s, e) = self.span(l);
        let mut want: Vec<f64> = vec![0.0; e - s];
        let mut processed = vec![false; e - s];

        let mut i = 0;
        while i < e - s {
            let id = s + i;
            let ns = if from_above {
                self.up.get(id)
            } else {
                self.down.get(id)
            };
            let ns = ns.map_or(&[][..], Vec::as_slice);
            if ns.len() == 1 {
                let target = ns[0];
                let mut j = i + 1;
                while j < e - s {
                    let next_id = s + j;
                    let next_ns = if from_above {
                        self.up.get(next_id)
                    } else {
                        self.down.get(next_id)
                    };
                    let next_ns = next_ns.map_or(&[][..], Vec::as_slice);
                    if next_ns == [target] {
                        j += 1;
                    } else {
                        break;
                    }
                }
                let first_id = s + i;
                let last_id = s + j - 1;
                let group_w =
                    (get(&self.cum, last_id) + get(&self.w, last_id)) - get(&self.cum, first_id);
                let target_center = get(&self.x, target) + get(&self.w, target) / 2.0;
                let g_left = target_center - group_w / 2.0;
                for k in i..j {
                    let cur_id = s + k;
                    want[k] = g_left + (get(&self.cum, cur_id) - get(&self.cum, first_id));
                    processed[k] = true;
                }
                i = j;
            } else {
                i += 1;
            }
        }

        for (i, p) in processed.iter().enumerate() {
            if *p {
                continue;
            }
            let id = s + i;
            let ns = if from_above {
                self.up.get(id)
            } else {
                self.down.get(id)
            };
            let ns = ns.map_or(&[][..], Vec::as_slice);
            let mut cs: Vec<f64> = ns
                .iter()
                .map(|&j| get(&self.x, j) + get(&self.w, j) / 2.0)
                .collect();
            cs.sort_by(|a, b| a.total_cmp(b));
            let n = cs.len();
            let med = match n {
                0 => None,
                _ if n % 2 == 1 => cs.get(n / 2).copied(),
                _ => match (cs.get(n / 2 - 1), cs.get(n / 2)) {
                    (Some(a), Some(b)) => Some((a + b) / 2.0),
                    _ => None,
                },
            };
            want[i] = med.map_or(get(&self.x, id), |m| m - get(&self.w, id) / 2.0);
        }

        let mut order: Vec<(Reverse<usize>, usize)> = Vec::with_capacity(e - s);
        for id in s..e {
            let ns = if from_above {
                self.up.get(id)
            } else {
                self.down.get(id)
            };
            let n = ns.map_or(0, |x| x.len());
            order.push((Reverse(n), id));
        }
        order.sort_unstable();

        let mut fixed = vec![false; e - s];
        for (_, id) in order {
            let mut lo = f64::NEG_INFINITY;
            for k in (s..id).rev() {
                if fixed.get(k - s).copied().unwrap_or(false) {
                    lo = get(&self.x, k) + get(&self.cum, id) - get(&self.cum, k);
                    break;
                }
            }
            let mut hi = f64::INFINITY;
            for k in id + 1..e {
                if fixed.get(k - s).copied().unwrap_or(false) {
                    hi = get(&self.x, k) - (get(&self.cum, k) - get(&self.cum, id));
                    break;
                }
            }
            let v = get(&want, id - s).max(lo).min(hi);
            set(&mut self.x, id, v);
            if let Some(f) = fixed.get_mut(id - s) {
                *f = true;
            }
        }
    }
}

/// Assigns abstract coordinates to every item of `layers`.
pub(crate) fn place_layers(
    layers: &Layers,
    diagram: &FlowDiagram,
    nodes: &[NodeLayout],
    style: &DiagramStyle,
) -> PlacedGraph {
    let node_gap = nonneg(style.layout.node_gap);
    let branch_gap = nonneg(style.layout.branch_gap);
    let rank_gap = nonneg(style.layout.rank_gap);
    let swap = diagram.direction == Direction::LeftToRight;

    // Sizes (abstract space) and flat ids.
    let mut starts = vec![0usize];
    let mut w: Vec<f64> = Vec::new();
    let mut h: Vec<f64> = Vec::new();
    let mut virt: Vec<bool> = Vec::new();
    for layer in &layers.layers {
        for item in layer {
            match item {
                LayerItem::Node(id) => {
                    let (nw, nh) = nodes.get(id.0).map_or((0.0, 0.0), |n| {
                        (nonneg(n.rect.width), nonneg(n.rect.height))
                    });
                    let (aw, ah) = if swap { (nh, nw) } else { (nw, nh) };
                    w.push(aw);
                    h.push(ah);
                    virt.push(false);
                }
                LayerItem::Virtual { .. } => {
                    w.push(VIRTUAL_WIDTH);
                    h.push(0.0);
                    virt.push(true);
                }
            }
        }
        starts.push(w.len());
    }
    let total = w.len();

    // Adjacency between consecutive stops of every edge chain.
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); total];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); total];
    let flat_id_init = |layer: usize, index: usize| -> Option<usize> {
        let s = *starts.get(layer)?;
        let e = *starts.get(layer + 1)?;
        (index < e - s).then_some(s + index)
    };
    for i in 0..diagram.edges.len() {
        for pair in layers.chain(EdgeId(i)).windows(2) {
            let [a, b] = pair else { continue };
            if b.layer != a.layer + 1 {
                continue;
            }
            if let (Some(fa), Some(fb)) = (
                flat_id_init(a.layer, a.index),
                flat_id_init(b.layer, b.index),
            ) {
                if let Some(d) = down.get_mut(fa) {
                    d.push(fb);
                }
                if let Some(u) = up.get_mut(fb) {
                    u.push(fa);
                }
            }
        }
    }

    // Separation prefix sums and the initial left-to-right packing.
    let mut cum = vec![0.0; total];
    let mut x = vec![0.0; total];
    for l in 0..layers.layers.len() {
        let s = starts.get(l).copied().unwrap_or(0);
        let e = starts.get(l + 1).copied().unwrap_or(s);
        let mut acc = 0.0;
        for id in s..e {
            set(&mut cum, id, acc);
            set(&mut x, id, acc);
            let next_virtual = id + 1 < e && virt.get(id + 1).copied().unwrap_or(false);
            let gap = if virt.get(id).copied().unwrap_or(false) || next_virtual {
                branch_gap
            } else {
                node_gap
            };
            acc += get(&w, id) + gap;
        }
    }

    let mut flat = Flat {
        starts,
        w,
        x,
        cum,
        up,
        down,
    };
    let nlayers = layers.layers.len();
    for _ in 0..PASSES {
        for l in 1..nlayers {
            flat.relax_layer(l, true);
        }
        for l in (0..nlayers.saturating_sub(1)).rev() {
            flat.relax_layer(l, false);
        }
    }

    // Channel alignment for long edges: keep virtual segments strictly vertical.
    for i in 0..diagram.edges.len() {
        let chain = layers.chain(EdgeId(i));
        if chain.len() <= 2 {
            continue;
        }
        let v_stops = &chain[1..chain.len() - 1];
        let mut lo_max = f64::NEG_INFINITY;
        let mut hi_min = f64::INFINITY;
        let mut current_xs = Vec::with_capacity(v_stops.len());

        for stop in v_stops {
            let Some(id) = flat.flat_id(stop.layer, stop.index) else {
                continue;
            };
            current_xs.push(get(&flat.x, id));
            let (s, e) = flat.span(stop.layer);
            let mut lo = f64::NEG_INFINITY;
            if id > s {
                let left_id = id - 1;
                lo = get(&flat.x, left_id) + get(&flat.w, left_id) + branch_gap;
            }
            let mut hi = f64::INFINITY;
            if id + 1 < e {
                let right_id = id + 1;
                hi = get(&flat.x, right_id) - branch_gap - get(&flat.w, id);
            }
            lo_max = lo_max.max(lo);
            hi_min = hi_min.min(hi);
        }

        if lo_max <= hi_min && !current_xs.is_empty() {
            current_xs.sort_by(|a, b| a.total_cmp(b));
            let med_x = current_xs[current_xs.len() / 2];
            let channel_x = med_x.clamp(lo_max, hi_min);
            for stop in v_stops {
                if let Some(id) = flat.flat_id(stop.layer, stop.index) {
                    set(&mut flat.x, id, channel_x);
                }
            }
        }
    }

    // Sequential node snapping: if parent and single child are nearly aligned, snap exactly.
    for i in 0..diagram.edges.len() {
        let chain = layers.chain(EdgeId(i));
        if chain.len() != 2 {
            continue;
        }
        let stop_a = chain[0];
        let stop_b = chain[1];
        if stop_b.layer != stop_a.layer + 1 {
            continue;
        }
        let (Some(id_a), Some(id_b)) = (
            flat.flat_id(stop_a.layer, stop_a.index),
            flat.flat_id(stop_b.layer, stop_b.index),
        ) else {
            continue;
        };
        let a_down = flat.down.get(id_a).map_or(&[][..], Vec::as_slice);
        let b_up = flat.up.get(id_b).map_or(&[][..], Vec::as_slice);
        if a_down == [id_b] && b_up == [id_a] {
            let ca = get(&flat.x, id_a) + get(&flat.w, id_a) / 2.0;
            let cb = get(&flat.x, id_b) + get(&flat.w, id_b) / 2.0;
            if (ca - cb).abs() < 2.0 {
                let want_xb = ca - get(&flat.w, id_b) / 2.0;
                let (s, e) = flat.span(stop_b.layer);
                let mut lo = f64::NEG_INFINITY;
                if id_b > s {
                    let left_id = id_b - 1;
                    lo = get(&flat.x, left_id) + get(&flat.w, left_id) + node_gap;
                }
                let mut hi = f64::INFINITY;
                if id_b + 1 < e {
                    let right_id = id_b + 1;
                    hi = get(&flat.x, right_id) - node_gap - get(&flat.w, id_b);
                }
                if want_xb >= lo && want_xb <= hi {
                    set(&mut flat.x, id_b, want_xb);
                }
            }
        }
    }

    // Vertical stacking; items are centred in their layer.
    let min_x = flat.x.iter().copied().fold(f64::INFINITY, f64::min);
    let shift = if min_x.is_finite() { -min_x } else { 0.0 };
    let mut placed: Vec<Vec<PlacedItem>> = Vec::with_capacity(nlayers);
    let mut y = 0.0;
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    for (l, layer) in layers.layers.iter().enumerate() {
        let (s, _) = flat.span(l);
        let lh = (0..layer.len())
            .map(|i| get(&h, s + i))
            .fold(0.0_f64, f64::max);
        let mut row = Vec::with_capacity(layer.len());
        for (i, item) in layer.iter().enumerate() {
            let id = s + i;
            let (iw, ih) = (get(&flat.w, id), get(&h, id));
            let rect = Rect::new(get(&flat.x, id) + shift, y + (lh - ih) / 2.0, iw, ih);
            width = width.max(rect.right());
            row.push(PlacedItem { item: *item, rect });
        }
        placed.push(row);
        height = y + lh;
        y = height + rank_gap;
    }
    PlacedGraph {
        layers: placed,
        bounds: Size::new(width, height),
    }
}
