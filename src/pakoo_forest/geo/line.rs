// src/pakoo_forest/geo/line.rs
// Cov3C
// Line
// INF 无穷远直线
//

use crate::pakoo_forest::calc::{Complex, Cov3C, Vec3C};
use crate::pakoo_forest::geo::dot::Dot;

pub struct Line {
    p: Cov3C,
}

// 约定：所有产生 Line 的地方已经完成归一化，所以用的时候不用处理
impl Line {
    pub const fn new(x_: Complex, y_: Complex, z: Complex) -> Line {
        Line {
            p: Cov3C::new(x_, y_, z),
        }
    }
    pub const fn from_vec3c(p_: Cov3C) -> Line {
        Line { p: p_ }
    }
    pub const INF: Line = Line::new(Complex::ZERO, Complex::ZERO, Complex::ONE);

    ///
    pub fn x(&self) -> Complex {
        self.p[0]
    }
    pub fn y(&self) -> Complex {
        self.p[1]
    }
    pub fn z(&self) -> Complex {
        self.p[2]
    }

    /// 两条直线平行，当且仅当它们与 L_inf 交于同一点
    pub fn is_pall(&self, other: Line, eps: f64) -> bool {
        (self.x() * other.y() - self.y() * other.x()).approx_eq(Complex::ZERO, eps)
    }

    /// 判定垂直，两个虚圆点和两直线的无穷远点构成调和四点
    pub fn is_vert(&self, other: Line, eps: f64) -> bool {
        (self.x() * other.x() + self.y() * other.y()).approx_eq(Complex::ZERO, eps)
    }

    
}
