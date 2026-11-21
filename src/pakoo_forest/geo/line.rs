// src/pakoo_forest/geo/line.rs
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

    /// 实直线
    pub const fn to_3f64(self) -> [f64; 3] {
        [self.p.v[0].re, self.p.v[1].re, self.p.v[2].re]
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

    /// 点 `d` 到直线的有向/复数距离（与 `l` 和 `d` 的齐次缩放无关）。
    /// 若 `d` 在无穷远（`z == 0`）或直线退化（`a² + b² == 0`），返回非有限值（如 `NaN` / `inf`）。
    #[inline]
    pub fn distance(&self, d: Dot) -> Complex {
        let denom_xy = (self.x() * self.x() + self.y() * self.y()).sqrt();
        self.evaluate(&d) / (d.z() * denom_xy)
    }

    /// 点 `d` 到直线的实数距离（模长）。
    #[inline]
    pub fn distance_f64(&self, d: Dot) -> f64 {
        self.distance(d).abs()
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
    #[must_use]
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

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Line;
    use crate::pakoo_forest::calc::{Complex, Cov3C};
    use crate::pakoo_forest::geo::Dot;

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let l = Line::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0));
        assert_eq!(l.x(), c(1.0, 2.0));
        assert_eq!(l.y(), c(3.0, -4.0));
        assert_eq!(l.z(), c(0.0, 5.0));
        assert_eq!(
            Line::from_f64s(1.0, 2.0, 3.0),
            Line::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(
            Line::from_cov3c(Cov3C::from_f64s(1.0, 2.0, 3.0)),
            Line::from_f64s(1.0, 2.0, 3.0)
        );
        assert_eq!(Line::INF, Line::from_f64s(0.0, 0.0, 1.0));
        assert_eq!(l.to_array(), [c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0)]);
        assert_eq!(
            l.to_cov3c(),
            Cov3C::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0))
        );
        // 过两点的直线
        let p = Dot::from_f64s(0.0, 0.0, 1.0);
        let q = Dot::from_f64s(1.0, 0.0, 1.0);
        assert_eq!(Line::from_points(&p, &q), p.join(q));
    }

    // ---- 平行 ----

    #[test]
    fn parallel() {
        let l = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let m = Line::from_f64s(0.0, 2.0, 3.0); // 2y + 3z = 0，平行
        assert!(l.is_pall(m, 0.0));
        assert!(m.is_pall(l, 0.0));
        // 尺度无关
        assert!(l.is_pall(l.scaled(c(2.0, 0.0)), 0.0));
        // 无穷远直线与一切直线平行
        assert!(Line::INF.is_pall(l, 0.0));
        assert!(l.is_pall(Line::INF, 0.0));
        // 不平行的例子
        assert!(!l.is_pall(Line::from_f64s(1.0, 0.0, 0.0), EPS));
        // 平行直线的交点在无穷远直线上
        let meet = l.meet(m);
        assert!(meet.is_at_infinity());
    }

    // ---- 垂直 ----

    #[test]
    fn perpendicular() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        assert!(x_axis.is_vert(y_axis, 0.0));
        assert!(y_axis.is_vert(x_axis, 0.0));
        let l = Line::from_f64s(1.0, 1.0, 0.0);
        let m = Line::from_f64s(1.0, -1.0, 0.0);
        assert!(l.is_vert(m, 0.0));
        assert!(!l.is_vert(Line::from_f64s(1.0, 0.0, 0.0), EPS));
        // 尺度无关
        assert!(x_axis.scaled(c(3.0, 0.0)).is_vert(y_axis, 0.0));
        // 无穷远直线与一切直线垂直（由绝对定义的约定）
        assert!(Line::INF.is_vert(x_axis, 0.0));
        // 垂线与自身的无穷远点：调和四点由虚圆点定义（I、J 与 l⊥ 的无穷远点共轭配对）
        assert_eq!(Dot::I.conj(), Dot::J);
    }

    // ---- 两线交点 ----

    #[test]
    fn meet() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        // 坐标轴交于齐次原点（射影相等）
        let origin = x_axis.meet(y_axis);
        assert!(origin.approx_eq(&Dot::ZERO, 0.0));
        assert!(origin.is_on_line(x_axis, 0.0));
        assert!(origin.is_on_line(y_axis, 0.0));
        // 平行直线相交于无穷远点，且该点同时在两直线上
        let l = Line::from_f64s(0.0, 1.0, 3.0);
        let m = Line::from_f64s(0.0, 1.0, -2.0);
        let meet = l.meet(m);
        assert!(meet.is_at_infinity());
        assert!(meet.is_on_line(l, 0.0));
        assert!(meet.is_on_line(m, 0.0));
        // 相同直线 → 无效点
        assert!(l.meet(l).is_zero());
        // 与无穷远直线的交点 == 方向无穷远点
        let g = Line::from_f64s(2.0, 3.0, 1.0);
        assert!(Line::INF.meet(g).approx_eq(&g.inf_point(), 0.0));
        assert!(Line::INF.meet(g).approx_eq(&Dot::inf(g), 0.0));
    }

    // ---- 关联判定 / 配对 ----

    #[test]
    fn contains_evaluate() {
        let p0 = Dot::from_f64s(0.0, 0.0, 1.0);
        let p1 = Dot::from_f64s(1.0, 2.0, 1.0);
        let l = p0.join(p1);
        // 精确判定
        assert!(l.contains(&p0, 0.0));
        assert!(l.contains(&p1, 0.0));
        assert!(l.contains(&p0.scaled(c(1.0, 2.0)), 0.0));
        assert_eq!(l * p0, Complex::ZERO); // 运算符 == evaluate
        assert_eq!(l.evaluate(&p1), Complex::ZERO);
        // 不包含
        assert!(!l.contains(&Dot::from_f64s(0.0, 1.0, 1.0), EPS));
        // 容差版：l: −2x + y = 0；点 (1e-10, 3e-10, 1) 偏离 1e-10
        let near = Dot::from_f64s(1e-10, 3e-10, 1.0);
        assert!(l.contains(&near, 1e-9));
        assert!(!l.contains(&near, 1e-12));
    }

    // ---- 方向无穷远点 ----

    #[test]
    fn inf_point() {
        let l = Line::from_f64s(2.0, 3.0, 1.0);
        let inf = l.inf_point();
        assert_eq!(inf, Dot::from_f64s(3.0, -2.0, 0.0));
        assert!(inf.is_at_infinity());
        assert!(inf.is_on_line(l, 0.0));
        assert!(inf.is_on_line(Line::INF, 0.0));
    }

    // ---- 射影相等 / 代表元运算 ----

    #[test]
    fn projective_ops() {
        let l = Line::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        // 与整体缩放无关（含复数缩放；非精确表示的比例因子需极小容差）
        assert!(l.approx_eq(&l.scaled(c(-2.0, 0.0)), 0.0));
        assert!(l.approx_eq(&l.scaled(c(0.3, -0.7)), EPS));
        let n = l.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert!(n.approx_eq(&l, EPS));
        assert_eq!(Line::from_f64s(0.0, 0.0, 0.0).normalize(), None);
        // 共轭 / 缩放
        assert_eq!(l.conj(), Line::new(c(3.0, -1.0), c(0.0, 2.0), c(1.0, -0.5)));
        assert_eq!(
            l.scaled(c(2.0, 0.0)),
            Line::from_cov3c(l.to_cov3c().mul_complex(c(2.0, 0.0)))
        );
    }

    // ---- 点到直线距离 ----

    #[test]
    fn distance_to_dot() {
        // 直线 x = 0 到点 (3, 4, 1) 的距离为 3，到直线上点的距离为 0
        let y_axis = Line::Y_AXIS; // x = 0[cite: 2]
        let p = Dot::from_f64s(3.0, 4.0, 1.0);
        let on_line = Dot::from_f64s(0.0, 5.0, 1.0);

        assert!((y_axis.distance_f64(p) - 3.0).abs() < EPS);
        assert!(y_axis.distance(on_line).abs() < EPS);

        // 经典斜线：3x + 4y - 10 = 0 到原点 (0, 0, 1) 的距离为 |-10| / 5 = 2
        let l = Line::from_f64s(3.0, 4.0, -10.0);
        assert!((l.distance_f64(Dot::ZERO) - 2.0).abs() < EPS);

        // 齐次尺度不变性：缩放直线或点不影响距离模长
        let l_scaled = l.scaled(c(2.0, -1.0));
        let p_scaled = p.scaled(c(0.5, 1.5));
        assert!((l_scaled.distance_f64(p_scaled) - l.distance_f64(p)).abs() < EPS);

        // 无穷远点 / 无穷远直线退化时返回非有限值
        assert!(!l.distance_f64(Dot::I).is_finite());
        assert!(!Line::INF.distance_f64(p).is_finite());
    }

    // ---- 谓词 / 输出 ----

    #[test]
    fn predicates_display() {
        assert!(Line::INF.is_at_infinity());
        assert!(!Line::from_f64s(1.0, 0.0, 0.0).is_at_infinity());
        assert!(Line::from_f64s(0.0, 0.0, 0.0).is_zero());
        assert!(!Line::INF.is_zero());
        assert!(Line::from_f64s(1.0, 2.0, 3.0).is_real());
        assert!(!Line::new(Complex::I, Complex::ZERO, Complex::ZERO).is_real());
        assert!(Line::from_f64s(1.0, 2.0, 3.0).is_finite());
        assert!(!Line::new(c(f64::INFINITY, 0.0), Complex::ZERO, Complex::ZERO).is_finite());
        // 输出
        assert_eq!(Line::from_f64s(1.0, 2.0, 3.0).to_string(), "(1 : 2 : 3)");
        assert_eq!(Line::INF.to_string(), "(0 : 0 : 1)");
        assert_eq!(format!("{:?}", Line::INF), "Line(0 : 0 : 1)");
    }
}
