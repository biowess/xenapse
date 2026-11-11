/// A half-open byte range `[start, end)` into a source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Creates a span; if `end < start` the bounds are swapped.
    pub fn new(start: usize, end: usize) -> Span {
        if end < start {
            Span {
                start: end,
                end: start,
            }
        } else {
            Span { start, end }
        }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Smallest span covering both `self` and `other`.
    pub fn merge(self, other: Span) -> Span {
        Span::new(
            self.start.min(other.start).min(self.end).min(other.end),
            self.start.max(other.start).max(self.end).max(other.end),
        )
    }

    /// The text covered by this span, or `None` if it is out of range,
    /// inverted, or not on character boundaries.
    pub fn slice<'a>(&self, src: &'a str) -> Option<&'a str> {
        src.get(self.start..self.end)
    }
}

/// A 1-based line and column; the column counts Unicode scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Maps byte offsets to line/column positions.
#[derive(Debug, Clone)]
pub struct LineIndex {
    text: String,
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Lines are split on `'\n'` only; a preceding `'\r'` is ordinary text.
    pub fn new(src: &str) -> LineIndex {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        LineIndex {
            text: src.to_owned(),
            line_starts,
        }
    }

    /// Position of a byte offset. Offsets past the end clamp to the end;
    /// offsets inside a multi-byte character snap back to its start.
    pub fn position(&self, offset: usize) -> Position {
        let mut off = offset.min(self.text.len());
        while off > 0 && !self.text.is_char_boundary(off) {
            off -= 1;
        }
        let idx = self
            .line_starts
            .partition_point(|&s| s <= off)
            .saturating_sub(1);
        let line_start = self.line_starts.get(idx).copied().unwrap_or(0);
        let column = self
            .text
            .get(line_start..off)
            .map_or(0, |s| s.chars().count());
        Position {
            line: idx + 1,
            column: column + 1,
        }
    }
}
