/// The diagram module selected by `IMPORT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    Flow,
    Mindmap,
}

/// FLOW layout direction (`DIRECTION TB|LR`). Default: `TopToBottom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopToBottom,
    LeftToRight,
}

/// MINDMAP layout mode (`LAYOUT RADIAL|TREE|HORIZONTAL`). Default: `Radial`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Radial,
    Tree,
    Horizontal,
}
