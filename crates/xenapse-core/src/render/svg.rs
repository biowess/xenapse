//! Minimal SVG string writer: XML escaping and number formatting (decision D9).

/// `true` for characters allowed by XML 1.0.
fn xml_legal(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

fn escape_into(out: &mut String, s: &str, attr: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' if attr => out.push_str("&#10;"),
            '\r' if attr => out.push_str("&#13;"),
            '\t' if attr => out.push_str("&#9;"),
            c if xml_legal(c) => out.push(c),
            _ => {}
        }
    }
}

/// Escapes element text; drops characters that are illegal in XML 1.0.
pub(crate) fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(&mut out, s, false);
    out
}

/// Escapes an attribute value; drops characters that are illegal in XML 1.0.
pub(crate) fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(&mut out, s, true);
    out
}

/// Formats a number with at most 2 decimals, trailing zeros trimmed, never
/// `-0`; non-finite values become `0`.
pub(crate) fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_owned();
    }
    let mut s = format!("{v:.2}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_owned();
    }
    s
}

/// `v` rounded exactly as [`fmt_num`] prints it.
pub(crate) fn round_num(v: f64) -> f64 {
    fmt_num(v).parse().unwrap_or(0.0)
}

/// Append-only writer; callers build tags with `begin` / `attr*` / `end_*`.
#[derive(Debug, Default)]
pub(crate) struct SvgWriter {
    buf: String,
}

impl SvgWriter {
    pub(crate) fn new() -> SvgWriter {
        SvgWriter::default()
    }

    pub(crate) fn begin(&mut self, name: &str) {
        self.buf.push('<');
        self.buf.push_str(name);
    }

    pub(crate) fn attr(&mut self, name: &str, value: &str) {
        self.buf.push(' ');
        self.buf.push_str(name);
        self.buf.push_str("=\"");
        self.buf.push_str(&escape_attr(value));
        self.buf.push('"');
    }

    pub(crate) fn attr_num(&mut self, name: &str, value: f64) {
        self.attr(name, &fmt_num(value));
    }

    /// Ends an open start tag (`>`).
    pub(crate) fn open_end(&mut self) {
        self.buf.push('>');
    }

    /// Ends a start tag as self-closing (`/>`).
    pub(crate) fn empty_end(&mut self) {
        self.buf.push_str("/>");
    }

    pub(crate) fn close(&mut self, name: &str) {
        self.buf.push_str("</");
        self.buf.push_str(name);
        self.buf.push('>');
    }

    pub(crate) fn text(&mut self, s: &str) {
        self.buf.push_str(&escape_text(s));
    }

    pub(crate) fn newline(&mut self) {
        self.buf.push('\n');
    }

    pub(crate) fn finish(self) -> String {
        self.buf
    }
}
