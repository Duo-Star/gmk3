// src/pakoo_forest/calc/complex.rs
//! 复数 `Complex` —— gmk3 数值内核的基础标量类型。
//!
//! # 设计要点
//!
//! - **按值、零分配**：`Complex` 是 `Copy` 的 16 字节结构，全部运算内联、无堆分配。
//! - **稳健的除法**：`div_complex` 与 `recip` 采用缩放（Smith）算法，
//!   不会因中间量 `|w|²` 上溢/下溢而损失精度；除以零得到 `NaN`。
//! - **主值分支**：`ln`、`sqrt`、`pow_*`、反三角/反双曲函数均取标准主值分支。
//! - **与 `f64` 的混合运算**：`add_f64` / `sub_f64` / `mul_f64` / `div_f64`（以及
//!   `f64_sub` / `f64_div`），并重载了对应的运算符（`Complex ± f64`、`f64 ± Complex`）。
//! - **调试输出**：实现 `Display`（形如 `3 + 4i`）与 `Debug`（形如 `Complex(3 + 4i)`）。
//!
//! # 边界行为
//!
//! - `x / 0` 与 `1 / 0` 返回 `NaN + NaN i`（与 IEEE 浮点语义一致）；
//! - `ln(0) = -∞ + NaN i`；`0⁰` 返回 `NaN`；
//! - 无穷/NaN 会自然地在运算中传播，可用 `is_finite` / `is_nan` / `is_infinite` 检测。

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// 复数 `a + bi`。
///
/// 字段公开以利于热路径直接访问（`z.re` / `z.im`），
/// 同时提供同名访问方法 `re()` / `im()`。
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Complex {
    /// 实部
    pub re: f64,
    /// 虚部
    pub im: f64,
}

impl Complex {
    /// `0 + 0i`
    pub const ZERO: Complex = Complex::new(0.0, 0.0);
    /// `1 + 0i`
    pub const ONE: Complex = Complex::new(1.0, 0.0);
    pub const NEG_ONE: Complex = Complex::new(-1.0, 0.0);
    /// `i`
    pub const I: Complex = Complex::new(0.0, 1.0);
    pub const NEG_I: Complex = Complex::new(0.0, -1.0);

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 由实部与虚部构造复数。
    #[inline]
    pub const fn new(re: f64, im: f64) -> Complex {
        Complex { re, im }
    }

    /// 由实数构造，虚部为 `0`
    #[inline]
    pub const fn from_f64(re: f64) -> Complex {
        Complex::new(re, 0.0)
    }

    /// 由极坐标 `(r, θ)` 构造 `r·e^{iθ}`。
    #[inline]
    #[must_use]
    pub fn from_polar(r: f64, theta: f64) -> Complex {
        let (s, c) = theta.sin_cos();
        Complex::new(r * c, r * s)
    }

    /// 实部。
    #[inline]
    pub const fn re(self) -> f64 {
        self.re
    }

    /// 虚部。
    #[inline]
    pub const fn im(self) -> f64 {
        self.im
    }

    /// 极坐标 `(模, 辐角)`。
    #[inline]
    #[must_use]
    pub fn to_polar(self) -> (f64, f64) {
        (self.abs(), self.arg())
    }

    // ------------------------------------------------------------------
    // 共轭 / 模 / 辐角
    // ------------------------------------------------------------------

    /// 共轭 `a - bi`。
    #[inline]
    #[must_use]
    pub fn conj(self) -> Complex {
        Complex::new(self.re, -self.im)
    }

    /// 模平方 `|z|² = a² + b²`。
    ///
    /// 注意：`|z|` 极大（约 `> 1e154`）时会溢出为 `inf`；关心该边界时请改用
    /// [`Complex::abs`]。
    #[inline]
    #[must_use]
    pub fn norm_sqr(self) -> f64 {
        self.re.mul_add(self.re, self.im * self.im)
    }

    /// 模 `|z|`。基于 `hypot`，无中间上溢/下溢。
    #[inline]
    #[must_use]
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// 辐角主值 `arg(z) ∈ (-π, π]`。
    #[inline]
    #[must_use]
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    
    
    // ------------------------------------------------------------------
    // 基本算术（复数）
    // ------------------------------------------------------------------

    /// `self + other`
    #[inline]
    #[must_use]
    pub fn add_complex(self, other: Complex) -> Complex {
        Complex::new(self.re + other.re, self.im + other.im)
    }

    /// `self - other`
    #[inline]
    #[must_use]
    pub fn sub_complex(self, other: Complex) -> Complex {
        Complex::new(self.re - other.re, self.im - other.im)
    }

    /// `self * other`，实虚部均用 FMA 融合乘加以提高精度。
    #[inline]
    #[must_use]
    pub fn mul_complex(self, other: Complex) -> Complex {
        Complex::new(
            self.re.mul_add(other.re, -(self.im * other.im)),
            self.re.mul_add(other.im, self.im * other.re),
        )
    }

    /// `self / other`。缩放（Smith）除法：以较大分量归一化，
    /// 避免中间量 `|other|²` 上溢/下溢。`other == 0` 时返回 `NaN`。
    #[inline]
    #[must_use]
    pub fn div_complex(self, other: Complex) -> Complex {
        if other.re.abs() >= other.im.abs() {
            let r = other.im / other.re;
            let d = other.re + other.im * r;
            Complex::new((self.re + self.im * r) / d, (self.im - self.re * r) / d)
        } else {
            let r = other.re / other.im;
            let d = other.im + other.re * r;
            Complex::new((self.re * r + self.im) / d, (self.im * r - self.re) / d)
        }
    }

    // ------------------------------------------------------------------
    // 基本算术（f64）
    // ------------------------------------------------------------------

    /// `self + x`
    #[inline]
    #[must_use]
    pub fn add_f64(self, x: f64) -> Complex {
        Complex::new(self.re + x, self.im)
    }

    /// `self - x`
    #[inline]
    #[must_use]
    pub fn sub_f64(self, x: f64) -> Complex {
        Complex::new(self.re - x, self.im)
    }

    /// `self * x`
    #[inline]
    #[must_use]
    pub fn mul_f64(self, x: f64) -> Complex {
        Complex::new(self.re * x, self.im * x)
    }

    /// `self / x`
    #[inline]
    #[must_use]
    pub fn div_f64(self, x: f64) -> Complex {
        Complex::new(self.re / x, self.im / x)
    }

    /// `x - self`
    #[inline]
    #[must_use]
    pub fn f64_sub(self, x: f64) -> Complex {
        Complex::new(x - self.re, -self.im)
    }

    /// `x / self`，走缩放除法。
    #[inline]
    #[must_use]
    pub fn f64_div(self, x: f64) -> Complex {
        Complex::from_f64(x).div_complex(self)
    }

    // ------------------------------------------------------------------
    // 复合运算
    // ------------------------------------------------------------------

    /// `self² = self * self`
    #[inline]
    #[must_use]
    pub fn sqr(self) -> Complex {
        self.mul_complex(self)
    }

    /// 倒数 `1 / self`。缩放算法；`self == 0` 时返回 `NaN`。
    #[inline]
    #[must_use]
    pub fn recip(self) -> Complex {
        if self.re.abs() >= self.im.abs() {
            let r = self.im / self.re;
            let d = self.re + self.im * r;
            Complex::new(1.0 / d, -r / d)
        } else {
            let r = self.re / self.im;
            let d = self.im + self.re * r;
            Complex::new(r / d, -1.0 / d)
        }
    }

    /// 绕原点旋转 `angle` 弧度：`self * e^{i·angle}`。
    #[inline]
    #[must_use]
    pub fn rotate(self, angle: f64) -> Complex {
        let (s, c) = angle.sin_cos();
        Complex::new(self.re * c - self.im * s, self.re * s + self.im * c)
    }

    // ------------------------------------------------------------------
    // 指数 / 对数 / 幂 / 平方根
    // ------------------------------------------------------------------

    /// 指数 `e^z = e^a (cos b + i sin b)`。
    #[inline]
    #[must_use]
    pub fn exp(self) -> Complex {
        let e = self.re.exp();
        Complex::new(e * self.im.cos(), e * self.im.sin())
    }

    /// 自然对数主值 `ln|z| + i·arg(z)`。
    #[inline]
    #[must_use]
    pub fn ln(self) -> Complex {
        Complex::new(self.abs().ln(), self.arg())
    }

    /// 幂 `self^w = exp(w·ln self)`，主值。
    #[inline]
    #[must_use]
    pub fn pow_complex(self, w: Complex) -> Complex {
        (w * self.ln()).exp()
    }

    /// 幂 `self^x`（实数指数），主值。
    #[inline]
    #[must_use]
    pub fn pow_f64(self, x: f64) -> Complex {
        self.ln().mul_f64(x).exp()
    }

    /// 平方根主值：`Re √z ≥ 0`（纯负实数时虚部为正）。
    #[inline]
    #[must_use]
    pub fn sqrt(self) -> Complex {
        if self.im == 0.0 {
            if self.re >= 0.0 {
                return Complex::new(self.re.sqrt(), 0.0);
            }
            return Complex::new(0.0, (-self.re).sqrt());
        }
        let r = self.abs();
        let re = ((r + self.re) * 0.5).sqrt();
        let im = ((r - self.re) * 0.5).sqrt().copysign(self.im);
        Complex::new(re, im)
    }

    // ------------------------------------------------------------------
    // 三角 / 双曲
    // ------------------------------------------------------------------

    /// 正弦 `sin z = sin a·cosh b + i·cos a·sinh b`
    #[inline]
    #[must_use]
    pub fn sin(self) -> Complex {
        Complex::new(
            self.re.sin() * self.im.cosh(),
            self.re.cos() * self.im.sinh(),
        )
    }

    /// 余弦 `cos z = cos a·cosh b − i·sin a·sinh b`
    #[inline]
    #[must_use]
    pub fn cos(self) -> Complex {
        Complex::new(
            self.re.cos() * self.im.cosh(),
            -self.re.sin() * self.im.sinh(),
        )
    }

    /// 正切 `tan z = sin z / cos z`
    #[inline]
    #[must_use]
    pub fn tan(self) -> Complex {
        self.sin().div_complex(self.cos())
    }

    /// 双曲正弦 `sinh z = sinh a·cos b + i·cosh a·sin b`
    #[inline]
    #[must_use]
    pub fn sinh(self) -> Complex {
        Complex::new(
            self.re.sinh() * self.im.cos(),
            self.re.cosh() * self.im.sin(),
        )
    }

    /// 双曲余弦 `cosh z = cosh a·cos b + i·sinh a·sin b`
    #[inline]
    #[must_use]
    pub fn cosh(self) -> Complex {
        Complex::new(
            self.re.cosh() * self.im.cos(),
            self.re.sinh() * self.im.sin(),
        )
    }

    /// 双曲正切 `tanh z = sinh z / cosh z`
    #[inline]
    #[must_use]
    pub fn tanh(self) -> Complex {
        self.sinh().div_complex(self.cosh())
    }

    // ------------------------------------------------------------------
    // 反三角 / 反双曲
    // ------------------------------------------------------------------

    /// 反正弦 `asin z = −i·ln(iz + √(1−z²))`，主值。
    #[inline]
    #[must_use]
    pub fn asin(self) -> Complex {
        -Complex::I * (Complex::I * self + (Complex::ONE - self.sqr()).sqrt()).ln()
    }

    /// 反余弦 `acos z = π/2 − asin z`，主值。
    #[inline]
    #[must_use]
    pub fn acos(self) -> Complex {
        Complex::new(std::f64::consts::FRAC_PI_2, 0.0).sub_complex(self.asin())
    }

    /// 反正切 `atan z = (i/2)·[ln(1−iz) − ln(1+iz)]`，主值。
    #[inline]
    #[must_use]
    pub fn atan(self) -> Complex {
        ((Complex::ONE - Complex::I * self).ln() - (Complex::ONE + Complex::I * self).ln())
            .mul_complex(Complex::I)
            .mul_f64(0.5)
    }

    /// 反双曲正弦 `asinh z = ln(z + √(z²+1))`，主值。
    #[inline]
    #[must_use]
    pub fn asinh(self) -> Complex {
        (self + (self.sqr() + Complex::ONE).sqrt()).ln()
    }

    /// 反双曲余弦 `acosh z = ln(z + √(z+1)·√(z−1))`，主值。
    #[inline]
    #[must_use]
    pub fn acosh(self) -> Complex {
        (self + (self + Complex::ONE).sqrt() * (self - Complex::ONE).sqrt()).ln()
    }

    /// 反双曲正切 `atanh z = ½·ln((1+z)/(1−z))`，主值。
    #[inline]
    #[must_use]
    pub fn atanh(self) -> Complex {
        ((Complex::ONE + self).div_complex(Complex::ONE - self))
            .ln()
            .mul_f64(0.5)
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// `self == 0`
    #[inline]
    pub fn is_zero(self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }

    /// 虚部为零（即位于实轴上）。
    #[inline]
    pub fn is_real(self) -> bool {
        self.im == 0.0
    }

    /// 实部为零且虚部非零（即位于虚轴上）。
    #[inline]
    pub fn is_pure_imaginary(self) -> bool {
        self.re == 0.0 && self.im != 0.0
    }

    /// 实部与虚部均有限。
    #[inline]
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// 任一分量为 `NaN`。
    #[inline]
    pub fn is_nan(self) -> bool {
        self.re.is_nan() || self.im.is_nan()
    }

    /// 任一分量为无穷。
    #[inline]
    pub fn is_infinite(self) -> bool {
        self.re.is_infinite() || self.im.is_infinite()
    }

    /// 近似相等：`|self − other| ≤ eps·(1 + |self| + |other|)`
    /// （混合绝对/相对误差，适用于跨数量级的比较）。
    #[inline]
    pub fn approx_eq(self, other: Complex, eps: f64) -> bool {
        (self - other).abs() <= eps * (1.0 + self.abs() + other.abs())
    }
}

// ---------------------------------------------------------------------------
// 与 f64 的转换
// ---------------------------------------------------------------------------

impl From<f64> for Complex {
    #[inline]
    fn from(x: f64) -> Complex {
        Complex::new(x, 0.0)
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im == 0.0 {
            write!(f, "{}", self.re)
        } else if self.re == 0.0 {
            write!(f, "{}i", self.im)
        } else if self.im.is_sign_negative() {
            write!(f, "{} - {}i", self.re, -self.im)
        } else {
            write!(f, "{} + {}i", self.re, self.im)
        }
    }
}

impl fmt::Debug for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Complex({self})")
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：复数 ⊛ 复数
// ---------------------------------------------------------------------------

impl Add for Complex {
    type Output = Complex;
    #[inline]
    fn add(self, rhs: Complex) -> Complex {
        self.add_complex(rhs)
    }
}

impl Sub for Complex {
    type Output = Complex;
    #[inline]
    fn sub(self, rhs: Complex) -> Complex {
        self.sub_complex(rhs)
    }
}

impl Mul for Complex {
    type Output = Complex;
    #[inline]
    fn mul(self, rhs: Complex) -> Complex {
        self.mul_complex(rhs)
    }
}

impl Div for Complex {
    type Output = Complex;
    #[inline]
    fn div(self, rhs: Complex) -> Complex {
        self.div_complex(rhs)
    }
}

impl Neg for Complex {
    type Output = Complex;
    #[inline]
    fn neg(self) -> Complex {
        Complex::new(-self.re, -self.im)
    }
}

impl AddAssign for Complex {
    #[inline]
    fn add_assign(&mut self, rhs: Complex) {
        *self = self.add_complex(rhs);
    }
}

impl SubAssign for Complex {
    #[inline]
    fn sub_assign(&mut self, rhs: Complex) {
        *self = self.sub_complex(rhs);
    }
}

impl MulAssign for Complex {
    #[inline]
    fn mul_assign(&mut self, rhs: Complex) {
        *self = self.mul_complex(rhs);
    }
}

impl DivAssign for Complex {
    #[inline]
    fn div_assign(&mut self, rhs: Complex) {
        *self = self.div_complex(rhs);
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：与 f64 混合
// ---------------------------------------------------------------------------

macro_rules! impl_scalar_binop {
    ($trait:ident, $method:ident, $fwd:ident, $rev:ident) => {
        impl $trait<f64> for Complex {
            type Output = Complex;
            #[inline]
            fn $method(self, rhs: f64) -> Complex {
                self.$fwd(rhs)
            }
        }
        impl $trait<Complex> for f64 {
            type Output = Complex;
            #[inline]
            fn $method(self, rhs: Complex) -> Complex {
                rhs.$rev(self)
            }
        }
    };
}

impl_scalar_binop!(Add, add, add_f64, add_f64);
impl_scalar_binop!(Sub, sub, sub_f64, f64_sub);
impl_scalar_binop!(Mul, mul, mul_f64, mul_f64);
impl_scalar_binop!(Div, div, div_f64, f64_div);

macro_rules! impl_scalar_assign {
    ($trait:ident, $method:ident, $fwd:ident) => {
        impl $trait<f64> for Complex {
            #[inline]
            fn $method(&mut self, rhs: f64) {
                *self = self.$fwd(rhs);
            }
        }
    };
}

impl_scalar_assign!(AddAssign, add_assign, add_f64);
impl_scalar_assign!(SubAssign, sub_assign, sub_f64);
impl_scalar_assign!(MulAssign, mul_assign, mul_f64);
impl_scalar_assign!(DivAssign, div_assign, div_f64);

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Complex;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    const EPS: f64 = 1e-12;

    fn assert_approx(a: Complex, b: Complex) {
        let diff = (a - b).abs();
        assert!(diff <= EPS, "近似断言失败：{a} ≈ {b}，误差 {diff}");
    }

    // ---- 基本算术（复数） ----

    #[test]
    fn basic_add_sub() {
        let z = Complex::new(3.0, 4.0);
        let w = Complex::new(1.0, -2.0);
        assert_eq!(z.add_complex(w), Complex::new(4.0, 2.0));
        assert_eq!(z.sub_complex(w), Complex::new(2.0, 6.0));
        assert_eq!(z + w, Complex::new(4.0, 2.0));
        assert_eq!(z - w, Complex::new(2.0, 6.0));
        assert_eq!(-z, Complex::new(-3.0, -4.0));
    }

    #[test]
    fn basic_mul_div() {
        let z = Complex::new(3.0, 4.0);
        let w = Complex::new(1.0, -2.0);
        // (3+4i)(1−2i) = 11 − 2i
        assert_eq!(z.mul_complex(w), Complex::new(11.0, -2.0));
        assert_eq!(z * w, Complex::new(11.0, -2.0));
        // (8+9i)/(1+2i) = 5.2 − 1.4i
        let q = Complex::new(8.0, 9.0).div_complex(Complex::new(1.0, 2.0));
        assert_approx(q, Complex::new(5.2, -1.4));
        assert_approx(
            Complex::new(8.0, 9.0) / Complex::new(1.0, 2.0),
            Complex::new(5.2, -1.4),
        );
        // 除以零 → NaN
        assert!(Complex::ONE.div_complex(Complex::ZERO).is_nan());
    }

    // ---- 与 f64 的混合运算 ----

    #[test]
    fn mixed_f64() {
        let z = Complex::new(2.0, 3.0);
        assert_eq!(z.add_f64(4.0), Complex::new(6.0, 3.0));
        assert_eq!(z + 4.0, Complex::new(6.0, 3.0));
        assert_eq!(4.0 + z, Complex::new(6.0, 3.0));
        assert_eq!(z.sub_f64(4.0), Complex::new(-2.0, 3.0));
        assert_eq!(z - 4.0, Complex::new(-2.0, 3.0));
        assert_eq!(4.0 - z, Complex::new(2.0, -3.0));
        assert_eq!(z.f64_sub(4.0), Complex::new(2.0, -3.0));
        assert_eq!(z.mul_f64(2.0), Complex::new(4.0, 6.0));
        assert_eq!(z * 2.0, Complex::new(4.0, 6.0));
        assert_eq!(2.0 * z, Complex::new(4.0, 6.0));
        assert_eq!(z.div_f64(2.0), Complex::new(1.0, 1.5));
        assert_eq!(z / 2.0, Complex::new(1.0, 1.5));
        // 2/(1+i) = 1 − i
        assert_approx(2.0 / Complex::new(1.0, 1.0), Complex::new(1.0, -1.0));
        assert_approx(Complex::new(1.0, 1.0).f64_div(2.0), Complex::new(1.0, -1.0));
    }

    #[test]
    fn assign_ops() {
        let mut z = Complex::new(1.0, 2.0);
        z += Complex::new(2.0, 3.0);
        assert_eq!(z, Complex::new(3.0, 5.0));
        z -= 1.0;
        assert_eq!(z, Complex::new(2.0, 5.0));
        z *= 2.0;
        assert_eq!(z, Complex::new(4.0, 10.0));
        z /= 2.0;
        assert_eq!(z, Complex::new(2.0, 5.0));
    }

    // ---- 除法稳健性 ----

    #[test]
    fn div_robustness() {
        // 大数相除不应中间溢出
        let z = Complex::new(1e200, 2e200);
        let w = Complex::new(1e200, 0.0);
        assert_approx(z / w, Complex::new(1.0, 2.0));
        // 小数相除不应中间下溢：(1+2i)/(i)·1e-200/1e-200 = 2 − i
        let z = Complex::new(1e-200, 2e-200);
        let w = Complex::new(0.0, 1e-200);
        assert_approx(z / w, Complex::new(2.0, -1.0));
        // 倒数跨数量级
        assert_approx(Complex::new(1.0, 1.0).recip(), Complex::new(0.5, -0.5));
        assert_approx(Complex::new(1e200, 0.0).recip(), Complex::new(1e-200, 0.0));
        assert_approx(Complex::new(1e-200, 0.0).recip(), Complex::new(1e200, 0.0));
        // (z / w) * w ≈ z
        let z = Complex::new(1.234, -5.678);
        let w = Complex::new(1e-150, 2e150);
        assert_approx((z / w) * w, z);
    }

    // ---- 指数 / 对数 ----

    #[test]
    fn exp_ln() {
        assert_eq!(Complex::ZERO.exp(), Complex::ONE);
        // e^{iθ} = cos θ + i sin θ
        let t = 0.7;
        assert_approx((Complex::I * t).exp(), Complex::new(t.cos(), t.sin()));
        // e^{iπ} + 1 = 0
        assert_approx((Complex::I * PI).exp() + 1.0, Complex::ZERO);
        // exp(z + w) = exp z · exp w
        let z = Complex::new(0.3, -0.2);
        let w = Complex::new(-0.5, 0.9);
        assert_approx((z + w).exp(), z.exp() * w.exp());
        // ln(e^z) = z（主值）
        let z = Complex::new(0.5, 1.2);
        assert_approx(z.exp().ln(), z);
        // ln(1+i) = ½·ln 2 + i·π/4
        assert_approx(
            Complex::new(1.0, 1.0).ln(),
            Complex::new(0.5 * 2f64.ln(), FRAC_PI_4),
        );
        // ln(−1) = iπ
        assert_approx(Complex::new(-1.0, 0.0).ln(), Complex::new(0.0, PI));
    }

    // ---- 幂 / 平方根 ----

    #[test]
    fn pow_sqrt() {
        let z = Complex::new(1.0, 1.0);
        // (1+i)² = 2i
        assert_approx(
            z.pow_complex(Complex::new(2.0, 0.0)),
            Complex::new(0.0, 2.0),
        );
        assert_approx(z.pow_f64(2.0), Complex::new(0.0, 2.0));
        // i^i = e^{−π/2}
        assert_approx(
            Complex::I.pow_complex(Complex::I),
            Complex::new((-FRAC_PI_2).exp(), 0.0),
        );
        // 主值分支：(−8)^{1/3} = 1 + i√3
        assert_approx(
            Complex::new(-8.0, 0.0).pow_f64(1.0 / 3.0),
            Complex::new(1.0, 3f64.sqrt()),
        );
        // 平方根
        assert_approx(Complex::new(-1.0, 0.0).sqrt(), Complex::I);
        assert_approx(Complex::new(4.0, 0.0).sqrt(), Complex::new(2.0, 0.0));
        assert_approx(Complex::new(0.0, 2.0).sqrt(), Complex::new(1.0, 1.0));
        // (√z)² = z
        let z = Complex::new(-2.0, 1.5);
        assert_approx(z.sqrt().sqr(), z);
    }

    // ---- 三角 / 反三角 ----

    #[test]
    fn trig() {
        let z = Complex::new(0.3, 0.4);
        // sin² + cos² = 1
        assert_approx(z.sin().sqr() + z.cos().sqr(), Complex::ONE);
        // sin(iz) = i·sinh z
        assert_approx((Complex::I * z).sin(), Complex::I * z.sinh());
        // sin z = (e^{iz} − e^{−iz}) / 2i
        let iz = Complex::I * z;
        assert_approx(z.sin(), (iz.exp() - (-iz).exp()) / Complex::new(0.0, 2.0));
        // 反函数往返（各自主值分支内）
        assert_approx(z.sin().asin(), z);
        assert_approx(z.cos().acos(), z);
        assert_approx(z.tan().atan(), z);
        assert_approx(z.sinh().asinh(), z);
        assert_approx(z.tanh().atanh(), z);
        let z2 = Complex::new(1.2, 0.5);
        assert_approx(z2.cosh().acosh(), z2);
    }

    // ---- 共轭 / 模 / 辐角 ----

    #[test]
    fn conj_norm() {
        let z = Complex::new(3.0, 4.0);
        assert_eq!(z.conj(), Complex::new(3.0, -4.0));
        assert_eq!(z + z.conj(), Complex::new(6.0, 0.0));
        assert_eq!(z - z.conj(), Complex::new(0.0, 8.0));
        assert_eq!(z * z.conj(), Complex::new(25.0, 0.0));
        assert_eq!(z.norm_sqr(), 25.0);
        assert_eq!(z.abs(), 5.0);
        assert_eq!(z.re(), 3.0);
        assert_eq!(z.im(), 4.0);
        assert_eq!(z.arg(), (4f64 / 3.0).atan());
        assert_eq!(Complex::I.arg(), FRAC_PI_2);
        assert_eq!(Complex::new(0.0, -1.0).arg(), -FRAC_PI_2);
    }

    // ---- 极坐标 / 旋转 ----

    #[test]
    fn polar_rotate() {
        assert_eq!(Complex::from_polar(2.0, 0.0), Complex::new(2.0, 0.0));
        assert_approx(Complex::from_polar(1.0, FRAC_PI_2), Complex::I);
        let z = Complex::new(1.0, 1.0);
        let (r, theta) = z.to_polar();
        assert_approx(z, Complex::from_polar(r, theta));
        assert!((r - 2f64.sqrt()).abs() < EPS);
        assert!((theta - FRAC_PI_4).abs() < EPS);
        // 旋转
        assert_approx(Complex::ONE.rotate(FRAC_PI_2), Complex::I);
        assert_approx(Complex::I.rotate(FRAC_PI_2), Complex::new(-1.0, 0.0));
    }

    // ---- 谓词 / 近似比较 / 转换 ----

    #[test]
    fn predicates_approx() {
        let z = Complex::new(1.0, 0.0);
        assert!(z.is_real());
        assert!(!z.is_pure_imaginary());
        assert!(Complex::I.is_pure_imaginary());
        assert!(Complex::ZERO.is_zero());
        assert!(z.is_finite());
        assert!(Complex::new(f64::NAN, 0.0).is_nan());
        assert!(Complex::new(f64::INFINITY, 0.0).is_infinite());
        assert!(Complex::new(1.0, 1e-14).approx_eq(Complex::ONE, 1e-12));
        assert!(!Complex::new(1.0, 1e-3).approx_eq(Complex::ONE, 1e-12));
        assert_eq!(Complex::from(2.5), Complex::new(2.5, 0.0));
    }

    // ---- 字符串输出 ----

    #[test]
    fn fmt_display() {
        assert_eq!(Complex::new(3.0, 4.0).to_string(), "3 + 4i");
        assert_eq!(Complex::new(3.0, -4.0).to_string(), "3 - 4i");
        assert_eq!(Complex::new(-3.0, 0.0).to_string(), "-3");
        assert_eq!(Complex::new(0.0, 5.0).to_string(), "5i");
        assert_eq!(Complex::new(0.0, -5.0).to_string(), "-5i");
        assert_eq!(Complex::ZERO.to_string(), "0");
        assert_eq!(format!("{:?}", Complex::new(1.0, 2.0)), "Complex(1 + 2i)");
    }
}
