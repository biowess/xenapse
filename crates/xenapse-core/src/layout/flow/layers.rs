//! FLOW layering (blueprint §8.1 steps 1-2): longest-path ranks, virtual items
//! for long edges, and crossing-reducing in-layer order. No coordinates.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::model::flow::{FlowDiagram, FlowEdgeKind};
use crate::model::{EdgeId, NodeId};

/// Number of alternating down/up barycenter sweeps.
const SWEEPS: usize = 8;

/// One slot in a layer: a real node, or a pass-through of a long edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerItem {
    Node(NodeId),
    Virtual { edge: EdgeId },
}

/// Position of an item: `layers.layers[layer][index]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChainStop {
    pub(crate) layer: usize,
    pub(crate) index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layers {
    /// Items per layer, in left-to-right order.
    pub(crate) layers: Vec<Vec<LayerItem>>,
    /// Rank (layer index) of each node, indexed by `NodeId`.
    pub(crate) rank_of: Vec<usize>,
    chains: Vec<Vec<ChainStop>>,
}

impl Layers {
    /// Stops of an edge from its source node to its target node, one per
    /// layer in between (virtual items included). Empty if out of range.
    pub(crate) fn chain(&self, edge: EdgeId) -> &[ChainStop] {
        self.chains.get(edge.0).map_or(&[], Vec::as_slice)
    }
}

fn at(v: &[usize], i: usize) -> usize {
    v.get(i).copied().unwrap_or(0)
}

/// Longest-path ranks in deterministic topological order (ties by `NodeId`).
/// `None` if the graph has a cycle.
fn assign_ranks(d: &FlowDiagram) -> Option<Vec<usize>> {
    let n = d.nodes.len();
    let mut indeg: Vec<usize> = (0..n).map(|i| d.predecessors(NodeId(i)).len()).collect();
    let mut rank = vec![0usize; n];
    let mut heap = BinaryHeap::new();
    for (i, &deg) in indeg.iter().enumerate() {
        if deg == 0 {
            heap.push(Reverse(i));
        }
    }
    let mut done = 0usize;
    while let Some(Reverse(u)) = heap.pop() {
        done += 1;
        let next = at(&rank, u).saturating_add(1);
        for &e in d.successors(NodeId(u)) {
            let Some(edge) = d.edge(e) else { continue };
            let v = edge.to.0;
            if let Some(r) = rank.get_mut(v) {
                *r = (*r).max(next);
            }
            if let Some(deg) = indeg.get_mut(v) {
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    heap.push(Reverse(v));
                }
            }
        }
    }
    (done == n).then_some(rank)
}

/// Crossings between a layer and the next, given item positions.
fn cross_between(layer: &[usize], down: &[Vec<usize>], pos: &[usize]) -> usize {
    let mut es: Vec<(usize, usize)> = Vec::new();
    for &u in layer {
        let pu = at(pos, u);
        for &v in down.get(u).map_or(&[][..], Vec::as_slice) {
            es.push((pu, at(pos, v)));
        }
    }
    es.sort_unstable();
    let mut count = 0usize;
    for (i, a) in es.iter().enumerate() {
        for b in es.iter().skip(i + 1) {
            if a.0 < b.0 && a.1 > b.1 {
                count += 1;
            }
        }
    }
    count
}

fn total_crossings(layers: &[Vec<usize>], down: &[Vec<usize>], pos: &[usize]) -> usize {
    layers.iter().map(|l| cross_between(l, down, pos)).sum()
}

/// Crossings on the two layer pairs touching layer `r`.
fn local_crossings(layers: &[Vec<usize>], r: usize, down: &[Vec<usize>], pos: &[usize]) -> usize {
    let above = r
        .checked_sub(1)
        .and_then(|p| layers.get(p))
        .map_or(0, |l| cross_between(l, down, pos));
    let below = layers.get(r).map_or(0, |l| cross_between(l, down, pos));
    above + below
}

fn set_pos(layer: &[usize], pos: &mut [usize]) {
    for (i, &it) in layer.iter().enumerate() {
        if let Some(p) = pos.get_mut(it) {
            *p = i;
        }
    }
}

/// Computes ranks, virtual items and in-layer order. `None` only if the graph
/// contains a cycle (excluded by D4 validation).
pub(crate) fn compute_layers(d: &FlowDiagram) -> Option<Layers> {
    let n = d.nodes.len();
    let rank_of = assign_ranks(d)?;
    let nlayers = rank_of.iter().copied().max()?.checked_add(1)?;

    // Item ids: nodes are 0..n, virtual items follow in edge order.
    let mut kinds: Vec<LayerItem> = (0..n).map(|i| LayerItem::Node(NodeId(i))).collect();
    let mut irank = rank_of.clone();
    let mut chain_ids: Vec<Vec<usize>> = Vec::with_capacity(d.edges.len());
    for (i, e) in d.edges.iter().enumerate() {
        let rf = *rank_of.get(e.from.0)?;
        let rt = *rank_of.get(e.to.0)?;
        if rf >= rt {
            return None;
        }
        let mut c = vec![e.from.0];
        for r in rf + 1..rt {
            c.push(kinds.len());
            kinds.push(LayerItem::Virtual { edge: EdgeId(i) });
            irank.push(r);
        }
        c.push(e.to.0);
        chain_ids.push(c);
    }
    let total = kinds.len();

    let mut up: Vec<Vec<usize>> = vec![Vec::new(); total];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); total];
    for c in &chain_ids {
        for w in c.windows(2) {
            if let [a, b] = *w {
                if let Some(x) = down.get_mut(a) {
                    x.push(b);
                }
                if let Some(x) = up.get_mut(b) {
                    x.push(a);
                }
            }
        }
    }

    // Initial order: iterative DFS from START, YES before ELSE.
    let mut succ: Vec<Vec<EdgeId>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut s: Vec<EdgeId> = d.successors(NodeId(i)).to_vec();
        s.sort_by_key(|&e| {
            let k = match d.edge(e).map(|x| x.kind) {
                Some(FlowEdgeKind::Yes) => 0u8,
                Some(FlowEdgeKind::Sequence) | None => 1,
                Some(FlowEdgeKind::No) => 2,
            };
            (k, e.0)
        });
        succ.push(s);
    }
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nlayers];
    let mut visited = vec![false; n];
    for root in std::iter::once(d.start.0).chain(0..n) {
        if visited.get(root).copied().unwrap_or(true) {
            continue;
        }
        if let Some(v) = visited.get_mut(root) {
            *v = true;
        }
        if let Some(l) = layers.get_mut(at(&irank, root)) {
            l.push(root);
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(top) = stack.last_mut() {
            let (u, k) = *top;
            let Some(&e) = succ.get(u).and_then(|s| s.get(k)) else {
                stack.pop();
                continue;
            };
            top.1 += 1;
            let chain = chain_ids.get(e.0).map_or(&[][..], Vec::as_slice);
            let inner = chain.get(1..chain.len().saturating_sub(1)).unwrap_or(&[]);
            for &it in inner {
                if let Some(l) = layers.get_mut(at(&irank, it)) {
                    l.push(it);
                }
            }
            let Some(v) = d.edge(e).map(|x| x.to.0) else {
                continue;
            };
            if !visited.get(v).copied().unwrap_or(true) {
                if let Some(f) = visited.get_mut(v) {
                    *f = true;
                }
                if let Some(l) = layers.get_mut(at(&irank, v)) {
                    l.push(v);
                }
                stack.push((v, 0));
            }
        }
    }

    // Barycenter sweeps; a layer is reordered only if local crossings strictly drop.
    let mut pos = vec![0usize; total];
    for l in &layers {
        set_pos(l, &mut pos);
    }
    let mut best_total = total_crossings(&layers, &down, &pos);
    let mut best = layers.clone();
    for s in 0..SWEEPS {
        let down_sweep = s % 2 == 0;
        let order: Vec<usize> = if down_sweep {
            (1..nlayers).collect()
        } else {
            (0..nlayers.saturating_sub(1)).rev().collect()
        };
        for r in order {
            let adj = if down_sweep { &up } else { &down };
            let Some(cur) = layers.get(r).cloned() else {
                continue;
            };
            let mut keyed: Vec<(usize, u128, u128)> = cur
                .iter()
                .map(|&it| {
                    let ns = adj.get(it).map_or(&[][..], Vec::as_slice);
                    if ns.is_empty() {
                        (it, at(&pos, it) as u128, 1)
                    } else {
                        (
                            it,
                            ns.iter().map(|&x| at(&pos, x) as u128).sum(),
                            ns.len() as u128,
                        )
                    }
                })
                .collect();
            // Stable: equal barycenters keep their previous relative order.
            keyed.sort_by(|a, b| (a.1 * b.2).cmp(&(b.1 * a.2)));
            let cand: Vec<usize> = keyed.iter().map(|k| k.0).collect();
            if cand == cur {
                continue;
            }
            let before = local_crossings(&layers, r, &down, &pos);
            if let Some(l) = layers.get_mut(r) {
                *l = cand;
                set_pos(l, &mut pos);
            }
            if local_crossings(&layers, r, &down, &pos) >= before {
                if let Some(l) = layers.get_mut(r) {
                    *l = cur;
                    set_pos(l, &mut pos);
                }
            }
        }
        let t = total_crossings(&layers, &down, &pos);
        if t < best_total {
            best_total = t;
            best = layers.clone();
        }
    }
    for l in &best {
        set_pos(l, &mut pos);
    }

    let out_layers: Vec<Vec<LayerItem>> = best
        .iter()
        .map(|l| l.iter().filter_map(|&it| kinds.get(it).copied()).collect())
        .collect();
    let chains: Vec<Vec<ChainStop>> = chain_ids
        .iter()
        .map(|c| {
            c.iter()
                .map(|&it| ChainStop {
                    layer: at(&irank, it),
                    index: at(&pos, it),
                })
                .collect()
        })
        .collect();
    Some(Layers {
        layers: out_layers,
        rank_of,
        chains,
    })
}
