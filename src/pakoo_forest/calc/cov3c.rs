// src/pakoo_forest/calc/cov3c.rs
//! 行向量 `Cov3C` —— 3 分量复行向量（协变分量）。
//!
//! 纯线性代数类型，与几何解耦。与 `Vec3C`（列向量）、`Mat3C`（矩阵）
//! 共同构成闭合的乘法代数：
//!
//! | 运算 | 结果 | 语义 |
//! |---|---|---|
//! | `Cov3C * Vec3C` | `Complex` | 对偶配对 `c·v` |
//! | `Cov3C * Mat3C` | `Cov3C` | 行向量变换 `c·M` |
//! | `Vec3C * Cov3C` | `Mat3C` | 外积 `v·c` |
//! | `Mat3C * Vec3C` | `Vec3C` | 线性映射作用 `M·v` |
//!
//! # 共轭约定
//!
//! `dot`（与 `Vec3C` 的配对）是双线性，不含共轭；Hermite 内积见 `Vec3C::inner`。

use std::array;
use std::fmt;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use super::Complex;
use super::mat3c::Mat3C;
use super::vec3c::Vec3C;

/// 3 分量复行向量，分量公开（`v[0..3]`）。
///
/// `Copy` 类型；`Default` 为零向量。
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Cov3C {
    /// 三个分量
    pub v: [Complex; 3],
}

impl Cov3C {
    /// 零向量
    pub const ZERO: Cov3C = Cov3C::new(Complex::ZERO, Complex::ZERO, Complex::ZERO);
    /// 标准基 `e₀ = (1, 0, 0)`
    pub const E0: Cov3C = Cov3C::new(Complex::ONE, Complex::ZERO, Complex::ZERO);
    /// 标准基 `e₁ = (0, 1, 0)`
    pub const E1: Cov3C = Cov3C::new(Complex::ZERO, Complex::ONE, Complex::ZERO);
    /// 标准基 `e₂ = (0, 0, 1)`
    pub const E2: Cov3C = Cov3C::new(Complex::ZERO, Complex::ZERO, Complex::ONE);

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 由三个分量构造。
    #[inline]
    pub const fn new(x: Complex, y: Complex, z: Complex) -> Cov3C {
        Cov3C { v: [x, y, z] }
    }

    /// 由数组构造。
    #[inline]
    pub const fn from_array(v: [Complex; 3]) -> Cov3C {
        Cov3C { v }
    }

    /// 由三个实数构造（各分量虚部为 0）。
    #[inline]
    pub const fn from_f64s(x: f64, y: f64, z: f64) -> Cov3C {
        Cov3C::new(
            Complex::from_f64(x),
            Complex::from_f64(y),
            Complex::from_f64(z),
        )
    }

    /// 转为数组。
    #[inline]
    pub const fn to_array(self) -> [Complex; 3] {
        self.v
    }

    // ------------------------------------------------------------------
    // 访问
    // ------------------------------------------------------------------

    /// 读取第 `i` 个分量（`0..3`，越界 panic）。
    #[inline]
    pub fn get(&self, i: usize) -> Complex {
        self.v[i]
    }

    /// 写入第 `i` 个分量（`0..3`，越界 panic）。
    #[inline]
    pub fn set(&mut self, i: usize, x: Complex) {
        self.v[i] = x;
    }

    /// 逐分量映射。
    #[inline]
    pub fn map(&self, f: impl FnMut(Complex) -> Complex) -> Cov3C {
        Cov3C { v: self.v.map(f) }
    }

    // ------------------------------------------------------------------
    // 基本算术
    // ------------------------------------------------------------------

    /// `self + other`
    #[inline]
    #[must_use]
    pub fn add(&self, other: Cov3C) -> Cov3C {
        Cov3C {
            v: array::from_fn(|i| self.v[i] + other.v[i]),
        }
    }

    /// `self − other`
    #[inline]
    #[must_use]
    pub fn sub(&self, other: Cov3C) -> Cov3C {
        Cov3C {
            v: array::from_fn(|i| self.v[i] - other.v[i]),
        }
    }

    /// `−self`
    #[inline]
    #[must_use]
    pub fn neg(&self) -> Cov3C {
        self.map(|x| -x)
    }

    /// `self * s`（复标量）
    #[inline]
    #[must_use]
    pub fn mul_complex(&self, s: Complex) -> Cov3C {
        self.map(|x| x * s)
    }

    /// `self / s`（复标量）
    #[inline]
    #[must_use]
    pub fn div_complex(&self, s: Complex) -> Cov3C {
        self.map(|x| x / s)
    }

    /// `self * s`（实标量）
    #[inline]
    #[must_use]
    pub fn mul_f64(&self, s: f64) -> Cov3C {
        self.map(|x| x.mul_f64(s))
    }

    /// `self / s`（实标量）
    #[inline]
    #[must_use]
    pub fn div_f64(&self, s: f64) -> Cov3C {
        self.map(|x| x.div_f64(s))
    }

    // ------------------------------------------------------------------
    // 与 Vec3C / Mat3C
    // ------------------------------------------------------------------

    /// 与列向量的双线性配对（不含共轭）：`Σ c[i]·v[i]`。
    #[inline]
    #[must_use]
    pub fn dot(&self, v: Vec3C) -> Complex {
        self.v[0] * v.v[0] + self.v[1] * v.v[1] + self.v[2] * v.v[2]
    }

    /// 行向量左乘矩阵：`c·M`（结果仍为行向量）。
    #[inline]
    #[must_use]
    pub fn mul_mat(&self, m: &Mat3C) -> Cov3C {
        Mat3C::mul_cov(m, *self)
    }

    /// 转置为列向量（分量不变）。
    #[inline]
    #[must_use]
    pub fn transpose(&self) -> Vec3C {
        Vec3C::from_array(self.v)
    }

    /// 逐分量共轭。
    #[inline]
    #[must_use]
    pub fn conj(&self) -> Cov3C {
        self.map(Complex::conj)
    }

    /// 共轭转置（`→ Vec3C`）。
    #[inline]
    #[must_use]
    pub fn adjoint(&self) -> Vec3C {
        self.conj().transpose()
    }

    // ------------------------------------------------------------------
    // 范数
    // ------------------------------------------------------------------

    /// 范数平方 `Σ|vᵢ|²`。分量极大时可能溢出为 `inf`，此时请改用 [`Cov3C::norm`]。
    #[inline]
    #[must_use]
    pub fn norm_sqr(&self) -> f64 {
        self.v.iter().map(|x| x.norm_sqr()).sum()
    }

    /// 范数 `√(Σ|vᵢ|²)`。用 `hypot` 累积，无中间上溢/下溢。
    #[inline]
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.v[0]
            .abs()
            .hypot(self.v[1].abs())
            .hypot(self.v[2].abs())
    }

    /// 归一化为单位范数向量。零向量或范数非有限时返回 `None`。
    #[inline]
    pub fn normalize(&self) -> Option<Cov3C> {
        let n = self.norm();
        if n == 0.0 || !n.is_finite() {
            return None;
        }
        Some(self.mul_f64(1.0 / n))
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 是否为零向量。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.v.iter().all(|x| *x == Complex::ZERO)
    }

    /// 所有分量均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.v.iter().all(|x| x.is_finite())
    }

    /// 逐分量近似相等（混合绝对/相对误差 `eps`）。
    #[inline]
    pub fn approx_eq(&self, other: Cov3C, eps: f64) -> bool {
        (0..3).all(|i| self.v[i].approx_eq(other.v[i], eps))
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Cov3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}, {}]", self.v[0], self.v[1], self.v[2])
    }
}

impl fmt::Debug for Cov3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cov3C({self})")
    }
}

// ---------------------------------------------------------------------------
// 索引 `c[i]`
// ---------------------------------------------------------------------------

impl Index<usize> for Cov3C {
    type Output = Complex;
    #[inline]
    fn index(&self, i: usize) -> &Complex {
        &self.v[i]
    }
}

impl IndexMut<usize> for Cov3C {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut Complex {
        &mut self.v[i]
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：向量 ⊛ 向量 / 标量 / Vec3C / Mat3C
// ---------------------------------------------------------------------------

impl Add for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn add(self, rhs: Cov3C) -> Cov3C {
        Cov3C::add(&self, rhs)
    }
}

impl Sub for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn sub(self, rhs: Cov3C) -> Cov3C {
        Cov3C::sub(&self, rhs)
    }
}

impl Neg for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn neg(self) -> Cov3C {
        Cov3C::neg(&self)
    }
}

impl AddAssign for Cov3C {
    #[inline]
    fn add_assign(&mut self, rhs: Cov3C) {
        *self = Cov3C::add(self, rhs);
    }
}

impl SubAssign for Cov3C {
    #[inline]
    fn sub_assign(&mut self, rhs: Cov3C) {
        *self = Cov3C::sub(self, rhs);
    }
}

impl Mul<f64> for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn mul(self, rhs: f64) -> Cov3C {
        Cov3C::mul_f64(&self, rhs)
    }
}

impl Mul<Cov3C> for f64 {
    type Output = Cov3C;
    #[inline]
    fn mul(self, rhs: Cov3C) -> Cov3C {
        Cov3C::mul_f64(&rhs, self)
    }
}

impl Div<f64> for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn div(self, rhs: f64) -> Cov3C {
        Cov3C::div_f64(&self, rhs)
    }
}

impl Mul<Complex> for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn mul(self, rhs: Complex) -> Cov3C {
        Cov3C::mul_complex(&self, rhs)
    }
}

impl Mul<Cov3C> for Complex {
    type Output = Cov3C;
    #[inline]
    fn mul(self, rhs: Cov3C) -> Cov3C {
        Cov3C::mul_complex(&rhs, self)
    }
}

impl Div<Complex> for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn div(self, rhs: Complex) -> Cov3C {
        Cov3C::div_complex(&self, rhs)
    }
}

impl MulAssign<f64> for Cov3C {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        *self = Cov3C::mul_f64(self, rhs);
    }
}

impl MulAssign<Complex> for Cov3C {
    #[inline]
    fn mul_assign(&mut self, rhs: Complex) {
        *self = Cov3C::mul_complex(self, rhs);
    }
}

impl DivAssign<f64> for Cov3C {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        *self = Cov3C::div_f64(self, rhs);
    }
}

impl DivAssign<Complex> for Cov3C {
    #[inline]
    fn div_assign(&mut self, rhs: Complex) {
        *self = Cov3C::div_complex(self, rhs);
    }
}

/// `Cov3C * Vec3C → Complex`：对偶配对。
impl Mul<Vec3C> for Cov3C {
    type Output = Complex;
    #[inline]
    fn mul(self, rhs: Vec3C) -> Complex {
        Cov3C::dot(&self, rhs)
    }
}

/// `Cov3C * Mat3C → Cov3C`：行向量变换 `c·M`。
impl Mul<Mat3C> for Cov3C {
    type Output = Cov3C;
    #[inline]
    fn mul(self, rhs: Mat3C) -> Cov3C {
        Cov3C::mul_mat(&self, &rhs)
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{Cov3C, Mat3C, Vec3C};
    use crate::pakoo_forest::calc::Complex;

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    fn assert_approx(a: Complex, b: Complex) {
        assert!(a.approx_eq(b, EPS), "近似断言失败：{a} ≈ {b}");
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let c3 = Cov3C::new(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0));
        assert_eq!(c3.get(0), c(1.0, 2.0));
        assert_eq!(c3[1], c(3.0, 4.0));
        assert_eq!(c3.to_array(), [c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0)]);
        assert_eq!(Cov3C::from_array(c3.to_array()), c3);
        assert_eq!(
            Cov3C::from_f64s(1.0, 2.0, 3.0),
            Cov3C::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(Cov3C::ZERO, Cov3C::from_f64s(0.0, 0.0, 0.0));
        assert_eq!(
            Cov3C::E0.to_array(),
            [Complex::ONE, Complex::ZERO, Complex::ZERO]
        );

        let mut w = c3;
        w.set(2, c(9.0, 9.0));
        assert_eq!(w.get(2), c(9.0, 9.0));
        w[0] = c(-1.0, 0.0);
        assert_eq!(w.get(0), c(-1.0, 0.0));
        assert_eq!(
            c3.map(|x| x.conj()),
            Cov3C::new(c(1.0, -2.0), c(3.0, -4.0), c(5.0, -6.0))
        );
    }

    // ---- 基本算术 ----

    #[test]
    fn arithmetic() {
        let a = Cov3C::from_f64s(1.0, 2.0, 3.0);
        let b = Cov3C::from_f64s(4.0, 5.0, 6.0);
        assert_eq!(a.add(b), Cov3C::from_f64s(5.0, 7.0, 9.0));
        assert_eq!(a + b, a.add(b));
        assert_eq!(a.sub(b), Cov3C::from_f64s(-3.0, -3.0, -3.0));
        assert_eq!(a - b, a.sub(b));
        assert_eq!(a.neg(), Cov3C::from_f64s(-1.0, -2.0, -3.0));
        assert_eq!(-a, a.neg());
        assert_eq!(a.add(a.neg()), Cov3C::ZERO);
        assert_eq!(a.sub(a), Cov3C::ZERO);
    }

    #[test]
    fn scalar_ops() {
        let a = Cov3C::from_f64s(1.0, 2.0, 3.0);
        assert_eq!(a.mul_f64(2.0), Cov3C::from_f64s(2.0, 4.0, 6.0));
        assert_eq!(a * 2.0, a.mul_f64(2.0));
        assert_eq!(2.0 * a, a.mul_f64(2.0));
        assert_eq!(a / 2.0, a.mul_f64(0.5));
        assert_eq!(
            a.mul_complex(Complex::I),
            Cov3C::new(c(0.0, 1.0), c(0.0, 2.0), c(0.0, 3.0))
        );
        assert_eq!(a * Complex::I, a.mul_complex(Complex::I));
        assert_eq!(Complex::I * a, a.mul_complex(Complex::I));
        let mut x = a;
        x += a;
        assert_eq!(x, a.mul_f64(2.0));
        x -= a;
        assert_eq!(x, a);
        x *= 2.0;
        x /= 2.0;
        assert_eq!(x, a);
    }

    // ---- 与 Vec3C / Mat3C 的代数 ----

    #[test]
    fn pairing() {
        let cov = Cov3C::new(c(0.5, 0.25), c(1.0, 0.0), c(0.0, -1.0));
        let v = Vec3C::new(c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.0));
        let expected = cov.v[0] * v.v[0] + cov.v[1] * v.v[1] + cov.v[2] * v.v[2];
        assert_eq!(cov.dot(v), expected);
        assert_eq!(v.dot(cov), expected); // 与方向无关
        assert_eq!(cov * v, expected); // 运算符 == dot
    }

    #[test]
    fn mul_mat() {
        let cov = Cov3C::from_f64s(1.0, 2.0, 3.0);
        // 单位矩阵：c·I = c
        assert_eq!(cov.mul_mat(&Mat3C::IDENTITY), cov);
        assert_eq!(cov * Mat3C::IDENTITY, cov);
        // 对角矩阵逐分量缩放
        let d = Mat3C::from_diag([c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)]);
        assert_eq!(cov.mul_mat(&d), Cov3C::from_f64s(2.0, 6.0, 12.0));
        assert_eq!(cov * d, cov.mul_mat(&d));
        // 与 Mat3C::mul_cov 一致
        assert_eq!(cov.mul_mat(&d), d.mul_cov(cov));
        // 结合律：(c·A)·B = c·(A·B)
        let a = Mat3C::new([
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
            [c(7.0, 0.0), c(8.0, 0.0), c(10.0, 0.0)],
        ]);
        let b = Mat3C::new([
            [c(0.0, 1.0), c(1.0, 0.0), c(0.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 1.0), c(1.0, 0.0)],
            [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 1.0)],
        ]);
        let lhs = cov.mul_mat(&a).mul_mat(&b);
        let rhs = cov.mul_mat(&a.mul(&b));
        assert!(lhs.approx_eq(rhs, EPS), "结合律失败：{lhs} vs {rhs}");
        // 三重积结合：(c·A)·v == c·(A·v)
        let v = Vec3C::from_f64s(1.0, -1.0, 2.0);
        assert_approx((cov * a) * v, cov * (a * v));
    }

    // ---- 转置 / 共轭 / 伴随 ----

    #[test]
    fn transpose_conj_adjoint() {
        let c3 = Cov3C::new(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0));
        let t = c3.transpose();
        assert_eq!(t.to_array(), c3.to_array());
        assert_eq!(t.transpose(), c3); // 往返
        assert_eq!(
            c3.conj(),
            Cov3C::new(c(1.0, -2.0), c(3.0, -4.0), c(5.0, -6.0))
        );
        let a = c3.adjoint();
        assert_eq!(a, c3.conj().transpose());
        assert_eq!(a.transpose(), c3.conj());
    }

    // ---- 范数 / 谓词 / 输出 ----

    #[test]
    fn norm_predicates_display() {
        let c3 = Cov3C::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        assert_eq!(
            c3.norm_sqr(),
            c3.v[0].norm_sqr() + c3.v[1].norm_sqr() + c3.v[2].norm_sqr()
        );
        assert_eq!(Cov3C::from_f64s(3.0, 4.0, 0.0).norm(), 5.0);
        let n = c3.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert_eq!(Cov3C::ZERO.normalize(), None);

        assert!(Cov3C::ZERO.is_zero());
        assert!(!Cov3C::E0.is_zero());
        assert!(Cov3C::E0.is_finite());
        assert!(Cov3C::from_f64s(1.0, 1e-14, 0.0).approx_eq(Cov3C::E0, 1e-12));
        assert!(!Cov3C::from_f64s(1.0, 1e-3, 0.0).approx_eq(Cov3C::E0, 1e-12));

        let d = Cov3C::new(c(3.0, 4.0), c(0.0, 5.0), c(-3.0, 0.0));
        assert_eq!(d.to_string(), "[3 + 4i, 5i, -3]");
        assert_eq!(format!("{d:?}"), "Cov3C([3 + 4i, 5i, -3])");
    }
}
