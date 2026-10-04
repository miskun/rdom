//! [`Sides`]: one value per box side, the shape of every per-side
//! property family (`border-*-color`, `border-*-width`, …).

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
