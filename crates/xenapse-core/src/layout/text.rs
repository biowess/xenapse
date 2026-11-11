//! Text measurement boundary (blueprint §8.3, decision D8) and word wrapping.

/// Font properties that affect text metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpec {
    pub family: Vec<String>,
    pub size: f64,
    pub weight: u16,
}

/// Supplies text metrics to the layout engine. Hosts may implement this with
/// real font metrics; `ApproxMeasurer` is the deterministic built-in.
pub trait TextMeasurer {
    /// Width of `text` on a single line.
    fn text_width(&self, text: &str, font: &FontSpec) -> f64;
    /// Height of one line.
    fn line_height(&self, font: &FontSpec) -> f64;
}

/// Deterministic per-character-class width table. Needs no font data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApproxMeasurer;

const NARROW: f64 = 0.32;
const WIDE: f64 = 0.86;
const DIGIT: f64 = 0.56;
const UPPER: f64 = 0.68;
const OTHER: f64 = 0.54;
const BOLD_FACTOR: f64 = 1.08;
const LINE_HEIGHT_FACTOR: f64 = 1.25;

fn is_cjk_or_fullwidth(u: u32) -> bool {
    matches!(u,
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA960..=0xA97F
        | 0xAC00..=0xD7AF
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF
        | 0x20000..=0x3FFFD)
}

fn is_zero_width(u: u32) -> bool {
    matches!(u, 0x0300..=0x036F | 0x200B..=0x200F | 0x20D0..=0x20FF | 0xFE00..=0xFE0F)
}

/// Width of one character in ems.
fn char_em(c: char) -> f64 {
    let u = c as u32;
    if c.is_control() || is_zero_width(u) {
        return 0.0;
    }
    if is_cjk_or_fullwidth(u) {
        return 1.0;
    }
    match c {
        ' ' | 'i' | 'j' | 'l' | 'I' | '!' | '|' | '\'' | '"' | '.' | ',' | ':' | ';' | '`'
        | 't' | 'f' | 'r' | '(' | ')' | '[' | ']' | '{' | '}' | '-' | '/' | '\\' => NARROW,
        'm' | 'w' | 'M' | 'W' | '@' | '%' | '&' => WIDE,
        _ if c.is_ascii_digit() => DIGIT,
        _ if c.is_uppercase() => UPPER,
        _ => OTHER,
    }
}

fn sane_size(size: f64) -> f64 {
    if size.is_finite() && size > 0.0 {
        size
    } else {
        0.0
    }
}

impl TextMeasurer for ApproxMeasurer {
    fn text_width(&self, text: &str, font: &FontSpec) -> f64 {
        let ems: f64 = text.chars().map(char_em).sum();
        let bold = if font.weight >= 600 { BOLD_FACTOR } else { 1.0 };
        ems * sane_size(font.size) * bold
    }

    fn line_height(&self, font: &FontSpec) -> f64 {
        sane_size(font.size) * LINE_HEIGHT_FACTOR
    }
}

/// Wraps `text` into lines no wider than `max_width` where possible.
/// Explicit `\n` always starts a new line (empty lines are kept); words are
/// separated by whitespace runs and greedily packed; a word wider than
/// `max_width` is broken by character, with at least one character per line.
/// Never returns an empty vector.
pub(crate) fn wrap_text(
    text: &str,
    max_width: f64,
    measurer: &dyn TextMeasurer,
    font: &FontSpec,
) -> Vec<String> {
    let max = if max_width.is_nan() {
        f64::INFINITY
    } else {
        max_width.max(0.0)
    };
    let space_w = measurer.text_width(" ", font);
    let mut lines: Vec<String> = Vec::new();

    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut line_w = 0.0_f64;
        let mut started = false;

        for word in paragraph.split_whitespace() {
            let word_w = measurer.text_width(word, font);
            if started && line_w + space_w + word_w <= max {
                line.push(' ');
                line.push_str(word);
                line_w += space_w + word_w;
                continue;
            }
            if started {
                lines.push(std::mem::take(&mut line));
            }
            started = true;
            if word_w <= max {
                line.push_str(word);
                line_w = word_w;
                continue;
            }
            // Over-long word: break by character.
            let mut buf = String::new();
            let mut buf_w = 0.0_f64;
            let mut buf_chars = 0usize;
            let mut tmp = [0u8; 4];
            for ch in word.chars() {
                let cw = measurer.text_width(ch.encode_utf8(&mut tmp), font);
                if buf_chars > 0 && buf_w + cw > max {
                    lines.push(std::mem::take(&mut buf));
                    buf_w = 0.0;
                    buf_chars = 0;
                }
                buf.push(ch);
                buf_w += cw;
                buf_chars += 1;
            }
            line = buf;
            line_w = buf_w;
        }
        lines.push(line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
