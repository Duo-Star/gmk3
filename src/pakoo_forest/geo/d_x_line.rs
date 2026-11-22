// src/pakoo_forest/geo/d_x_line.rs

use crate::pakoo_forest::geo::x_line::XLine;
use std::fmt;

/// 一对交叉直线 `{xlm, xln}`，彼此不区分。
#[derive(Clone, Copy, PartialEq)]
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

    #[inline]
    pub const fn xlm(&self) -> XLine {
        self.xlm
    }

    #[inline]
    pub const fn xln(&self) -> XLine {
        self.xln
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.xlm.is_finite() && self.xln.is_finite()
    }

    /// 无序、尺度无关的射影近似相等。
    #[inline]
    pub fn approx_eq(&self, other: &DXLine, eps: f64) -> bool {
        (self.xlm.approx_eq(&other.xlm, eps) && self.xln.approx_eq(&other.xln, eps))
            || (self.xlm.approx_eq(&other.xln, eps) && self.xln.approx_eq(&other.xlm, eps))
    }
}

impl fmt::Display for DXLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DXLine:({} ~ {})", self.xlm, self.xln)
    }
}

impl fmt::Debug for DXLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DXLine:({} ~ {})", self.xlm, self.xln)
    }
}