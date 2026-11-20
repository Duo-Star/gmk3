// src/pakoo_forest/geo/x_line.rs
// 交叉直线
//
use crate::pakoo_forest::calc::Vec3C;
use crate::pakoo_forest::geo::{Dot, Line, Ddot};
use std::fmt;
use std::fmt::{Display, Formatter};

// 交叉直线
pub struct XLine {
    lm: Line,
    ln: Line,
}

impl XLine {
    /// 从两条直线
    /// 默认方法
    #[inline]
    pub const fn new(lm_: Line, ln_: Line) -> XLine {
        XLine { lm: lm_, ln: ln_ }
    }
    /// 从 4 点
    /// 每条直线 2 点
    #[inline]
    pub  fn from_d4(lm_p1: Dot, lm_p2: Dot, ln_p1: Dot, ln_p2: Dot) -> XLine {
        XLine { lm: lm_p1.join(lm_p2), ln: ln_p1.join(ln_p2) }
    }
    /// 从点和骈点
    /// 点作为交叉直线的中心
    #[inline]
    pub fn from_d_dp(dot: Dot, d_dot:Ddot) -> XLine {
        XLine::from_d4(dot, d_dot.p(), dot, d_dot.q())
    }

    /// XY 坐标轴
    pub const XY_AXIS: XLine = XLine::new(Line::X_AXIS, Line::Y_AXIS);

    /// 获取其中一条直线 lm
    pub fn lm(&self) -> Line {
        self.lm
    }
    /// 获取其中一条直线 ln
    pub fn ln(&self) -> Line {
        self.ln
    }

    /// 自交叉点
    pub fn center(&self) -> Dot {
        self.lm.meet(self.ln)
    }

}


impl fmt::Display for XLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} | {}",
            self.lm,
            self.ln,
        )
    }
}

impl fmt::Debug for XLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} | {}",
            self.lm,
            self.ln,
        )
    }
}
