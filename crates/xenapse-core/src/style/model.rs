//! Typed, validated XCSS model (decision D7). Produced by
//! `validate_stylesheet`; consumed by style resolution.

use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Paint {
    None,
    Color(Rgba),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrowhead {
    Triangle,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Dash {
    None,
    List(Vec<f64>),
}

/// What a selector styles. `NodeText` is a node selector with a `text` suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Diagram,
    Node,
    NodeText,
    Edge,
}

impl Target {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Target::Diagram => "`diagram`",
            Target::Node => "nodes",
            Target::NodeText => "node text",
            Target::Edge => "edges",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selector {
    Diagram,
    Node {
        class: Option<String>,
        id: Option<String>,
        text: bool,
    },
    Edge {
        class: Option<String>,
    },
}

impl Selector {
    pub(crate) fn target(&self) -> Target {
        match self {
            Selector::Diagram => Target::Diagram,
            Selector::Node { text: false, .. } => Target::Node,
            Selector::Node { text: true, .. } => Target::NodeText,
            Selector::Edge { .. } => Target::Edge,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Property {
    Fill,
    Stroke,
    StrokeWidth,
    Color,
    FontFamily,
    FontSize,
    FontWeight,
    TextAlign,
    Padding,
    MinWidth,
    MaxWidth,
    BorderRadius,
    StrokeDasharray,
    Arrowhead,
    NodeGap,
    RankGap,
    BranchGap,
    DiagramPadding,
}

impl Property {
    pub(crate) const ALL: [Property; 18] = [
        Property::Fill,
        Property::Stroke,
        Property::StrokeWidth,
        Property::Color,
        Property::FontFamily,
        Property::FontSize,
        Property::FontWeight,
        Property::TextAlign,
        Property::Padding,
        Property::MinWidth,
        Property::MaxWidth,
        Property::BorderRadius,
        Property::StrokeDasharray,
        Property::Arrowhead,
        Property::NodeGap,
        Property::RankGap,
        Property::BranchGap,
        Property::DiagramPadding,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Property::Fill => "fill",
            Property::Stroke => "stroke",
            Property::StrokeWidth => "stroke-width",
            Property::Color => "color",
            Property::FontFamily => "font-family",
            Property::FontSize => "font-size",
            Property::FontWeight => "font-weight",
            Property::TextAlign => "text-align",
            Property::Padding => "padding",
            Property::MinWidth => "min-width",
            Property::MaxWidth => "max-width",
            Property::BorderRadius => "border-radius",
            Property::StrokeDasharray => "stroke-dasharray",
            Property::Arrowhead => "arrowhead",
            Property::NodeGap => "node-gap",
            Property::RankGap => "rank-gap",
            Property::BranchGap => "branch-gap",
            Property::DiagramPadding => "diagram-padding",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Property> {
        Property::ALL.iter().copied().find(|p| p.name() == name)
    }

    /// The properties allowed for a target, in a fixed order.
    pub(crate) fn valid_for(target: Target) -> &'static [Property] {
        match target {
            Target::Node => &[
                Property::Fill,
                Property::Stroke,
                Property::StrokeWidth,
                Property::Color,
                Property::FontFamily,
                Property::FontSize,
                Property::FontWeight,
                Property::TextAlign,
                Property::Padding,
                Property::MinWidth,
                Property::MaxWidth,
                Property::BorderRadius,
            ],
            Target::NodeText => &[
                Property::Color,
                Property::TextAlign,
                Property::FontFamily,
                Property::FontSize,
                Property::FontWeight,
            ],
            Target::Edge => &[
                Property::Stroke,
                Property::StrokeWidth,
                Property::StrokeDasharray,
                Property::Arrowhead,
                Property::FontFamily,
                Property::FontSize,
                Property::Color,
            ],
            Target::Diagram => &[
                Property::NodeGap,
                Property::RankGap,
                Property::BranchGap,
                Property::DiagramPadding,
            ],
        }
    }

    pub(crate) fn applies_to(self, target: Target) -> bool {
        Property::valid_for(target).contains(&self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Paint(Paint),
    Color(Rgba),
    /// A length in px.
    Length(f64),
    Number(f64),
    FontFamily(Vec<String>),
    FontWeight(u16),
    TextAlign(TextAlign),
    Arrowhead(Arrowhead),
    Dash(Dash),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypedDeclaration {
    pub property: Property,
    pub value: Value,
    /// Property through the last value part, in stylesheet bytes.
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValidatedRule {
    pub selector: Selector,
    /// type 1, class 10, id 100, summed (D7).
    pub specificity: u32,
    /// Index of the rule in the raw stylesheet; later wins ties.
    pub order: usize,
    pub declarations: Vec<TypedDeclaration>,
    pub selector_span: Span,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ValidatedStylesheet {
    pub rules: Vec<ValidatedRule>,
}
