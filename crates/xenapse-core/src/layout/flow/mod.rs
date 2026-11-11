//! FLOW layout (blueprint §8.1; decision D8).

pub(crate) mod layers;
pub(crate) mod position;
pub(crate) mod route;

use super::{measure_nodes, EdgeLayout, Layout, TextMeasurer};
use crate::model::Diagram;
use crate::style::ResolvedStyles;

/// Full FLOW layout: measure → layers → positions → routes → transform.
/// Every `NodeId` and `EdgeId` gets an entry. A non-FLOW diagram or a cyclic
/// graph (excluded by validation) yields measured nodes at the origin and
/// edges without routes.
pub(crate) fn layout_flow(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
) -> Layout {
    let nodes = measure_nodes(diagram, styles, measurer);
    let Diagram::Flow(flow) = diagram else {
        return Layout {
            width: 0.0,
            height: 0.0,
            nodes,
            edges: Vec::new(),
        };
    };
    let Some(layers) = layers::compute_layers(flow) else {
        let edges = flow
            .edges
            .iter()
            .map(|_| EdgeLayout {
                points: Vec::new(),
                label: None,
            })
            .collect();
        return Layout {
            width: 0.0,
            height: 0.0,
            nodes,
            edges,
        };
    };
    let placed = position::place_layers(&layers, flow, &nodes, &styles.diagram);
    route::route_and_transform(flow, &layers, &placed, nodes, styles, measurer)
}
