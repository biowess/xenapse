//! Raw stylesheet -> typed rules with diagnostics (D7). Never panics on input.

use super::model::{
    Arrowhead, Dash, Paint, Property, Rgba, Selector, Target, TextAlign, TypedDeclaration,
    ValidatedRule, ValidatedStylesheet, Value,
};
use super::stylesheet::{
    Declaration, Selector as RawSelector, SelectorPart, SelectorPartKind, Stylesheet, ValueKind,
    ValuePart,
};
use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::source::Span;

/// Validates every rule. Unknown selector: warning, rule skipped. Unsupported
/// property or property invalid for the target: warning, declaration skipped.
/// Invalid value: error, declaration skipped.
pub(crate) fn validate_stylesheet(raw: &Stylesheet) -> (ValidatedStylesheet, Diagnostics) {
    let mut diags = Diagnostics::new();
    let mut rules = Vec::new();
    for (order, rule) in raw.rules.iter().enumerate() {
        let Some((selector, specificity)) = interpret_selector(&rule.selector) else {
            diags.push(
                Diagnostic::warning(
                    codes::CSS_UNKNOWN_SELECTOR,
                    Origin::Stylesheet,
                    rule.selector.span,
                    format!("unknown selector `{}`; rule skipped", selector_text(&rule.selector)),
                )
                .with_help(
                    "valid selectors: `diagram`, `node`, `edge`, `.class`, `#id`, `node.class`, \
                     `edge.class`, and `text` after a node selector (`node text`, `.class text`, `#id text`)",
                ),
            );
            continue;
        };
        let target = selector.target();
        let declarations = rule
            .declarations
            .iter()
            .filter_map(|d| validate_declaration(target, d, &mut diags))
            .collect();
        rules.push(ValidatedRule {
            selector,
            specificity,
            order,
            declarations,
            selector_span: rule.selector.span,
            span: rule.span,
        });
    }
    (ValidatedStylesheet { rules }, diags)
}

fn selector_text(raw: &RawSelector) -> String {
    let mut out = String::new();
    for p in &raw.parts {
        if p.space_before {
            out.push(' ');
        }
        match p.kind {
            SelectorPartKind::Ident => {}
            SelectorPartKind::Class => out.push('.'),
            SelectorPartKind::Id => out.push('#'),
        }
        out.push_str(&p.text);
    }
    out
}

/// Interprets the raw parts per D7; `None` means unknown selector.
fn interpret_selector(raw: &RawSelector) -> Option<(Selector, u32)> {
    use SelectorPartKind as K;
    let mut parts: &[SelectorPart] = &raw.parts;
    let mut text = false;
    if let Some((last, rest)) = parts.split_last() {
        if !rest.is_empty() && last.kind == K::Ident && last.text == "text" && last.space_before {
            text = true;
            parts = rest;
        }
    }
    if parts.iter().any(|p| p.space_before) {
        return None;
    }
    let node = |class: Option<String>, id: Option<String>| Selector::Node { class, id, text };
    match parts {
        [a] if a.kind == K::Ident => match a.text.as_str() {
            "diagram" if !text => Some((Selector::Diagram, 1)),
            "node" => Some((node(None, None), 1)),
            "edge" if !text => Some((Selector::Edge { class: None }, 1)),
            _ => None,
        },
        [a] if a.kind == K::Class => Some((node(Some(a.text.clone()), None), 10)),
        [a] if a.kind == K::Id => Some((node(None, Some(a.text.clone())), 100)),
        [a, b] if a.kind == K::Ident && b.kind == K::Class => match a.text.as_str() {
            "node" => Some((node(Some(b.text.clone()), None), 11)),
            "edge" if !text => Some((
                Selector::Edge {
                    class: Some(b.text.clone()),
                },
                11,
            )),
            _ => None,
        },
        _ => None,
    }
}

fn valid_list(target: Target) -> String {
    Property::valid_for(target)
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn validate_declaration(
    target: Target,
    d: &Declaration,
    diags: &mut Diagnostics,
) -> Option<TypedDeclaration> {
    let Some(property) = Property::from_name(&d.property) else {
        diags.push(
            Diagnostic::warning(
                codes::CSS_UNSUPPORTED_PROPERTY,
                Origin::Stylesheet,
                d.property_span,
                format!("unsupported property `{}`; declaration skipped", d.property),
            )
            .with_help(format!(
                "valid properties for {}: {}",
                target.label(),
                valid_list(target)
            )),
        );
        return None;
    };
    if !property.applies_to(target) {
        diags.push(
            Diagnostic::warning(
                codes::CSS_PROPERTY_TARGET,
                Origin::Stylesheet,
                d.property_span,
                format!(
                    "property `{}` is not valid for {}; declaration skipped",
                    property.name(),
                    target.label()
                ),
            )
            .with_help(format!(
                "valid properties for {}: {}",
                target.label(),
                valid_list(target)
            )),
        );
        return None;
    }
    let whole = match (d.value.first(), d.value.last()) {
        (Some(a), Some(b)) => a.span.merge(b.span),
        _ => d.span,
    };
    match parse_value(property, &d.value, whole) {
        Ok(value) => Some(TypedDeclaration {
            property,
            value,
            span: d.span,
        }),
        Err(bad) => {
            diags.push(
                Diagnostic::error(
                    codes::CSS_INVALID_VALUE,
                    Origin::Stylesheet,
                    bad.span,
                    format!("invalid value for `{}`: {}", property.name(), bad.message),
                )
                .with_help(value_help(property)),
            );
            None
        }
    }
}

struct Bad {
    span: Span,
    message: String,
}

fn bad(span: Span, message: impl Into<String>) -> Bad {
    Bad {
        span,
        message: message.into(),
    }
}

fn value_help(p: Property) -> &'static str {
    match p {
        Property::Fill | Property::Stroke => {
            "valid values: a color (`#RGB`, `#RRGGBB` or `#RRGGBBAA`) or `none`"
        }
        Property::Color => "valid values: a color (`#RGB`, `#RRGGBB` or `#RRGGBBAA`)",
        Property::StrokeWidth => "valid values: a number >= 0, optionally with `px`",
        Property::FontFamily => {
            "valid values: a comma-separated list of quoted names or identifiers, e.g. \"IBM Plex Sans\", Arial, sans-serif"
        }
        Property::FontSize | Property::MinWidth | Property::MaxWidth => {
            "valid values: a length > 0 in px, e.g. `14px`"
        }
        Property::FontWeight => "valid values: `normal`, `bold`, or 100, 200, 300, 400, 500, 600, 700, 800, 900",
        Property::TextAlign => "valid values: `left`, `center`, `right`",
        Property::Padding
        | Property::BorderRadius
        | Property::NodeGap
        | Property::RankGap
        | Property::BranchGap
        | Property::DiagramPadding => "valid values: a length >= 0 in px, e.g. `12px`",
        Property::StrokeDasharray => {
            "valid values: `none`, or a list of numbers >= 0 such as `6 4` or `6, 4`"
        }
        Property::Arrowhead => "valid values: `triangle`, `none`",
    }
}

fn parse_value(p: Property, parts: &[ValuePart], whole: Span) -> Result<Value, Bad> {
    match p {
        Property::Fill | Property::Stroke => paint(one(parts, whole)?).map(Value::Paint),
        Property::Color => color(one(parts, whole)?).map(Value::Color),
        Property::StrokeWidth => length(one(parts, whole)?, Range::NonNegative).map(Value::Number),
        Property::FontSize | Property::MinWidth | Property::MaxWidth => {
            length(one(parts, whole)?, Range::Positive).map(Value::Length)
        }
        Property::Padding
        | Property::BorderRadius
        | Property::NodeGap
        | Property::RankGap
        | Property::BranchGap
        | Property::DiagramPadding => {
            length(one(parts, whole)?, Range::NonNegative).map(Value::Length)
        }
        Property::FontFamily => font_family(parts, whole).map(Value::FontFamily),
        Property::FontWeight => font_weight(one(parts, whole)?).map(Value::FontWeight),
        Property::TextAlign => {
            let part = one(parts, whole)?;
            match ident(part) {
                Some("left") => Ok(Value::TextAlign(TextAlign::Left)),
                Some("center") => Ok(Value::TextAlign(TextAlign::Center)),
                Some("right") => Ok(Value::TextAlign(TextAlign::Right)),
                _ => Err(bad(part.span, "expected `left`, `center` or `right`")),
            }
        }
        Property::Arrowhead => {
            let part = one(parts, whole)?;
            match ident(part) {
                Some("triangle") => Ok(Value::Arrowhead(Arrowhead::Triangle)),
                Some("none") => Ok(Value::Arrowhead(Arrowhead::None)),
                _ => Err(bad(part.span, "expected `triangle` or `none`")),
            }
        }
        Property::StrokeDasharray => dash(parts, whole).map(Value::Dash),
    }
}

fn one(parts: &[ValuePart], whole: Span) -> Result<&ValuePart, Bad> {
    match parts {
        [only] => Ok(only),
        _ => Err(bad(whole, "expected a single value")),
    }
}

fn ident(part: &ValuePart) -> Option<&str> {
    match &part.kind {
        ValueKind::Ident(s) => Some(s.as_str()),
        _ => None,
    }
}

fn paint(part: &ValuePart) -> Result<Paint, Bad> {
    if ident(part) == Some("none") {
        return Ok(Paint::None);
    }
    color(part).map(Paint::Color)
}

fn color(part: &ValuePart) -> Result<Rgba, Bad> {
    let ValueKind::Hex(hex) = &part.kind else {
        return Err(bad(part.span, "expected a color such as `#RRGGBB`"));
    };
    let digits: Option<Vec<u8>> = hex
        .chars()
        .map(|c| c.to_digit(16).and_then(|v| u8::try_from(v).ok()))
        .collect();
    let rgba = match digits.as_deref() {
        Some([r, g, b]) => Rgba {
            r: r * 17,
            g: g * 17,
            b: b * 17,
            a: 255,
        },
        Some([r1, r2, g1, g2, b1, b2]) => Rgba {
            r: r1 * 16 + r2,
            g: g1 * 16 + g2,
            b: b1 * 16 + b2,
            a: 255,
        },
        Some([r1, r2, g1, g2, b1, b2, a1, a2]) => Rgba {
            r: r1 * 16 + r2,
            g: g1 * 16 + g2,
            b: b1 * 16 + b2,
            a: a1 * 16 + a2,
        },
        _ => {
            return Err(bad(
                part.span,
                "colors are `#RGB`, `#RRGGBB` or `#RRGGBBAA` with hexadecimal digits",
            ))
        }
    };
    Ok(rgba)
}

#[derive(Clone, Copy)]
enum Range {
    Positive,
    NonNegative,
}

/// A number with no unit or `px`, within `range`.
fn length(part: &ValuePart, range: Range) -> Result<f64, Bad> {
    let ValueKind::Number { value, unit } = &part.kind else {
        return Err(bad(part.span, "expected a number, optionally with `px`"));
    };
    if let Some(u) = unit.as_deref() {
        if u != "px" {
            return Err(bad(
                part.span,
                format!("unit `{u}` is not supported (only `px`)"),
            ));
        }
    }
    if !value.is_finite() {
        return Err(bad(part.span, "the number is out of range"));
    }
    match range {
        Range::Positive if *value <= 0.0 => Err(bad(part.span, "the value must be greater than 0")),
        Range::NonNegative if *value < 0.0 => Err(bad(part.span, "the value must not be negative")),
        _ => Ok(*value),
    }
}

fn font_family(parts: &[ValuePart], whole: Span) -> Result<Vec<String>, Bad> {
    let mut groups: Vec<Vec<&ValuePart>> = vec![Vec::new()];
    for p in parts {
        if matches!(p.kind, ValueKind::Comma) {
            groups.push(Vec::new());
        } else if let Some(last) = groups.last_mut() {
            last.push(p);
        }
    }
    let mut families = Vec::new();
    for group in groups {
        let first_span = group.first().map_or(whole, |p| p.span);
        let name = match group.as_slice() {
            [] => return Err(bad(whole, "empty entry in the font family list")),
            [single] if matches!(single.kind, ValueKind::Str(_)) => match &single.kind {
                ValueKind::Str(s) => s.trim().to_owned(),
                _ => String::new(),
            },
            many => {
                let words: Option<Vec<&str>> = many.iter().map(|p| ident(p)).collect();
                match words {
                    Some(w) => w.join(" "),
                    None => {
                        return Err(bad(
                            first_span,
                            "a font family is one quoted name or one or more identifiers",
                        ))
                    }
                }
            }
        };
        if name.is_empty() {
            return Err(bad(first_span, "a font family name must not be empty"));
        }
        families.push(name);
    }
    Ok(families)
}

fn font_weight(part: &ValuePart) -> Result<u16, Bad> {
    match &part.kind {
        ValueKind::Ident(s) if s == "normal" => Ok(400),
        ValueKind::Ident(s) if s == "bold" => Ok(700),
        ValueKind::Number { value, unit: None }
            if value.fract() == 0.0 && (100.0..=900.0).contains(value) && value % 100.0 == 0.0 =>
        {
            Ok(*value as u16)
        }
        _ => Err(bad(
            part.span,
            "expected `normal`, `bold` or 100 to 900 in steps of 100",
        )),
    }
}

fn dash(parts: &[ValuePart], whole: Span) -> Result<Dash, Bad> {
    if let [only] = parts {
        if ident(only) == Some("none") {
            return Ok(Dash::None);
        }
    }
    let mut list = Vec::new();
    let mut after_comma = false;
    for p in parts {
        match &p.kind {
            ValueKind::Number { .. } => {
                list.push(length(p, Range::NonNegative)?);
                after_comma = false;
            }
            ValueKind::Comma if !list.is_empty() && !after_comma => after_comma = true,
            _ => return Err(bad(p.span, "expected `none` or a list of numbers")),
        }
    }
    if list.is_empty() || after_comma {
        return Err(bad(whole, "expected `none` or a list of numbers"));
    }
    Ok(Dash::List(list))
}
