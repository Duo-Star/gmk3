// src/pakoo_forest/geo/d_x_line.rs

// 一对交叉直线

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