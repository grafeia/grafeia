use std::ops::{BitOr, BitOrAssign};
use std::fmt;
use super::FlexMeasure;

#[derive(Copy, Clone, Debug)]
pub enum Glue {
    None,
    Space {
        breaking:   bool,
        measure:    FlexMeasure
    },
    Newline {
        fill:       bool
    }
}

impl BitOr for Glue {
    type Output = Glue;
    
    fn bitor(self, rhs: Glue) -> Glue {
        use self::Glue::*;
        
        match (self, rhs) {
            // Glue::None wins over anything else
            (None, _) | (_, None) => None,
            
            (Space { breaking: false, .. }, Newline { .. }) |
            (Newline { .. }, Space { breaking: false, .. }) => {
                panic!("Newline and NonBreaking requested");
            },
            
            // NonBreaking wins over Breaking
            (Space { breaking: false, measure: a }, Space { breaking: true,  measure: b }) |
            (Space { breaking: true,  measure: a }, Space { breaking: false, measure: b })
             => Space { breaking: false, measure: a.max(b) },
            
            // Newline wins over Breaking
            (Newline { fill: a }, Space { breaking: true, .. }) |
            (Space { breaking: true, .. }, Newline { fill: a })
             => Newline { fill: a },
            
            (Space { breaking: true, measure: a }, Space { breaking: true,  measure: b })
             => Space { breaking: true, measure: a.max(b) },
             
            (Space { breaking: false, measure: a }, Space { breaking: false,  measure: b })
             => Space { breaking: false, measure: a.max(b) },
             
            (Newline { fill: a }, Newline { fill: b })
             => Newline { fill: a | b }
        }
    }
}
impl BitOrAssign for Glue {
    fn bitor_assign(&mut self, rhs: Glue) {
        *self = *self | rhs;
    }
}

impl Glue {
    pub fn space(measure: FlexMeasure) -> Glue {
        Glue::Space { breaking: true, measure }
    }
    pub fn nbspace(measure: FlexMeasure) -> Glue {
        Glue::Space { breaking: false, measure }
    }
    pub fn newline() -> Glue {
        Glue::Newline { fill: false }
    }
    pub fn hfill() -> Glue {
        Glue::Newline { fill: true }
    }
    pub fn any() -> Glue {
        Glue::Space { breaking: true, measure: FlexMeasure::zero() }
    }
}

impl fmt::Display for Glue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Glue::None => Ok(()),
            Glue::Space { breaking: true, .. } => write!(f, "␣"),
            Glue::Space { breaking: false, .. } => write!(f, "~"),
            Glue::Newline { fill: _ } => write!(f, "␤")
        }
    }
}

