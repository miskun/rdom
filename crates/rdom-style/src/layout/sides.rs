//! [`Sides`] and [`Corners`]: one value per box side or corner, the
//! shape of every per-side and per-corner property family
//! (`border-*-color`, `border-*-width`, `border-*-radius`, …).

/// One value per side of a box, in CSS order: top, right, bottom, left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Sides<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T> Sides<T> {
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// `v` on every side.
    pub fn all(v: T) -> Self
    where
        T: Clone,
    {
        Self::new(v.clone(), v.clone(), v.clone(), v)
    }

    /// 1 to 4 values expanded clockwise from the top (CSS Backgrounds 3
    /// §4.1, Box 3 §3.2): one is every side, two are vertical then
    /// horizontal, three are top, horizontal, bottom.
    pub fn from_values(values: &[T]) -> Option<Self>
    where
        T: Clone,
    {
        Some(match values {
            [a] => Self::all(a.clone()),
            [a, b] => Self::new(a.clone(), b.clone(), a.clone(), b.clone()),
            [a, b, c] => Self::new(a.clone(), b.clone(), c.clone(), b.clone()),
            [a, b, c, d] => Self::new(a.clone(), b.clone(), c.clone(), d.clone()),
            _ => return None,
        })
    }

    /// The sides as an array, top first.
    pub fn to_array(self) -> [T; 4] {
        [self.top, self.right, self.bottom, self.left]
    }

    /// References to the sides, top first.
    pub fn each(&self) -> [&T; 4] {
        [&self.top, &self.right, &self.bottom, &self.left]
    }

    /// Mutable references to the sides, top first.
    pub fn each_mut(&mut self) -> [&mut T; 4] {
        [
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
            &mut self.left,
        ]
    }

    /// `f` of every side.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Sides<U> {
        Sides::new(f(self.top), f(self.right), f(self.bottom), f(self.left))
    }

    /// The sides of `self` and `other` paired.
    pub fn zip<U>(self, other: Sides<U>) -> Sides<(T, U)> {
        Sides::new(
            (self.top, other.top),
            (self.right, other.right),
            (self.bottom, other.bottom),
            (self.left, other.left),
        )
    }

    /// True when every side equals the top.
    pub fn uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.right == self.top && self.bottom == self.top && self.left == self.top
    }
}

/// One value per corner of a box, in CSS order: top-left, top-right,
/// bottom-right, bottom-left (CSS Backgrounds 3 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T> Corners<T> {
    pub const fn new(top_left: T, top_right: T, bottom_right: T, bottom_left: T) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// `v` on every corner.
    pub fn all(v: T) -> Self
    where
        T: Clone,
    {
        Self::new(v.clone(), v.clone(), v.clone(), v)
    }

    /// 1 to 4 values expanded clockwise from the top-left (§5.2): one is
    /// every corner, two are top-left / bottom-right then top-right /
    /// bottom-left, three are top-left, the top-right / bottom-left
    /// pair, bottom-right.
    pub fn from_values(values: &[T]) -> Option<Self>
    where
        T: Clone,
    {
        let s = Sides::from_values(values)?;
        Some(Self::new(s.top, s.right, s.bottom, s.left))
    }

    /// The corners as an array, top-left first.
    pub fn to_array(self) -> [T; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }

    /// References to the corners, top-left first.
    pub fn each(&self) -> [&T; 4] {
        [
            &self.top_left,
            &self.top_right,
            &self.bottom_right,
            &self.bottom_left,
        ]
    }

    /// Mutable references to the corners, top-left first.
    pub fn each_mut(&mut self) -> [&mut T; 4] {
        [
            &mut self.top_left,
            &mut self.top_right,
            &mut self.bottom_right,
            &mut self.bottom_left,
        ]
    }

    /// `f` of every corner.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Corners<U> {
        Corners::new(
            f(self.top_left),
            f(self.top_right),
            f(self.bottom_right),
            f(self.bottom_left),
        )
    }

    /// The corners of `self` and `other` paired.
    pub fn zip<U>(self, other: Corners<U>) -> Corners<(T, U)> {
        Corners::new(
            (self.top_left, other.top_left),
            (self.top_right, other.top_right),
            (self.bottom_right, other.bottom_right),
            (self.bottom_left, other.bottom_left),
        )
    }
}
