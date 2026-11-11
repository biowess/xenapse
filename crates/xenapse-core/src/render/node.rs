//! Node shapes and text (decision D9). Paint is written as inline presentation
//! attributes; geometry comes from the layout.

use super::svg::{fmt_num, SvgWriter};
use crate::geometry::Rect;
use crate::layout::{NodeLayout, ShapeGeom};
use crate::model::{Diagram, NodeId};
use crate::source::Span;
use crate::style::{NodeStyle, Paint, Rgba, TextAlign};

/// Vertical radius of a cylinder's elliptical caps.
fn cylinder_ry(height: f64) -> f64 {
    8.0_f64.min(height / 4.0).max(0.0)
}

fn n(v: f64) -> String {
    fmt_num(v)
}

/// Built-in and user classes of node `i`.
fn classes(diagram: &Diagram, i: usize) -> (&'static [&'static str], &[String]) {
    match diagram {
        Diagram::Flow(f) => match f.nodes.get(i) {
            Some(node) => (node.kind.builtin_classes(), node.classes.as_slice()),
            None => (&[], &[]),
        },
        Diagram::Mindmap(m) => match m.nodes.get(i) {
            Some(node) => (&[], node.classes.as_slice()),
            None => (&[], &[]),
        },
    }
}

pub(super) fn hex(c: Rgba) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

pub(super) fn paint_attr(w: &mut SvgWriter, name: &str, paint: Paint) {
    match paint {
        Paint::None => w.attr(name, "none"),
        Paint::Color(c) => {
            w.attr(name, &hex(c));
            if c.a != 255 {
                w.attr_num(&format!("{name}-opacity"), f64::from(c.a) / 255.0);
            }
        }
    }
}

fn stroke_attrs(w: &mut SvgWriter, style: &NodeStyle) {
    paint_attr(w, "stroke", style.paint.stroke);
    if style.paint.stroke != Paint::None {
        w.attr_num("stroke-width", style.paint.stroke_width);
    }
}

fn shape_paint(w: &mut SvgWriter, style: &NodeStyle) {
    paint_attr(w, "fill", style.paint.fill);
    stroke_attrs(w, style);
}

/// Font family as a CSS list: identifiers bare, everything else quoted.
pub(super) fn css_family(family: &[String]) -> String {
    let parts: Vec<String> = family
        .iter()
        .map(|name| {
            let clean: String = name.chars().filter(|c| !c.is_control()).collect();
            let mut chars = clean.chars();
            let ident = chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && !matches!(
                    clean.to_ascii_lowercase().as_str(),
                    "inherit" | "initial" | "unset" | "revert" | "default"
                );
            if ident {
                clean
            } else {
                format!("'{}'", clean.replace('\\', "\\\\").replace('\'', "\\'"))
            }
        })
        .collect();
    parts.join(", ")
}

/// All classes of node `i`: `xn-node`, built-in classes, then user classes.
pub(super) fn node_classes(diagram: &Diagram, i: usize) -> Vec<String> {
    let (builtin, user) = classes(diagram, i);
    let mut out = vec!["xn-node".to_owned()];
    out.extend(builtin.iter().map(|b| format!("xn-{b}")));
    out.extend(user.iter().cloned());
    out
}

/// `(explicit source id, label, span)` of node `i`.
pub(super) fn node_meta(diagram: &Diagram, i: usize) -> Option<(Option<String>, String, Span)> {
    match diagram {
        Diagram::Flow(f) => f
            .nodes
            .get(i)
            .map(|n| (n.source_id.clone(), n.label.clone(), n.span)),
        Diagram::Mindmap(m) => m
            .nodes
            .get(i)
            .map(|n| (n.source_id.clone(), n.label.clone(), n.span)),
    }
}

pub(super) fn node_is_hidden(diagram: &Diagram, i: usize) -> bool {
    match diagram {
        Diagram::Flow(f) => f.nodes.get(i).is_some_and(|n| n.hidden),
        Diagram::Mindmap(_) => false,
    }
}

pub(super) fn render_node(
    w: &mut SvgWriter,
    diagram: &Diagram,
    styles: &crate::style::ResolvedStyles,
    i: usize,
    nl: &NodeLayout,
) {
    let Some(style) = styles.node(NodeId(i)) else {
        return;
    };
    let Some((source_id, label, span)) = node_meta(diagram, i) else {
        return;
    };
    w.begin("g");
    w.attr("id", &format!("xn-n{i}"));
    w.attr("class", &node_classes(diagram, i).join(" "));
    w.attr("data-xn-start", &span.start.to_string());
    w.attr("data-xn-end", &span.end.to_string());
    if let Some(id) = &source_id {
        w.attr("data-xn-id", id);
    }
    w.open_end();
    w.newline();
    w.begin("title");
    w.open_end();
    w.text(&label);
    w.close("title");
    w.newline();
    render_shape(w, nl, style);
    render_text(w, nl, style);
    w.close("g");
    w.newline();
}

fn corner_radius(style: &NodeStyle, r: &Rect) -> f64 {
    let v = style.paint.border_radius;
    if v.is_finite() && v > 0.0 {
        v.min(r.width / 2.0).min(r.height / 2.0).max(0.0)
    } else {
        0.0
    }
}

fn rect_el(w: &mut SvgWriter, r: &Rect, radius: f64, style: &NodeStyle) {
    w.begin("rect");
    w.attr_num("x", r.x);
    w.attr_num("y", r.y);
    w.attr_num("width", r.width);
    w.attr_num("height", r.height);
    if radius > 0.0 {
        w.attr_num("rx", radius);
        w.attr_num("ry", radius);
    }
    shape_paint(w, style);
    w.empty_end();
    w.newline();
}

fn polygon_el(w: &mut SvgWriter, pts: &[(f64, f64)], style: &NodeStyle) {
    let points: Vec<String> = pts
        .iter()
        .map(|(x, y)| format!("{},{}", n(*x), n(*y)))
        .collect();
    w.begin("polygon");
    w.attr("points", &points.join(" "));
    shape_paint(w, style);
    w.empty_end();
    w.newline();
}

fn path_el(w: &mut SvgWriter, d: &str, fill: Option<&NodeStyle>, stroke: &NodeStyle) {
    w.begin("path");
    w.attr("d", d);
    match fill {
        Some(s) => paint_attr(w, "fill", s.paint.fill),
        None => w.attr("fill", "none"),
    }
    stroke_attrs(w, stroke);
    w.empty_end();
    w.newline();
}

fn render_shape(w: &mut SvgWriter, nl: &NodeLayout, style: &NodeStyle) {
    let r = &nl.rect;
    let (right, bottom, cx, cy) = (
        r.right(),
        r.bottom(),
        r.x + r.width / 2.0,
        r.y + r.height / 2.0,
    );
    match nl.shape {
        ShapeGeom::Terminator => rect_el(w, r, r.width.min(r.height) / 2.0, style),
        ShapeGeom::Rect | ShapeGeom::RoundedRect => rect_el(w, r, corner_radius(style, r), style),
        ShapeGeom::DoubleRect => {
            let radius = corner_radius(style, r);
            rect_el(w, r, radius, style);
            if style.paint.stroke != Paint::None {
                let (_, _, ex, _) = nl.shape.inflation();
                let inset = (ex / 2.0).min(r.width / 4.0);
                // Keep the inner lines inside rounded corners.
                let dy = if radius > inset {
                    radius - (inset * (2.0 * radius - inset)).max(0.0).sqrt()
                } else {
                    0.0
                };
                let (top, bot) = (r.y + dy, bottom - dy);
                let d = format!(
                    "M {} {} V {} M {} {} V {}",
                    n(r.x + inset),
                    n(top),
                    n(bot),
                    n(right - inset),
                    n(top),
                    n(bot)
                );
                path_el(w, &d, None, style);
            }
        }
        ShapeGeom::Ellipse => {
            w.begin("ellipse");
            w.attr_num("cx", cx);
            w.attr_num("cy", cy);
            w.attr_num("rx", r.width / 2.0);
            w.attr_num("ry", r.height / 2.0);
            shape_paint(w, style);
            w.empty_end();
            w.newline();
        }
        ShapeGeom::Diamond => {
            polygon_el(w, &[(cx, r.y), (right, cy), (cx, bottom), (r.x, cy)], style)
        }
        ShapeGeom::Parallelogram => {
            let (_, _, ex, _) = nl.shape.inflation();
            let s = ex.min(r.width / 3.0);
            polygon_el(
                w,
                &[
                    (r.x + s, r.y),
                    (right, r.y),
                    (right - s, bottom),
                    (r.x, bottom),
                ],
                style,
            );
        }
        ShapeGeom::Cylinder => {
            let (rx, ry) = (r.width / 2.0, cylinder_ry(r.height));
            let arc = |sweep: u8, x: f64, y: f64| {
                format!("A {} {} 0 0 {} {} {}", n(rx), n(ry), sweep, n(x), n(y))
            };
            let body = format!(
                "M {} {} {} V {} {} Z",
                n(r.x),
                n(r.y + ry),
                arc(1, right, r.y + ry),
                n(bottom - ry),
                arc(1, r.x, bottom - ry)
            );
            path_el(w, &body, Some(style), style);
            if style.paint.stroke != Paint::None {
                let rim = format!("M {} {} {}", n(r.x), n(r.y + ry), arc(0, right, r.y + ry));
                path_el(w, &rim, None, style);
            }
        }
    }
}

/// Fraction of the font size from a line's vertical centre down to its baseline.
const BASELINE_SHIFT: f64 = 0.35;

fn render_text(w: &mut SvgWriter, nl: &NodeLayout, style: &NodeStyle) {
    if nl.lines.is_empty() {
        return;
    }
    let r = &nl.rect;
    let font = &style.text.layout.font;
    let pad = if style.layout.padding.is_finite() {
        style.layout.padding.max(0.0)
    } else {
        0.0
    };
    let (kx, _, ex, _) = nl.shape.inflation();
    // Width of the text area the layout reserved inside this shape.
    let area_w = (((r.width - ex) / kx) - 2.0 * pad).max(0.0);
    let cx = r.x + r.width / 2.0;
    let (anchor, x) = match style.text.paint.align {
        TextAlign::Left => ("start", cx - area_w / 2.0),
        TextAlign::Center => ("middle", cx),
        TextAlign::Right => ("end", cx + area_w / 2.0),
    };
    let cap_shift = if nl.shape == ShapeGeom::Cylinder {
        cylinder_ry(r.height) / 2.0
    } else {
        0.0
    };
    let block_h = nl.line_height * nl.lines.len() as f64;
    let top = r.y + r.height / 2.0 + cap_shift - block_h / 2.0;

    w.begin("text");
    let family = css_family(&font.family);
    if !family.is_empty() {
        w.attr("font-family", &family);
    }
    w.attr_num("font-size", font.size);
    w.attr("font-weight", &font.weight.to_string());
    paint_attr(w, "fill", Paint::Color(style.text.paint.color));
    w.attr("text-anchor", anchor);
    w.open_end();
    for (j, line) in nl.lines.iter().enumerate() {
        let y = top + (j as f64 + 0.5) * nl.line_height + BASELINE_SHIFT * font.size;
        w.begin("tspan");
        w.attr_num("x", x);
        w.attr_num("y", y);
        w.open_end();
        w.text(line);
        w.close("tspan");
    }
    w.close("text");
    w.newline();
}
