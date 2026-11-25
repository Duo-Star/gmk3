// src/pakoo_forest/geo/line/line.rs
//! 齐次直线 `Line` —— 射影平面上的直线 `(a : b : c)`。
//!
//! 是 `Cov3C`（行向量）的 Newtype：点 `(x:y:z)` 在直线上当且仅当
//! `a·x + b·y + c·z = 0`。与 `Dot` 互为对偶：两线交点是叉积 [`Line::meet`]，
//! 关联关系是对偶配对 `Line × Dot → Complex`（`l * p == 0` ⇔ 点在线上）。
//!
//! # 尺度约定
//!
//! 齐次坐标只定义到整体非零比例：`Line` 的构造不做归一化（保持叉积等的精确性），
//! 所有关联/相等判定均与整体缩放无关；需要单位代表元时显式调用 [`Line::normalize`]。
//!
//! # 绝对与无穷远
//!
//! - [`Line::INF`]：无穷远直线 `(0 : 0 : 1)`；
//! - [`Dot::I`] / [`Dot::J`]：两个虚圆点（欧氏度量的"绝对"），
//!   垂直判定 [`Line::is_vert`] 即由它们定义。

use std::fmt;
use std::ops::Mul;

use crate::pakoo_forest::calc::{Complex, Cov3C, Vec3C};
use crate::pakoo_forest::geo::conic::Conic;
use crate::pakoo_forest::geo::dot::Dot;

/// 齐次直线 `(a : b : c)`，定义到整体非零比例。
#[derive(Clone, Copy, PartialEq)]
pub struct Line {
    /// 方程系数行向量（私有，通过 `x()` / `y()` / `z()` 与 `to_cov3c()` 访问）
    p: Cov3C,
}

impl Line {
    /// 无穷远直线 `(0 : 0 : 1)`
    pub const INF: Line = Line::new(Complex::ZERO, Complex::ZERO, Complex::ONE);
    pub const X_AXIS: Line = Line::new(Complex::ZERO, Complex::ONE, Complex::ZERO);
    pub const Y_AXIS: Line = Line::new(Complex::ONE, Complex::ZERO, Complex::ZERO);

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 构造函数：方程 `a·x + b·y + c·z = 0` 的系数。
    #[inline]
    pub const fn new(a: Complex, b: Complex, c: Complex) -> Line {
        Line {
            p: Cov3C::new(a, b, c),
        }
    }

    /// 由三个实数构造。
    #[inline]
    pub const fn from_f64s(a: f64, b: f64, c: f64) -> Line {
        Line {
            p: Cov3C::from_f64s(a, b, c),
        }
    }

    /// 由行向量构造。
    #[inline]
    pub const fn from_cov3c(p: Cov3C) -> Line {
        Line { p }
    }

    /// 过两点的直线。
    #[inline]
    pub fn from_points(p: &Dot, q: &Dot) -> Line {
        p.join(*q)
    }

    /// 方程系数 `a`（`a·x + b·y + c·z = 0`）。
    #[inline]
    pub fn x(&self) -> Complex {
        self.p.v[0]
    }

    /// 方程系数 `b`。
    #[inline]
    pub fn y(&self) -> Complex {
        self.p.v[1]
    }

    /// 方程系数 `c`。
    #[inline]
    pub fn z(&self) -> Complex {
        self.p.v[2]
    }

    /// 行向量形式。
    #[inline]
    pub const fn to_cov3c(self) -> Cov3C {
        self.p
    }

    /// 数组形式。
    #[inline]
    pub const fn to_array(self) -> [Complex; 3] {
        self.p.v
    }

    /// 导出实齐次直线系数（带去全局复相位与虚部容差检测）；非射影实直线或含非有限分量返回 `None`。
    pub fn try_to_3f64(self, eps: f64) -> Option<[f64; 3]> {
        let v = &self.p.v;
        let mut max_idx = 0usize;
        let mut max_abs = 0.0f64;
        for (i, c) in v.iter().enumerate() {
            if !c.is_finite() {
                return None;
            }
            let a = c.abs();
            if a > max_abs {
                max_abs = a;
                max_idx = i;
            }
        }
        if max_abs == 0.0 {
            return Some([0.0; 3]);
        }
        let tol = eps * max_abs;
        if v.iter().all(|c| c.im.abs() <= tol) {
            return Some([v[0].re, v[1].re, v[2].re]);
        }
        let phase_conj = v[max_idx].conj().div_f64(max_abs);
        let mut out = [0.0; 3];
        for i in 0..3 {
            let z = v[i] * phase_conj;
            if z.im.abs() > tol {
                return None;
            }
            out[i] = z.re;
        }
        Some(out)
    }

    /// 实直线系数（默认容差 `1e-12`）；非射影实直线返回全 `NaN`。
    #[inline]
    pub fn to_3f64(self) -> [f64; 3] {
        self.try_to_3f64(1e-12).unwrap_or([f64::NAN; 3])
    }

    #[inline]
    pub fn to_2f64(self) -> [f64; 2] {
        let a: [f64; 3] = self.to_3f64();
        [a[0], a[1]]
    }

    /// 嵌入为二重直线二次曲线：`Q = lᵀ · l`（秩 1）。
    #[inline]
    #[must_use]
    pub fn as_conic(&self) -> Conic {
        let (a, b, c) = (self.x(), self.y(), self.z());
        Conic::new(a * a, a * b, b * b, a * c, b * c, c * c)
    }

    // ------------------------------------------------------------------
    // 几何运算
    // ------------------------------------------------------------------

    /// 两直线的交点：`meet(l, m) = l × m`。
    ///
    /// 平行直线相交于无穷远点（`is_at_infinity()` 为真）；
    /// 相同（重合）直线得到无效点 `(0:0:0)`。
    #[inline]
    #[must_use]
    pub fn meet(&self, other: Line) -> Dot {
        let a = Vec3C::from_array(self.p.v);
        let b = Vec3C::from_array(other.p.v);
        Dot::from_vec3c(a.cross(b))
    }

    /// 直线在点处的取值 `l·p`（为 0 ⇔ 点在线上）。
    #[inline]
    #[must_use]
    pub fn evaluate(&self, p: &Dot) -> Complex {
        self.p.dot(p.p)
    }

    /// 点是否在直线上（关联判定，与点/线的整体缩放无关）。
    /// `eps = 0` 时精确判定。
    #[inline]
    pub fn contains(&self, p: &Dot, eps: f64) -> bool {
        let s = eps * (1.0 + self.norm() * p.norm());
        self.evaluate(p).abs() <= s
    }

    /// 点 `d` 到直线的复代数距离 `(l·p) / (z * sqrt(a^2 + b^2))`（`sqrt` 取主值分支）。
    ///
    /// 与 `l` 和 `d` 的齐次缩放无关；若 `d` 在无穷远（`z == 0`）或直线退化（`a^2 + b^2 == 0`），
    /// 返回非有限值（`NaN` / `inf`）[cite: 2]。
    #[inline]
    pub fn distance_complex(&self, d: Dot) -> Complex {
        let denom_xy = (self.x() * self.x() + self.y() * self.y()).sqrt();
        self.evaluate(&d) / (d.z() * denom_xy)
    }

    /// 点 `d` 到直线的复平方距离 `((l·p) / z)^2 / (a^2 + b^2)`（无 `sqrt` 分支切割）。
    ///
    /// 与 `l` 和 `d` 的齐次缩放无关；若 `d` 在无穷远（`z == 0`）或直线退化（`a^2 + b^2 == 0`），
    /// 返回非有限值[cite: 2]。
    #[inline]
    pub fn distance_sqr_complex(&self, d: Dot) -> Complex {
        let num = self.evaluate(&d);
        let denom_sqr = d.z() * d.z() * (self.x() * self.x() + self.y() * self.y());
        (num * num) / denom_sqr
    }

    /// 点 `d` 到直线的实数距离（模长）。
    ///
    /// 与 `l` 和 `d` 的齐次缩放无关；退化时返回非有限值[cite: 2]。
    #[inline]
    pub fn distance_f64(&self, d: Dot) -> f64 {
        self.distance_complex(d).abs()
    }

    /// 直线方向的无穷远点 `(b : −a : 0)`。
    #[inline]
    pub fn inf_point(&self) -> Dot {
        Dot::new(self.y(), -self.x(), Complex::ZERO)
    }

    /// 两条直线平行 ⇔ 与无穷远直线交于同一点（叉积的 `z` 分量为零）。
    /// `eps = 0` 时精确判定。
    #[inline]
    pub fn is_pall(&self, other: Line, eps: f64) -> bool {
        let s = eps * (1.0 + self.norm() * other.norm());
        (self.x() * other.y() - self.y() * other.x()).abs() <= s
    }

    /// 两条直线垂直：两直线的无穷远点与两个虚圆点构成调和四点，
    /// 等价于方向部分正交 `a·a' + b·b' = 0`。`eps = 0` 时精确判定。
    #[inline]
    pub fn is_vert(&self, other: Line, eps: f64) -> bool {
        let s = eps * (1.0 + self.norm() * other.norm());
        (self.x() * other.x() + self.y() * other.y()).abs() <= s
    }

    // ------------------------------------------------------------------
    // 代表元运算（不改变射影意义）
    // ------------------------------------------------------------------

    /// 逐分量共轭。
    #[inline]
    #[must_use = "这将返回逐分量共轭后的直线"]
    pub fn conj(&self) -> Line {
        Line::from_cov3c(self.p.conj())
    }

    /// 整体缩放（同一直线）。
    #[inline]
    #[must_use]
    pub fn scaled(&self, s: Complex) -> Line {
        Line::from_cov3c(self.p.mul_complex(s))
    }

    /// 单位范数代表元（唯一到单位复相位）；无效直线（`(0:0:0)` 或非有限）返回 `None`。
    #[inline]
    pub fn normalize(&self) -> Option<Line> {
        self.p.normalize().map(Line::from_cov3c)
    }

    /// 代表元的范数。
    #[inline]
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.p.norm()
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 无效直线 `(0 : 0 : 0)`。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.p.is_zero()
    }

    /// 是否为无穷远直线（`a == 0 && b == 0 && c != 0`）。
    #[inline]
    pub fn is_at_infinity(&self) -> bool {
        self.x() == Complex::ZERO && self.y() == Complex::ZERO && self.z() != Complex::ZERO
    }

    /// 所有系数均为实数（实直线）。
    #[inline]
    pub fn is_real(&self) -> bool {
        self.p.v.iter().all(|x| x.is_real())
    }

    /// 所有系数均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.p.is_finite()
    }

    /// 射影相等（与整体缩放无关）：`|l × m| ≤ eps·(1 + |l|·|m|)`。
    /// `eps = 0` 时精确判定。
    ///
    /// 注意：派生的 `PartialEq` 是按分量精确比较的，跨缩放的比较请用本方法。
    #[inline]
    pub fn approx_eq(&self, other: &Line, eps: f64) -> bool {
        let a = Vec3C::from_array(self.p.v);
        let b = Vec3C::from_array(other.p.v);
        let c = a.cross(b);
        c.norm() <= eps * (1.0 + a.norm() * b.norm())
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line:[{} : {} : {}]", self.x(), self.y(), self.z())
    }
}

impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line:[{} : {} : {}]", self.x(), self.y(), self.z())
    }
}

// ---------------------------------------------------------------------------
// 运算符：对偶配对
// ---------------------------------------------------------------------------

/// `Line × Dot → Complex`：对偶配对 `l·p`（为 0 ⇔ 点在线上）。
impl Mul<Dot> for Line {
    type Output = Complex;
    #[inline]
    fn mul(self, rhs: Dot) -> Complex {
        Line::evaluate(&self, &rhs)
    }
}
