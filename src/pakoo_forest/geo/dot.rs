// src/pakoo_forest/geo/dot.rs
//! 齐次点 `Dot` —— 射影平面上的点 `(x : y : z)`。
//!
//! 是 `Vec3C`（列向量）的 Newtype。两点连线是叉积 [`Dot::join`]；
//! 点在直线上的判定是协变/逆变的对偶配对（见 `Line::contains`）。
//!
//! # 尺度约定
//!
//! 齐次坐标只定义到整体非零比例：`Dot` 的构造不做归一化（保持叉积等的精确性），
//! 所有关联/相等判定均与整体缩放无关；需要单位代表元时显式调用 [`Dot::normalize`]。

use std::fmt;

use crate::pakoo_forest::calc::{Complex, Mat3C, Vec3C};
use crate::pakoo_forest::geo::line::Line;

/// 齐次点 `(x : y : z)`，定义到整体非零比例。
#[derive(Clone, Copy, PartialEq)]
pub struct Dot {
    /// 齐次坐标列向量（公开，便于底层运算）
    pub p: Vec3C,
}

impl Dot {
    /// 齐次原点 `(0 : 0 : 1)`。
    ///
    /// 注意：这不是无效点 `(0:0:0)`；无效点用 [`Dot::is_zero`] 检测。
    pub const ZERO: Dot = Dot::new(Complex::ZERO, Complex::ZERO, Complex::ONE);

    /// 虚圆点 `I = (1 : i : 0)`——无穷远直线上的绝对点
    pub const I: Dot = Dot::new(Complex::ONE, Complex::I, Complex::ZERO);
    /// 虚圆点 `J = (1 : −i : 0)`
    pub const J: Dot = Dot::new(Complex::ONE, Complex::NEG_I, Complex::ZERO);

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 由齐次分量构造。
    #[inline]
    pub const fn new(x: Complex, y: Complex, z: Complex) -> Dot {
        Dot {
            p: Vec3C::new(x, y, z),
        }
    }

    /// 由三个实数构造。
    #[inline]
    pub const fn from_f64s(x: f64, y: f64, z: f64) -> Dot {
        Dot {
            p: Vec3C::from_f64s(x, y, z),
        }
    }

    /// 由列向量构造。
    #[inline]
    pub const fn from_vec3c(p: Vec3C) -> Dot {
        Dot { p }
    }

    /// 仿射坐标 `(x, y)` 的齐次嵌入 `(x : y : 1)`。
    #[inline]
    pub const fn from_affine(x: Complex, y: Complex) -> Dot {
        Dot::new(x, y, Complex::ONE)
    }

    /// 仿射坐标 `(x/z, y/z)`；无穷远点（`z == 0`）无仿射坐标，返回 `None`。
    #[inline]
    pub fn to_affine(&self) -> Option<(Complex, Complex)> {
        if self.p.v[2] == Complex::ZERO {
            return None;
        }
        Some((self.p.v[0] / self.p.v[2], self.p.v[1] / self.p.v[2]))
    }

    /// 齐次分量 `x`。
    #[inline]
    pub fn x(&self) -> Complex {
        self.p.v[0]
    }

    /// 齐次分量 `y`。
    #[inline]
    pub fn y(&self) -> Complex {
        self.p.v[1]
    }

    /// 齐次分量 `z`。
    #[inline]
    pub fn z(&self) -> Complex {
        self.p.v[2]
    }

    /// 列向量形式。
    #[inline]
    pub const fn to_vec3c(self) -> Vec3C {
        self.p
    }

    /// 数组形式。
    #[inline]
    pub const fn to_array(self) -> [Complex; 3] {
        self.p.v
    }

    ///
    /// 仅当点为实点（所有分量虚部为 0）时返回实坐标，否则 None。
    pub fn to_3f64(&self) -> [f64; 3] {
        if !self.is_real() {
            return [f64::NAN; 3];
        }
        [self.p.v[0].re, self.p.v[1].re, self.p.v[2].re]
    }
    // ------------------------------------------------------------------
    // 几何运算
    // ------------------------------------------------------------------

    /// 直线 `l` 方向上的无穷远点 `(l.y : −l.x : 0)`。
    #[inline]
    pub fn inf(l: Line) -> Dot {
        Dot::new(l.y(), -l.x(), Complex::ZERO)
    }

    /// 过两点的直线：`join(p, q) = p × q`。
    #[inline]
    #[must_use]
    pub fn join(&self, other: Dot) -> Line {
        let v = self.p.cross(other.p);
        Line::new(v.v[0], v.v[1], v.v[2])
    }

    /// 点在直线上（关联判定，与点/线的整体缩放无关）。
    /// `eps = 0` 时精确判定。
    #[inline]
    pub fn is_on_line(&self, l: Line, eps: f64) -> bool {
        l.contains(self, eps)
    }

    /// 三点共线（三列行列式为零），与整体缩放无关。
    /// `eps = 0` 时精确判定。
    #[inline]
    pub fn is_collinear(&self, p0: Dot, p1: Dot, eps: f64) -> bool {
        let det = Mat3C::from_3vec(self.p, p0.p, p1.p).det();
        let s = eps * (1.0 + self.norm() * p0.norm() * p1.norm());
        det.abs() <= s
    }

    // ------------------------------------------------------------------
    // 代表元运算（不改变射影意义）
    // ------------------------------------------------------------------

    /// 逐分量共轭。共轭是射影平面上的反全纯对合：`conj(I) = J`。
    #[inline]
    #[must_use]
    pub fn conj(&self) -> Dot {
        Dot::from_vec3c(self.p.conj())
    }

    /// 整体缩放（同一射影点）。
    #[inline]
    #[must_use]
    pub fn scaled(&self, s: Complex) -> Dot {
        Dot::from_vec3c(self.p.mul_complex(s))
    }

    /// 单位范数代表元（唯一到单位复相位）；无效点（`(0:0:0)` 或非有限）返回 `None`。
    #[inline]
    pub fn normalize(&self) -> Option<Dot> {
        self.p.normalize().map(Dot::from_vec3c)
    }

    /// 代表元的范数。
    #[inline]
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.p.norm()
    }


    /// 计算一个点和另一点的中点
    #[inline]
    #[must_use]
    pub fn mid(&self, other: &Dot) -> Dot {
        // 中点不是射影概念，属于仿射概念
        // 在扩大欧氏平面里当然可以算，但它只用到了“无穷远直线”，不需要两个虚圆点，也不需要角度/长度
        // 这里使用任意齐次坐标下的代表元无关公式
        // P = (X_1:Y_1:Z_1), Q = (X_2:Y_2:Z_2), 且 Z_1  Z_2 ！=0 则
        // M = (X_1 Z_2 + X_2 Z_1 : Y_1 Z_2 + Y_2 Z_1 : 2 Z_1 Z_2),
        let (p, q) = (&self.p.v, &other.p.v);
        let (z1, z2) = (p[2], q[2]);
        Dot {
            p: Vec3C::new(
                z2 * p[0] + z1 * q[0],
                z2 * p[1] + z1 * q[1],
                z1 * z2 * Complex::from_f64(2.0),
            ),
        }
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 无效点 `(0 : 0 : 0)`。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.p.is_zero()
    }

    /// 无穷远点（`z == 0` 且非无效点）。
    #[inline]
    pub fn is_at_infinity(&self) -> bool {
        self.p.v[2] == Complex::ZERO && !self.p.is_zero()
    }

    /// 所有分量均为实数（实点）。
    #[inline]
    pub fn is_real(&self) -> bool {
        self.p.v.iter().all(|x| x.is_real())
    }

    /// 所有分量均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.p.is_finite()
    }

    /// 射影相等（与整体缩放无关）：`|p × q| ≤ eps·(1 + |p|·|q|)`。
    /// `eps = 0` 时精确判定。
    ///
    /// 注意：派生的 `PartialEq` 是按分量精确比较的，跨缩放的比较请用本方法。
    #[inline]
    pub fn approx_eq(&self, other: &Dot, eps: f64) -> bool {
        let c = self.p.cross(other.p);
        c.norm() <= eps * (1.0 + self.p.norm() * other.p.norm())
    }
}

impl Default for Dot {
    /// 默认点为齐次原点 `(0 : 0 : 1)`。
    fn default() -> Dot {
        Dot::ZERO
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Dot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} : {} : {})", self.x(), self.y(), self.z())
    }
}

impl fmt::Debug for Dot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dot({} : {} : {})", self.x(), self.y(), self.z())
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Dot;
    use crate::pakoo_forest::calc::Complex;
    use crate::pakoo_forest::geo::Line;

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let p = Dot::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0));
        assert_eq!(p.x(), c(1.0, 2.0));
        assert_eq!(p.y(), c(3.0, -4.0));
        assert_eq!(p.z(), c(0.0, 5.0));
        assert_eq!(
            Dot::from_f64s(1.0, 2.0, 3.0),
            Dot::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(Dot::from_vec3c(p.p), p);
        assert_eq!(
            Dot::from_affine(c(1.0, 2.0), c(3.0, 4.0)),
            Dot::new(c(1.0, 2.0), c(3.0, 4.0), Complex::ONE)
        );
        assert_eq!(
            Dot::ZERO,
            Dot::new(Complex::ZERO, Complex::ZERO, Complex::ONE)
        );
        assert_eq!(p.to_array(), [c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0)]);
        assert_eq!(Dot::default(), Dot::ZERO);
        // 仿射坐标往返
        assert_eq!(
            Dot::from_f64s(2.0, 4.0, 2.0).to_affine(),
            Some((c(1.0, 0.0), c(2.0, 0.0)))
        );
        assert_eq!(
            Dot::from_affine(c(1.0, 0.0), c(2.0, 0.0)).to_affine(),
            Some((c(1.0, 0.0), c(2.0, 0.0)))
        );
        // 无穷远点无仿射坐标
        assert_eq!(Dot::I.to_affine(), None);
    }

    // ---- 虚圆点 ----

    #[test]
    fn circular_points() {
        assert_eq!(Dot::I, Dot::new(Complex::ONE, Complex::I, Complex::ZERO));
        assert_eq!(
            Dot::J,
            Dot::new(Complex::ONE, Complex::NEG_I, Complex::ZERO)
        );
        // 共轭互换两个虚圆点
        assert_eq!(Dot::I.conj(), Dot::J);
        assert_eq!(Dot::J.conj(), Dot::I);
        // 虚圆点在无穷远直线上
        assert!(Dot::I.is_at_infinity());
        assert!(Dot::J.is_at_infinity());
        assert_eq!(Line::INF * Dot::I, Complex::ZERO);
        assert_eq!(Line::INF * Dot::J, Complex::ZERO);
        // 两个虚圆点的连线是无穷远直线（差一个标量因子）
        assert!(Dot::I.join(Dot::J).is_pall(Line::INF, 0.0));
    }

    // ---- 两点连线 ----

    #[test]
    fn join() {
        // (0:0:1) 与 (1:0:1) 决定直线 y = 0
        let l = Dot::from_f64s(0.0, 0.0, 1.0).join(Dot::from_f64s(1.0, 0.0, 1.0));
        assert_eq!(
            (l.x(), l.y(), l.z()),
            (Complex::ZERO, Complex::ONE, Complex::ZERO)
        );
        // 生成点精确地在结果线上
        assert!(l.contains(&Dot::from_f64s(0.0, 0.0, 1.0), 0.0));
        assert!(l.contains(&Dot::from_f64s(1.0, 0.0, 1.0), 0.0));
        assert!(l.contains(&Dot::from_f64s(-3.0, 0.0, 1.0), 0.0));
        assert!(!l.contains(&Dot::from_f64s(0.0, 1.0, 1.0), 0.0));
        // 交换律：join(p, q) 与 join(q, p) 是同一直线（反号）
        let p = Dot::new(c(1.0, 0.0), c(2.0, -1.0), c(3.0, 0.5));
        let q = Dot::from_f64s(4.0, -2.0, 1.0);
        assert!(p.join(q).approx_eq(&q.join(p), 0.0));
    }

    // ---- 关联判定与尺度无关性 ----

    #[test]
    fn incidence_scale_invariance() {
        let p = Dot::new(c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.5));
        let q = Dot::from_f64s(4.0, -2.0, 1.0);
        let l = p.join(q);
        assert!(p.is_on_line(l, 0.0));
        assert!(q.is_on_line(l, 0.0));
        // 点/线整体缩放不影响关联性
        assert!(
            p.scaled(c(2.0, 3.0))
                .is_on_line(l.scaled(c(0.5, -0.25)), 0.0)
        );
        // 配对运算符 == evaluate
        assert_eq!(l * p, l.evaluate(&p));
        assert_eq!(l * p, Complex::ZERO);
        // 不关联的点
        let r = Dot::from_f64s(0.0, 0.0, 1.0);
        assert!(!r.is_on_line(l, EPS));
    }

    // ---- 两线交点 ----

    #[test]
    fn meet() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        // 坐标轴交于齐次原点（射影相等，分量可能反号）
        let origin = x_axis.meet(y_axis);
        assert!(origin.approx_eq(&Dot::ZERO, 0.0));
        assert!(origin.is_on_line(x_axis, 0.0));
        assert!(origin.is_on_line(y_axis, 0.0));
        // 与无穷远直线相交 → 该直线的无穷远点
        let l = Line::from_f64s(1.0, 2.0, 3.0);
        let inf_meet = Line::INF.meet(l);
        assert!(inf_meet.is_at_infinity());
        assert!(inf_meet.approx_eq(&Dot::inf(l), 0.0));
        // 过同一点的两直线交于该点
        let a = Dot::from_f64s(1.0, 2.0, 1.0);
        let b = Dot::from_f64s(-1.0, 3.0, 2.0);
        let c3 = Dot::from_f64s(5.0, 0.5, 4.0);
        let l1 = a.join(b);
        let l2 = a.join(c3);
        assert!(l1.meet(l2).approx_eq(&a, EPS));
    }

    // ---- 三点共线 ----

    #[test]
    fn collinear() {
        let p = Dot::from_f64s(0.0, 0.0, 1.0);
        let p0 = Dot::from_f64s(1.0, 0.0, 1.0);
        let p1 = Dot::from_f64s(2.0, 0.0, 1.0);
        assert!(p.is_collinear(p0, p1, 0.0));
        assert!(p.is_collinear(p1, p0, 0.0)); // 顺序无关
        assert!(p.scaled(c(2.0, 0.0)).is_collinear(p0, p1, 0.0)); // 尺度无关
        let q = Dot::from_f64s(0.0, 1.0, 1.0);
        assert!(!p.is_collinear(p0, q, EPS));
        // 重合点视为共线
        assert!(p.is_collinear(p, q, 0.0));
    }

    // ---- 无穷远方向点 ----

    #[test]
    fn inf_direction() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        assert_eq!(Dot::inf(x_axis), Dot::from_f64s(1.0, 0.0, 0.0));
        assert_eq!(Dot::inf(y_axis), Dot::from_f64s(0.0, -1.0, 0.0));
        assert_eq!(x_axis.inf_point(), Dot::inf(x_axis));
        // 方向点在直线上、且在无穷远直线上
        let inf = Dot::inf(x_axis);
        assert!(inf.is_at_infinity());
        assert!(inf.is_on_line(x_axis, 0.0));
        assert!(inf.is_on_line(Line::INF, 0.0));
    }

    // ---- 射影相等 / 代表元运算 ----

    #[test]
    fn projective_equality() {
        let p = Dot::new(c(1.0, 0.5), c(-2.0, 1.0), c(3.0, 0.0));
        // 与整体缩放无关（含复数缩放与反号）
        assert!(p.approx_eq(&p.scaled(c(2.0, -1.0)), 0.0));
        assert!(p.approx_eq(&p.scaled(c(-1.0, 0.0)), 0.0));
        assert!(p.approx_eq(&p.scaled(Complex::I), 0.0));
        let q = Dot::new(c(1.1, 0.5), c(-2.0, 1.0), c(3.0, 0.0));
        assert!(p.approx_eq(&q, 1e-1));
        assert!(!p.approx_eq(&q, 1e-6));
    }

    #[test]
    fn normalize_scaled_conj() {
        let p = Dot::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        let n = p.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert!(n.approx_eq(&p, EPS)); // 归一化不改变射影点
        // 无效点无法归一化
        assert_eq!(Dot::from_f64s(0.0, 0.0, 0.0).normalize(), None);
        // 共轭 / 缩放
        assert_eq!(p.conj(), Dot::new(c(3.0, -1.0), c(0.0, 2.0), c(1.0, -0.5)));
        assert_eq!(
            p.scaled(c(2.0, 0.0)),
            Dot::from_vec3c(p.p.mul_complex(c(2.0, 0.0)))
        );
    }

    // ---- 谓词 / 输出 ----

    #[test]
    fn predicates_display() {
        assert!(Dot::from_f64s(0.0, 0.0, 0.0).is_zero());
        assert!(!Dot::ZERO.is_zero()); // ZERO 是齐次原点 (0:0:1)，不是无效点
        assert!(Dot::I.is_at_infinity());
        assert!(!Dot::from_f64s(1.0, 2.0, 1.0).is_at_infinity());
        assert!(Dot::from_f64s(1.0, 2.0, 1.0).is_real());
        assert!(!Dot::I.is_real());
        assert!(Dot::from_f64s(1.0, 2.0, 1.0).is_finite());
        assert!(!Dot::new(c(f64::NAN, 0.0), Complex::ZERO, Complex::ZERO).is_finite());
        // 输出
        assert_eq!(Dot::from_f64s(1.0, 2.0, 3.0).to_string(), "(1 : 2 : 3)");
        assert_eq!(Dot::ZERO.to_string(), "(0 : 0 : 1)");
        assert_eq!(Dot::I.to_string(), "(1 : 1i : 0)");
        assert_eq!(format!("{:?}", Dot::I), "Dot(1 : 1i : 0)");
    }
}
