// src/pakoo_forest/geo/x_line.rs
// 交叉直线

use crate::pakoo_forest::calc::{Mat3C, Vec3C};
use crate::pakoo_forest::geo::{Conic, Ddot, Dot, Line};
use std::fmt;

/// 交叉直线 `{lm, ln}`，成员直线彼此不区分。
#[derive(Clone, Copy, PartialEq)]
pub struct XLine {
    lm: Line,
    ln: Line,
}

impl XLine {
    /// 从两条直线构造（默认方法）
    #[inline]
    pub const fn new(lm_: Line, ln_: Line) -> XLine {
        XLine { lm: lm_, ln: ln_ }
    }

    /// 从 4 点构造（每条直线 2 点）
    #[inline]
    pub fn from_d4(lm_p1: Dot, lm_p2: Dot, ln_p1: Dot, ln_p2: Dot) -> XLine {
        XLine {
            lm: lm_p1.join(lm_p2),
            ln: ln_p1.join(ln_p2),
        }
    }

    /// 从点和骈点构造（点作为交叉直线的中心）
    #[inline]
    pub fn from_d_dp(dot: Dot, d_dot: Ddot) -> XLine {
        XLine::from_d4(dot, d_dot.p(), dot, d_dot.q())
    }

    /// XY 坐标轴
    pub const XY_AXIS: XLine = XLine::new(Line::X_AXIS, Line::Y_AXIS);

    /// 获取其中一条直线 lm
    #[inline]
    pub const fn lm(&self) -> Line {
        self.lm
    }

    /// 获取其中一条直线 ln
    #[inline]
    pub const fn ln(&self) -> Line {
        self.ln
    }

    /// 自交叉点
    #[inline]
    pub fn center(&self) -> Dot {
        self.lm.meet(self.ln)
    }

    /// 提升为（至多秩 2 的）二次曲线：`Q = ½(lₘᵀ·lₙ + lₙᵀ·lₘ)`。
    #[inline]
    #[must_use]
    pub fn as_conic(&self) -> Conic {
        let m = Mat3C::from_outer(Vec3C::from_array(self.lm.to_array()), self.ln.to_cov3c());
        Conic::from_matrix(m)
    }

    /// 两直线是否均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.lm.is_finite() && self.ln.is_finite()
    }

    /// 任一直线是否为无效零直线。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.lm.is_zero() || self.ln.is_zero()
    }

    /// 无序、尺度无关的射影近似相等。
    #[inline]
    pub fn approx_eq(&self, other: &XLine, eps: f64) -> bool {
        (self.lm.approx_eq(&other.lm, eps) && self.ln.approx_eq(&other.ln, eps))
            || (self.lm.approx_eq(&other.ln, eps) && self.ln.approx_eq(&other.lm, eps))
    }
}

impl fmt::Display for XLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XLine:({} ~ {})", self.lm, self.ln)
    }
}

impl fmt::Debug for XLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XLine:({} ~ {})", self.lm, self.ln)
    }
}