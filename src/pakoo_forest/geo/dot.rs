// src/pakoo_forest/geo/dot.rs
// 齐次化坐标  Vec3C

use crate::pakoo_forest::calc::{Complex, Vec3C};
use crate::pakoo_forest::geo::{Line};

// 约定：所有产生 Line 的地方已经完成归一化，所以用的时候不用处理
// Dot [x : Y : Z]
pub struct Dot {
    pub p: Vec3C
}
impl Dot {
    /// 构造函数
    pub const fn new(x_:Complex, y_:Complex, z :Complex) -> Dot {
        Dot {p: Vec3C::new(x_, y_, z)}
    }
    ///
    pub const fn from_f64s(x_:f64, y_:f64, z_:f64) -> Dot {
        Dot {p: Vec3C::from_f64s(x_, y_, z_)}
    }
    ///
    pub const fn from_vec3c(p_: Vec3C) -> Dot {
        Dot {p: p_}
    }
    /// 零点常量
    pub const ZERO: Dot = Dot::new(Complex::ZERO, Complex::ZERO, Complex::ONE);

    /// 虚圆点 I J
    pub const I: Dot = Dot::new(Complex::ONE, Complex::I, Complex::ZERO);
    pub const J: Dot = Dot::new(Complex::ONE, Complex::NEG_I, Complex::ZERO);

    /// 通过直线指定方向的无穷远点
    pub fn inf(l:Line) -> Dot {
        Dot::new(l.y(), -l.x(), Complex::ZERO)
    }

    /// 构造直线
    pub fn join(&self, other:Dot) -> Line {

    }

    pub fn is_on_line(&self) -> bool {

    }


}

