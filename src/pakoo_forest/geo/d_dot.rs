use crate::pakoo_forest::calc::Vec3C;
use crate::pakoo_forest::geo::{Dot, Line};
use std::fmt;
use std::fmt::{Display, Formatter};

// 一对点
pub struct Ddot {
    p: Dot,
    q: Dot,
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

    /// 虚圆点  各向同性点
    pub const CYCLIC: Ddot = Ddot::new(Dot::I, Dot::J);

    pub fn p(&self) -> Dot {
        self.p
    }
    pub fn q(&self) -> Dot {
        self.q
    }
    ///
    pub fn line(&self) -> Line {
        Line::from_points(&self.p, &self.q)
    }
    ///
    pub fn center(&self) -> Dot {
        self.p.mid(&self.q)
    }
}


impl fmt::Display for Ddot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({} : {} : {}) | ({} : {} : {})",
            self.p.x(),
            self.p.y(),
            self.p.z(),
            self.q.x(),
            self.q.y(),
            self.q.z()
        )
    }
}

impl fmt::Debug for Ddot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({} : {} : {}) | ({} : {} : {})",
            self.p.x(),
            self.p.y(),
            self.p.z(),
            self.q.x(),
            self.q.y(),
            self.q.z()
        )
    }
}
