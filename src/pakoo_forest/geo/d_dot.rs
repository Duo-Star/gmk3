// src/pakoo_forest/geo/d_dot.rs

use crate::pakoo_forest::calc::Vec3C;
use crate::pakoo_forest::geo::{Dot, Line};
use std::fmt;

/// 一对点，骈点 `Ddot` 由 `{p, q}` 组成，彼此不区分。
/// 对无序两点的抽象，比如直线和圆锥曲线的两个交点、圆锥曲线的焦点对。
#[derive(Clone, Copy, PartialEq)]
pub struct Ddot {
    p: Dot,
    q: Dot,
}

#[inline]
fn dot_proj_approx_eq(a: Dot, b: Dot, eps: f64) -> bool {
    if a.is_zero() || b.is_zero() {
        return a.is_zero() && b.is_zero();
    }
    let c = a.p.cross(b.p);
    c.norm() <= eps * (1.0 + a.norm() * b.norm())
}

impl Ddot {
    #[inline]
    pub const fn new(p_: Dot, q_: Dot) -> Ddot {
        Ddot { p: p_, q: q_ }
    }

    #[inline]
    pub const fn from_vec3cs(p_: Vec3C, q_: Vec3C) -> Ddot {
        Ddot {
            p: Dot::from_vec3c(p_),
            q: Dot::from_vec3c(q_),
        }
    }

    /// 虚圆点（各向同性点）
    pub const CYCLIC: Ddot = Ddot::new(Dot::I, Dot::J);

    #[inline]
    pub const fn p(&self) -> Dot {
        self.p
    }

    #[inline]
    pub const fn q(&self) -> Dot {
        self.q
    }

    /// 两点连线
    #[inline]
    pub fn line(&self) -> Line {
        Line::from_points(&self.p, &self.q)
    }

    /// 两点中心
    #[inline]
    pub fn center(&self) -> Dot {
        self.p.mid(&self.q)
    }

    /// 两分量均有限且非零。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.p.is_finite() && self.q.is_finite()
    }

    /// 点 `d` 是否属于该骈点（射影容差 `eps`）。
    #[inline]
    pub fn contains(&self, d: &Dot, eps: f64) -> bool {
        dot_proj_approx_eq(self.p, *d, eps) || dot_proj_approx_eq(self.q, *d, eps)
    }

    /// 无序、尺度无关的射影近似相等。
    #[inline]
    pub fn approx_eq(&self, other: &Ddot, eps: f64) -> bool {
        (dot_proj_approx_eq(self.p, other.p, eps) && dot_proj_approx_eq(self.q, other.q, eps))
            || (dot_proj_approx_eq(self.p, other.q, eps)
            && dot_proj_approx_eq(self.q, other.p, eps))
    }
}

impl fmt::Display for Ddot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ddot:({} ~ {})", self.p, self.q)
    }
}

impl fmt::Debug for Ddot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ddot:({} ~ {})", self.p, self.q)
    }
}