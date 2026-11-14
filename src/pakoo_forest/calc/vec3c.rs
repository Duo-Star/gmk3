// src/pakoo_forest/calc/vec3c.rs
//! 列向量 `Vec3C` —— 3 分量复列向量（逆变分量）。
//!
//! 纯线性代数类型，与几何解耦。与 `Cov3C`（行向量）、`Mat3C`（矩阵）
//! 共同构成闭合的乘法代数：
//!
//! | 运算 | 结果 | 语义 |
//! |---|---|---|
//! | `Mat3C * Vec3C` | `Vec3C` | 线性映射作用 `M·v` |
//! | `Vec3C * Cov3C` | `Mat3C` | 外积 `v·c` |
//! | `Cov3C * Vec3C` | `Complex` | 对偶配对 `c·v` |
//! | `Cov3C * Mat3C` | `Cov3C` | 行向量变换 `c·M` |
//!
//! # 共轭约定
//!
//! - `dot`（与 `Cov3C` 的配对）是双线性，不含共轭；
//! - [`Vec3C::inner`] 是 Hermite 内积 `⟨u,v⟩ = Σ conj(uᵢ)·vᵢ`（共轭第一参数）。

use std::array;
use std::fmt;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use super::Complex;
use super::cov3c::Cov3C;
use super::matrix3c::Mat3C;

/// 3 分量复列向量，分量公开（`v[0..3]`）。
///
/// `Copy` 类型；`Default` 为零向量。
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Vec3C {
    /// 三个分量
    pub v: [Complex; 3],
}

impl Vec3C {
    /// 零向量
    pub const ZERO: Vec3C = Vec3C::new(Complex::ZERO, Complex::ZERO, Complex::ZERO);
    /// 标准基 `e₀ = (1, 0, 0)`
    pub const E0: Vec3C = Vec3C::new(Complex::ONE, Complex::ZERO, Complex::ZERO);
    /// 标准基 `e₁ = (0, 1, 0)`
    pub const E1: Vec3C = Vec3C::new(Complex::ZERO, Complex::ONE, Complex::ZERO);
    /// 标准基 `e₂ = (0, 0, 1)`
    pub const E2: Vec3C = Vec3C::new(Complex::ZERO, Complex::ZERO, Complex::ONE);

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 由三个分量构造。
    #[inline]
    pub const fn new(x: Complex, y: Complex, z: Complex) -> Vec3C {
        Vec3C { v: [x, y, z] }
    }

    /// 由数组构造。
    #[inline]
    pub const fn from_array(v: [Complex; 3]) -> Vec3C {
        Vec3C { v }
    }

    /// 由三个实数构造（各分量虚部为 0）。
    #[inline]
    pub const fn from_f64s(x: f64, y: f64, z: f64) -> Vec3C {
        Vec3C::new(
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
    pub fn map(&self, f: impl FnMut(Complex) -> Complex) -> Vec3C {
        Vec3C { v: self.v.map(f) }
    }

    // ------------------------------------------------------------------
    // 基本算术
    // ------------------------------------------------------------------

    /// `self + other`
    #[inline]
    #[must_use]
    pub fn add(&self, other: Vec3C) -> Vec3C {
        Vec3C {
            v: array::from_fn(|i| self.v[i] + other.v[i]),
        }
    }

    /// `self − other`
    #[inline]
    #[must_use]
    pub fn sub(&self, other: Vec3C) -> Vec3C {
        Vec3C {
            v: array::from_fn(|i| self.v[i] - other.v[i]),
        }
    }

    /// `−self`
    #[inline]
    #[must_use]
    pub fn neg(&self) -> Vec3C {
        self.map(|x| -x)
    }

    /// `self * s`（复标量）
    #[inline]
    #[must_use]
    pub fn mul_complex(&self, s: Complex) -> Vec3C {
        self.map(|x| x * s)
    }

    /// `self / s`（复标量）
    #[inline]
    #[must_use]
    pub fn div_complex(&self, s: Complex) -> Vec3C {
        self.map(|x| x / s)
    }

    /// `self * s`（实标量）
    #[inline]
    #[must_use]
    pub fn mul_f64(&self, s: f64) -> Vec3C {
        self.map(|x| x.mul_f64(s))
    }

    /// `self / s`（实标量）
    #[inline]
    #[must_use]
    pub fn div_f64(&self, s: f64) -> Vec3C {
        self.map(|x| x.div_f64(s))
    }

    // ------------------------------------------------------------------
    // 与 Cov3C / Mat3C
    // ------------------------------------------------------------------

    /// 与行向量的双线性配对（不含共轭）：`Σ v[i]·c[i]`。
    #[inline]
    #[must_use]
    pub fn dot(&self, c: Cov3C) -> Complex {
        self.v[0] * c.v[0] + self.v[1] * c.v[1] + self.v[2] * c.v[2]
    }

    /// 外积 `v·c`（3×1 乘 1×3）：`m[i][j] = v[i]·c[j]`。
    #[inline]
    #[must_use]
    pub fn outer(&self, c: Cov3C) -> Mat3C {
        Mat3C::from_outer(*self, c)
    }

    /// 叉积 `v × w`（3 维反对称双线性）。
    #[inline]
    #[must_use]
    pub fn cross(&self, w: Vec3C) -> Vec3C {
        Vec3C::new(
            self.v[1] * w.v[2] - self.v[2] * w.v[1],
            self.v[2] * w.v[0] - self.v[0] * w.v[2],
            self.v[0] * w.v[1] - self.v[1] * w.v[0],
        )
    }

    /// Hermite 内积 `⟨self, w⟩ = Σ conj(self[i])·w[i]`（共轭第一参数）。
    #[inline]
    #[must_use]
    pub fn inner(&self, w: Vec3C) -> Complex {
        self.v[0].conj() * w.v[0] + self.v[1].conj() * w.v[1] + self.v[2].conj() * w.v[2]
    }

    /// 转置为行向量（分量不变）。
    #[inline]
    #[must_use]
    pub fn transpose(&self) -> Cov3C {
        Cov3C::from_array(self.v)
    }

    /// 逐分量共轭。
    #[inline]
    #[must_use]
    pub fn conj(&self) -> Vec3C {
        self.map(Complex::conj)
    }

    /// 共轭转置（`→ Cov3C`）：`⟨self, w⟩ = self.adjoint().dot(w)`。
    #[inline]
    #[must_use]
    pub fn adjoint(&self) -> Cov3C {
        self.conj().transpose()
    }

    // ------------------------------------------------------------------
    // 范数
    // ------------------------------------------------------------------

    /// 范数平方 `Σ|vᵢ|²`。分量极大时可能溢出为 `inf`，此时请改用 [`Vec3C::norm`]。
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
    pub fn normalize(&self) -> Option<Vec3C> {
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
    pub fn approx_eq(&self, other: Vec3C, eps: f64) -> bool {
        (0..3).all(|i| self.v[i].approx_eq(other.v[i], eps))
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Vec3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}, {}]", self.v[0], self.v[1], self.v[2])
    }
}

impl fmt::Debug for Vec3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec3C({self})")
    }
}

// ---------------------------------------------------------------------------
// 索引 `v[i]`
// ---------------------------------------------------------------------------

impl Index<usize> for Vec3C {
    type Output = Complex;
    #[inline]
    fn index(&self, i: usize) -> &Complex {
        &self.v[i]
    }
}

impl IndexMut<usize> for Vec3C {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut Complex {
        &mut self.v[i]
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：向量 ⊛ 向量 / 标量 / Cov3C
// ---------------------------------------------------------------------------

impl Add for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn add(self, rhs: Vec3C) -> Vec3C {
        Vec3C::add(&self, rhs)
    }
}

impl Sub for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn sub(self, rhs: Vec3C) -> Vec3C {
        Vec3C::sub(&self, rhs)
    }
}

impl Neg for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn neg(self) -> Vec3C {
        Vec3C::neg(&self)
    }
}

impl AddAssign for Vec3C {
    #[inline]
    fn add_assign(&mut self, rhs: Vec3C) {
        *self = Vec3C::add(self, rhs);
    }
}

impl SubAssign for Vec3C {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec3C) {
        *self = Vec3C::sub(self, rhs);
    }
}

impl Mul<f64> for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn mul(self, rhs: f64) -> Vec3C {
        Vec3C::mul_f64(&self, rhs)
    }
}

impl Mul<Vec3C> for f64 {
    type Output = Vec3C;
    #[inline]
    fn mul(self, rhs: Vec3C) -> Vec3C {
        Vec3C::mul_f64(&rhs, self)
    }
}

impl Div<f64> for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn div(self, rhs: f64) -> Vec3C {
        Vec3C::div_f64(&self, rhs)
    }
}

impl Mul<Complex> for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn mul(self, rhs: Complex) -> Vec3C {
        Vec3C::mul_complex(&self, rhs)
    }
}

impl Mul<Vec3C> for Complex {
    type Output = Vec3C;
    #[inline]
    fn mul(self, rhs: Vec3C) -> Vec3C {
        Vec3C::mul_complex(&rhs, self)
    }
}

impl Div<Complex> for Vec3C {
    type Output = Vec3C;
    #[inline]
    fn div(self, rhs: Complex) -> Vec3C {
        Vec3C::div_complex(&self, rhs)
    }
}

impl MulAssign<f64> for Vec3C {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        *self = Vec3C::mul_f64(self, rhs);
    }
}

impl MulAssign<Complex> for Vec3C {
    #[inline]
    fn mul_assign(&mut self, rhs: Complex) {
        *self = Vec3C::mul_complex(self, rhs);
    }
}

impl DivAssign<f64> for Vec3C {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        *self = Vec3C::div_f64(self, rhs);
    }
}

impl DivAssign<Complex> for Vec3C {
    #[inline]
    fn div_assign(&mut self, rhs: Complex) {
        *self = Vec3C::div_complex(self, rhs);
    }
}

/// `Vec3C * Cov3C → Mat3C`：外积。
impl Mul<Cov3C> for Vec3C {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: Cov3C) -> Mat3C {
        Mat3C::from_outer(self, rhs)
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
        let v = Vec3C::new(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0));
        assert_eq!(v.get(0), c(1.0, 2.0));
        assert_eq!(v[1], c(3.0, 4.0));
        assert_eq!(v.to_array(), [c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0)]);
        assert_eq!(Vec3C::from_array(v.to_array()), v);
        assert_eq!(
            Vec3C::from_f64s(1.0, 2.0, 3.0),
            Vec3C::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(Vec3C::ZERO, Vec3C::from_f64s(0.0, 0.0, 0.0));
        assert_eq!(
            Vec3C::E0.to_array(),
            [Complex::ONE, Complex::ZERO, Complex::ZERO]
        );
        assert_eq!(
            Vec3C::E1.to_array(),
            [Complex::ZERO, Complex::ONE, Complex::ZERO]
        );
        assert_eq!(
            Vec3C::E2.to_array(),
            [Complex::ZERO, Complex::ZERO, Complex::ONE]
        );

        let mut w = v;
        w.set(2, c(9.0, 9.0));
        assert_eq!(w.get(2), c(9.0, 9.0));
        w[0] = c(-1.0, 0.0);
        assert_eq!(w.get(0), c(-1.0, 0.0));
        assert_eq!(
            v.map(|x| x.conj()),
            Vec3C::new(c(1.0, -2.0), c(3.0, -4.0), c(5.0, -6.0))
        );
    }

    // ---- 基本算术 ----

    #[test]
    fn arithmetic() {
        let v = Vec3C::from_f64s(1.0, 2.0, 3.0);
        let w = Vec3C::from_f64s(4.0, 5.0, 6.0);
        assert_eq!(v.add(w), Vec3C::from_f64s(5.0, 7.0, 9.0));
        assert_eq!(v + w, v.add(w));
        assert_eq!(v.sub(w), Vec3C::from_f64s(-3.0, -3.0, -3.0));
        assert_eq!(v - w, v.sub(w));
        assert_eq!(v.neg(), Vec3C::from_f64s(-1.0, -2.0, -3.0));
        assert_eq!(-v, v.neg());
        assert_eq!(v.add(v.neg()), Vec3C::ZERO);
        assert_eq!(v.sub(v), Vec3C::ZERO);
    }

    #[test]
    fn scalar_ops() {
        let v = Vec3C::from_f64s(1.0, 2.0, 3.0);
        assert_eq!(v.mul_f64(2.0), Vec3C::from_f64s(2.0, 4.0, 6.0));
        assert_eq!(v * 2.0, v.mul_f64(2.0));
        assert_eq!(2.0 * v, v.mul_f64(2.0));
        assert_eq!(v / 2.0, v.mul_f64(0.5));
        assert_eq!(
            v.mul_complex(Complex::I),
            Vec3C::new(c(0.0, 1.0), c(0.0, 2.0), c(0.0, 3.0))
        );
        assert_eq!(v * Complex::I, v.mul_complex(Complex::I));
        assert_eq!(Complex::I * v, v.mul_complex(Complex::I));
        assert_eq!(v.div_complex(Complex::I), v.mul_complex(c(0.0, -1.0)));
        assert_eq!(v / Complex::I, v.mul_complex(c(0.0, -1.0)));
        let mut x = v;
        x += v;
        assert_eq!(x, v.mul_f64(2.0));
        x -= v;
        assert_eq!(x, v);
        x *= 2.0;
        x /= 2.0;
        assert_eq!(x, v);
        x *= Complex::I;
        x /= Complex::I;
        assert_eq!(x, v);
    }

    // ---- 与 Cov3C / Mat3C 的代数 ----

    #[test]
    fn pairing_dot_outer() {
        let v = Vec3C::new(c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.0));
        let cov = Cov3C::new(c(0.5, 0.25), c(1.0, 0.0), c(0.0, -1.0));
        // 配对：Σ v[i]·c[i]
        let expected = v.v[0] * cov.v[0] + v.v[1] * cov.v[1] + v.v[2] * cov.v[2];
        assert_eq!(v.dot(cov), expected);
        assert_eq!(cov.dot(v), expected); // 双线性配对与方向无关
        assert_eq!(cov * v, expected); // 运算符 == dot
        // 外积：(v·c)[i][j] = v[i]·c[j]
        let outer = v * cov;
        assert_eq!(outer, Mat3C::from_outer(v, cov));
        assert_eq!(outer, v.outer(cov));
        assert_eq!(outer.get(0, 1), v.v[0] * cov.v[1]);
        // (v·c)·w = v·(c·w)
        let w = Vec3C::from_f64s(1.0, -1.0, 2.0);
        assert_eq!(outer.mul_vec(w), v.mul_complex(cov.dot(w)));
    }

    #[test]
    fn mat_pairing_consistency() {
        let m = Mat3C::new([
            [c(0.0, 1.0), c(1.0, 0.0), c(0.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 1.0), c(1.0, 0.0)],
            [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 1.0)],
        ]);
        let v = Vec3C::new(c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.0));
        let cov = Cov3C::new(c(0.5, 0.25), c(1.0, 0.0), c(0.0, -1.0));
        // 结合律：(m·v) 与 c 配对 == v 与 (c·m) 配对
        assert_approx(m.mul_vec(v).dot(cov), v.dot(cov.mul_mat(&m)));
        // 运算符链：c*m*v 两种结合顺序一致
        assert_approx((cov * m) * v, cov * (m * v));
        // m*v 与 c*m 的运算符一致性
        assert_eq!(m * v, m.mul_vec(v));
        assert_eq!(cov * m, cov.mul_mat(&m));
    }

    #[test]
    fn cross_product() {
        let v = Vec3C::from_f64s(1.0, 2.0, 3.0);
        let w = Vec3C::from_f64s(4.0, 5.0, 6.0);
        // v × w = [2·6−3·5, 3·4−1·6, 1·5−2·4] = [−3, 6, −3]
        assert_eq!(v.cross(w), Vec3C::from_f64s(-3.0, 6.0, -3.0));
        assert_eq!(v.cross(v), Vec3C::ZERO); // 自叉积为零
        assert_eq!(v.cross(w), w.cross(v).neg()); // 反交换
        assert_eq!(v.cross(w), Mat3C::from_cross(v).mul_vec(w)); // 与叉积矩阵一致
        // 双线性：v×(w+u) = v×w + v×u
        let u = Vec3C::from_f64s(-1.0, 0.0, 2.0);
        assert_eq!(v.cross(w.add(u)), v.cross(w).add(v.cross(u)));
    }

    // ---- 内积 / 范数 ----

    #[test]
    fn inner_norm() {
        let v = Vec3C::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        let w = Vec3C::new(c(0.5, 0.0), c(2.0, 1.0), c(-1.0, 0.5));
        // ⟨v, v⟩ = ‖v‖²（实数）
        assert_eq!(v.inner(v), c(v.norm_sqr(), 0.0));
        assert_eq!(
            v.norm_sqr(),
            v.v[0].norm_sqr() + v.v[1].norm_sqr() + v.v[2].norm_sqr()
        );
        // ⟨v, w⟩ = v†·w（adjoint 配对）
        assert_approx(v.inner(w), v.adjoint().dot(w));
        // ⟨v, w⟩ = conj(⟨w, v⟩)
        assert_approx(v.inner(w), w.inner(v).conj());
        // 范数
        assert_eq!(Vec3C::from_f64s(3.0, 4.0, 0.0).norm(), 5.0);
        let n = v.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert_eq!(Vec3C::ZERO.normalize(), None);
    }

    // ---- 转置 / 共轭 / 伴随 ----

    #[test]
    fn transpose_conj_adjoint() {
        let v = Vec3C::new(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0));
        let t = v.transpose();
        assert_eq!(t.to_array(), v.to_array());
        assert_eq!(t.transpose(), v); // 往返
        assert_eq!(
            v.conj(),
            Vec3C::new(c(1.0, -2.0), c(3.0, -4.0), c(5.0, -6.0))
        );
        let a = v.adjoint();
        assert_eq!(a, v.conj().transpose());
        assert_eq!(a.transpose(), v.conj());
    }

    // ---- 谓词 / 近似比较 / 输出 ----

    #[test]
    fn predicates_display() {
        assert!(Vec3C::ZERO.is_zero());
        assert!(!Vec3C::E0.is_zero());
        assert!(Vec3C::E0.is_finite());
        assert!(!Vec3C::new(c(f64::NAN, 0.0), Complex::ZERO, Complex::ZERO).is_finite());
        assert!(Vec3C::from_f64s(1.0, 1e-14, 0.0).approx_eq(Vec3C::E0, 1e-12));
        assert!(!Vec3C::from_f64s(1.0, 1e-3, 0.0).approx_eq(Vec3C::E0, 1e-12));

        let v = Vec3C::new(c(3.0, 4.0), c(0.0, 5.0), c(-3.0, 0.0));
        assert_eq!(v.to_string(), "[3 + 4i, 5i, -3]");
        assert_eq!(format!("{v:?}"), "Vec3C([3 + 4i, 5i, -3])");
    }
}
