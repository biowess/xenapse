//! MINDMAP layouts (blueprint §8.2; decisions D5, D8). Kept separate from the
//! FLOW layout on purpose.

pub(crate) mod radial;
pub(crate) mod tree;

use crate::lang::LayoutMode;
use crate::layout::{Layout, TextMeasurer};
use crate::model::Diagram;
use crate::style::ResolvedStyles;

/// Single MINDMAP layout entry point: dispatches on the diagram's layout mode.
/// HORIZONTAL is the tidy-tree algorithm transposed.
pub(crate) fn layout_mindmap(
    diagram: &Diagram,
    styles: &ResolvedStyles,
    measurer: &dyn TextMeasurer,
) -> Layout {
    let mode = match diagram {
        Diagram::Mindmap(map) => map.layout,
        Diagram::Flow(_) => LayoutMode::Radial,
    };
    match mode {
        LayoutMode::Radial => radial::layout_radial(diagram, styles, measurer),
        LayoutMode::Tree => tree::layout_tree(diagram, styles, measurer, false),
        LayoutMode::Horizontal => tree::layout_tree(diagram, styles, measurer, true),
    }
}
