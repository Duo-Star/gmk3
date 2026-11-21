// src/pakoo_forest/geo/d_x_line.rs

// 一对交叉直线

use std::fmt;
use crate::pakoo_forest::geo::x_line::XLine;

pub struct DXLine {
    xlm: XLine,
    xln: XLine,
}


impl DXLine {
    #[inline]
    pub const fn new(xlm_: XLine, xln_: XLine) -> DXLine {
        DXLine {
            xlm: xlm_,
            xln: xln_,
        }
    }
    
    
}




impl fmt::Display for DXLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DXLine:({} ~ {})",
            self.xlm,
            self.xln,
        )
    }
}

impl fmt::Debug for DXLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DXLine:({} ~ {})",
            self.xlm,
            self.xln,
        )
    }
}
