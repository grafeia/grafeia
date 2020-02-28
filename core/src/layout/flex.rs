use std::ops::*;
use crate::units::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlexMeasure {
    pub shrink:     Length,
    pub stretch:    Length,
    pub length:     Length,
}

impl FlexMeasure {
    pub fn fixed(length: Length) -> FlexMeasure {
        FlexMeasure {
            shrink:  length,
            stretch: length,
            length:  length,
        }
    }

    /// factor = -1 => self.shrink,
    /// factor =  0 => self.width,
    /// factor = +1 => self.stretch
    pub fn at(&self, factor: f32) -> Length {
        let delta = match factor < 0. {
            false => self.stretch - self.length,
            true => self.length - self.shrink,
        };
        delta * factor + self.length
    }
    
    /// calculate the factor that yields the given length.
    /// Or None when there is none.
    // m.at(m.factor(w).unwrap()) == w
    pub fn factor(&self, length: Length) -> Option<f32> {
        debug_assert!(self.shrink <= self.length);
        debug_assert!(self.length <= self.stretch);
        if length < self.shrink || length > self.stretch {
            return None;
        }
        
        if length == self.length {
            Some(0.0)
        } else {
            let delta = length - self.length; // d > 0 => stretch, d < 0 => shrink
            if delta >= Length::zero() {
                if self.stretch > self.length {
                    Some(delta / (self.stretch - self.length))
                } else {
                    Some(1.0)
                }
            } else {
                if self.shrink < self.length {
                    Some(delta / (self.length - self.shrink))
                } else {
                    Some(-1.0)
                }
            }
        }
    }
    
    pub fn extend(&mut self, length: Length) {
        if length > self.length {
            self.length = length;
        }
        if length > self.stretch {
            self.stretch = length;
        }
    }
}
impl Add for FlexMeasure {
    type Output = FlexMeasure;
    
    fn add(self, rhs: FlexMeasure) -> FlexMeasure {
        FlexMeasure {
            length:  self.length  + rhs.length,
            stretch: self.stretch + rhs.stretch,
            shrink:  self.shrink  + rhs.shrink,
        }
    }
}
impl Sub for FlexMeasure {
    type Output = FlexMeasure;
    
    fn sub(self, rhs: FlexMeasure) -> FlexMeasure {
        FlexMeasure {
            length:  self.length  - rhs.length,
            stretch: self.stretch - rhs.stretch,
            shrink:  self.shrink  - rhs.shrink,
        }
    }
}
impl FlexMeasure {
    pub fn zero() -> FlexMeasure {
        FlexMeasure {
            length:  Length::zero(),
            stretch: Length::zero(),
            shrink:  Length::zero(),
        }
    }
    pub fn max(self, other: FlexMeasure) -> FlexMeasure {
        FlexMeasure {
            length:  self.length .max(other.length),
            stretch: self.stretch.max(other.stretch),
            shrink:  self.shrink .max(other.shrink),
        }
    }
}
impl Default for FlexMeasure {
    fn default() -> FlexMeasure {
        FlexMeasure::zero()
    }
}
impl AddAssign for FlexMeasure {
    fn add_assign(&mut self, rhs: FlexMeasure) {
        self.length  += rhs.length;
        self.stretch += rhs.stretch;
        self.shrink  += rhs.shrink;
    }
}
impl SubAssign for FlexMeasure {
    fn sub_assign(&mut self, rhs: FlexMeasure) {
        self.length  -= rhs.length;
        self.stretch -= rhs.stretch;
        self.shrink  -= rhs.shrink;
    }
}
impl Mul<f32> for FlexMeasure {
    type Output = FlexMeasure;
    
    fn mul(self, f: f32) -> FlexMeasure {
        FlexMeasure {
            length:     self.length * f,
            stretch:    self.stretch * f,
            shrink:     self.shrink * f,
        }
    }
}