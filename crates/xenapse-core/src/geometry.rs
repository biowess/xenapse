//! Plain geometry types. All fields are `f64` in SVG user units.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Point {
    pub(crate) fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

impl Size {
    pub(crate) fn new(width: f64, height: f64) -> Size {
        Size { width, height }
    }
}

impl Rect {
    pub(crate) fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn right(&self) -> f64 {
        self.x + self.width
    }

    pub(crate) fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub(crate) fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub(crate) fn top_mid(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y)
    }

    pub(crate) fn bottom_mid(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.bottom())
    }

    pub(crate) fn left_mid(&self) -> Point {
        Point::new(self.x, self.y + self.height / 2.0)
    }

    pub(crate) fn right_mid(&self) -> Point {
        Point::new(self.right(), self.y + self.height / 2.0)
    }
}
