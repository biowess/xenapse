//! Edge paths, arrowheads and labels (decision D9). The arrowhead is a filled
//! `<path>` polygon, never a `<marker>`.

use super::node::{css_family, paint_attr};
use super::svg::{fmt_num, SvgWriter};
use crate::geometry::{Point, Rect};
use crate::layout::{EdgeLayout, TextAnchor};
use crate::model::{Diagram, EdgeId};
use crate::source::Span;
use crate::style::{Arrowhead, Dash};
use crate::style::{EdgeStyle, Paint, ResolvedStyles};

/// Fraction of the font size from a line's vertical centre down to its baseline.
const BASELINE_SHIFT: f64 = 0.35;
/// Line spacing of multi-line edge labels, as a multiple of the font size.
const LABEL_LINE: f64 = 1.2;

/// `(span, built-in class, label)` of edge `i`.
pub(super) fn edge_meta(
    diagram: &Diagram,
    i: usize,
) -> Option<(Span, Option<&'static str>, Option<String>)> {
    match diagram {
        Diagram::Flow(f) => f
            .edges
            .get(i)
            .map(|e| (e.span, e.kind.builtin_class(), e.label.clone())),
        Diagram::Mindmap(m) => m.edge_span(i).map(|s| (s, None, None)),
    }
}

pub(super) fn edge_is_hidden(diagram: &Diagram, i: usize) -> bool {
    match diagram {
        Diagram::Flow(f) => {
            let Some(e) = f.edges.get(i) else {
                return false;
            };
            f.nodes.get(e.from.0).is_some_and(|n| n.hidden)
                || f.nodes.get(e.to.0).is_some_and(|n| n.hidden)
        }
        Diagram::Mindmap(_) => false,
    }
}

/// Bounding box of the route points (zero box when the route is empty).
pub(super) fn edge_bbox(el: &EdgeLayout) -> Rect {
    let mut it = el
        .points
        .iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite());
    let Some(first) = it.next() else {
        return Rect::default();
    };
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for p in it {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

fn pt(p: &Point) -> String {
    format!("{} {}", fmt_num(p.x), fmt_num(p.y))
}

/// Unit direction of the last non-degenerate segment and that segment's length.
fn last_direction(points: &[Point]) -> Option<(f64, f64, f64)> {
    let tip = points.last()?;
    for p in points.iter().rev().skip(1) {
        let (dx, dy) = (tip.x - p.x, tip.y - p.y);
        let len = dx.hypot(dy);
        if len.is_finite() && len > 1e-6 {
            return Some((dx / len, dy / len, len));
        }
    }
    None
}

pub(super) fn render_edge(
    w: &mut SvgWriter,
    diagram: &Diagram,
    styles: &ResolvedStyles,
    i: usize,
    el: &EdgeLayout,
) {
    let Some(style) = styles.edge(EdgeId(i)) else {
        return;
    };
    let Some((span, builtin, _)) = edge_meta(diagram, i) else {
        return;
    };
    let mut class = String::from("xn-edge");
    if let Some(b) = builtin {
        class.push_str(" xn-");
        class.push_str(b);
    }
    w.begin("g");
    w.attr("id", &format!("xn-e{i}"));
    w.attr("class", &class);
    w.attr("data-xn-start", &span.start.to_string());
    w.attr("data-xn-end", &span.end.to_string());
    w.open_end();
    w.newline();
    render_path(w, style, &el.points);
    render_label(w, style, el);
    w.close("g");
    w.newline();
}

fn render_path(w: &mut SvgWriter, style: &EdgeStyle, points: &[Point]) {
    let pts: Vec<Point> = points
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    if pts.len() < 2 {
        return;
    }
    let paint = &style.paint;
    let head = match (paint.arrowhead, paint.stroke, last_direction(&pts)) {
        (Arrowhead::Triangle, Paint::Color(c), Some((ux, uy, seg))) => {
            let len = 6.0 + 2.0 * paint.stroke_width.clamp(0.0, 8.0);
            Some((c, ux, uy, len, seg))
        }
        _ => None,
    };

    // Stop the line at the arrow base so a thick stroke does not poke past the tip.
    let mut line = pts.clone();
    if let (Some((_, ux, uy, len, seg)), Some(tip)) = (head, pts.last()) {
        if seg > len {
            if let Some(last) = line.last_mut() {
                *last = Point::new(tip.x - ux * len, tip.y - uy * len);
            }
        }
    }
    let mut d = String::new();
    for (k, p) in line.iter().enumerate() {
        d.push_str(if k == 0 { "M " } else { " L " });
        d.push_str(&pt(p));
    }
    w.begin("path");
    w.attr("d", &d);
    w.attr("fill", "none");
    paint_attr(w, "stroke", paint.stroke);
    if paint.stroke != Paint::None {
        w.attr_num("stroke-width", paint.stroke_width);
        if let Dash::List(list) = &paint.dash {
            let dash: Vec<String> = list.iter().map(|v| fmt_num(*v)).collect();
            if !dash.is_empty() {
                w.attr("stroke-dasharray", &dash.join(" "));
            }
        }
    }
    w.empty_end();
    w.newline();

    if let (Some((color, ux, uy, len, _)), Some(tip)) = (head, pts.last()) {
        let half = len * 0.4;
        let (bx, by) = (tip.x - ux * len, tip.y - uy * len);
        let (nx, ny) = (-uy, ux);
        let a = Point::new(bx + nx * half, by + ny * half);
        let b = Point::new(bx - nx * half, by - ny * half);
        w.begin("path");
        w.attr("d", &format!("M {} L {} L {} Z", pt(tip), pt(&a), pt(&b)));
        paint_attr(w, "fill", Paint::Color(color));
        w.attr("stroke", "none");
        w.empty_end();
        w.newline();
    }
}

fn render_label(w: &mut SvgWriter, style: &EdgeStyle, el: &EdgeLayout) {
    let Some(label) = &el.label else {
        return;
    };
    let lines: Vec<&str> = label.text.split('\n').collect();
    let size = style.layout.font_size;
    let family = css_family(&style.layout.font_family);
    let anchor = match label.anchor {
        TextAnchor::Middle => "middle",
    };
    let step = size * LABEL_LINE;
    let first = label.position.y - step * (lines.len() as f64 - 1.0) / 2.0 + BASELINE_SHIFT * size;

    w.begin("text");
    if !family.is_empty() {
        w.attr("font-family", &family);
    }
    w.attr_num("font-size", size);
    paint_attr(w, "fill", Paint::Color(style.paint.color));
    w.attr("text-anchor", anchor);
    w.open_end();
    for (j, line) in lines.iter().enumerate() {
        w.begin("tspan");
        w.attr_num("x", label.position.x);
        w.attr_num("y", first + step * j as f64);
        w.open_end();
        w.text(line);
        w.close("tspan");
    }
    w.close("text");
    w.newline();
}
