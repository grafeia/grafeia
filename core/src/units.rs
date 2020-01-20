use std::ops::{Add, AddAssign, Sub, SubAssign, Mul, Div};
use pathfinder_geometry::{
    rect::RectF,
    vector::Vector2F
};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Default)]
pub struct Length {
    // length in Millimeters
    pub value: f32,
}
impl Length {
    pub fn zero() -> Length {
        Length { value: 0.0 }
    }
    pub fn max(self, other: Length) -> Length {
        Length { value: self.value.max(other.value) }
    }
    pub fn min(self, other: Length) -> Length {
        Length { value: self.value.min(other.value) }
    }
    pub fn mm(value: f32) -> Length {
        Length { value }
    }
    pub fn cm(value: f32) -> Length {
        Length { value: 10. * value }
    }
    pub fn inch(value: f32) -> Length {
        Length { value: 25.4 * value }
    }
}
pub struct Bounds {
    pub top: Length,
    pub bottom: Length,
    pub left: Length,
    pub right: Length
}
impl Bounds {
    pub fn width(&self) -> Length {
        self.right - self.left
    }
    pub fn height(&self) -> Length {
        self.top - self.bottom
    }
}

#[derive(Copy, Clone)]
pub struct Rect {
    pub left: Length,
    pub top: Length,
    pub width: Length,
    pub height: Length
}
impl Into<RectF> for Rect {
    fn into(self) -> RectF {
        let origin = Vector2F::new(self.left.value, self.top.value);
        let size = Vector2F::new(self.width.value, self.height.value);

        RectF::from_points(origin, origin + size)
    }
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Length) -> Length {
        Length { value: self.value + rhs.value }
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Length) -> Length {
        Length { value: self.value - rhs.value }
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Length) {
        self.value += rhs.value;
    }
}
impl<'a> AddAssign<&'a Length> for Length {
    fn add_assign(&mut self, rhs: &'a Length) {
        self.value += rhs.value;
    }
}
impl SubAssign for Length {
    fn sub_assign(&mut self, rhs: Length) {
        self.value -= rhs.value;
    }
}

impl Mul<f32> for Length {
    type Output = Length;
    fn mul(self, rhs: f32) -> Length {
        Length { value: self.value * rhs }
    }
}
impl Div<f32> for Length {
    type Output = Length;
    fn div(self, rhs: f32) -> Length {
        Length { value: self.value / rhs }
    }
}

impl Div for Length {
    type Output = f32;
    fn div(self, rhs: Length) -> f32 {
        self.value / rhs.value
    }
}
