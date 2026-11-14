//! 3×3 复矩阵 `Mat3C` —— 齐次坐标下的几何线性代数内核。
//!
//! # 约定
//!
//! - **存储**：行优先，`m[row][col]`，字段公开可直接访问；
//! - **向量约定**：列向量。`M·v` 用 [`Mat3C::mul_vec`]，几何变换写作 `p' = M·p`；
//!   行向量 `v·M` 等价于 `Mᵀ·v`，用 [`Mat3C::transpose_mul_vec`]；
//! - **与几何解耦**：本模块只做纯线性代数，向量一律以 `[Complex; 3]` 表示，
//!   点 / 直线 / 圆锥曲线等几何对象之后以 Newtype 封装；
//! - **对称矩阵与行列约定无关**：圆锥曲线矩阵 `C`（`pᵀ C p = 0`）等自伴情形
//!   不受上述列向量约定的影响。
//!
//! # 稳健性
//!
//! - `try_inv` / `solve` 以行列式为零、NaN 或无穷判定不可逆，返回 `Option`；
//! - [`Mat3C::adjugate`] 提供"差一个标量因子的逆"，是射影几何中处理
//!   退化 / 尺度无关量的关键（如退化圆锥曲线的对偶）；
//! - Frobenius 范数用 `hypot` 累积，避免平方和溢出。
//!
//! 编译器会对 3×3 的小循环自动向量化，无需手工 SIMD。

use std::array;
use std::fmt;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

use super::Complex;

/// 3×3 复矩阵，行优先存储（`m[row][col]`）。
///
/// `Copy` 类型；`Default` 为零矩阵（加法幺元），乘法幺元见 [`Mat3C::IDENTITY`]。
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Mat3C {
    /// 行优先存储的 9 个元素：`m[行][列]`
    pub m: [[Complex; 3]; 3],
}

impl Mat3C {
    /// 零矩阵（加法幺元）
    pub const ZERO: Mat3C = Mat3C::new([[Complex::ZERO; 3]; 3]);
    /// 单位矩阵（乘法幺元）
    pub const IDENTITY: Mat3C = Mat3C::new([
        [Complex::ONE, Complex::ZERO, Complex::ZERO],
        [Complex::ZERO, Complex::ONE, Complex::ZERO],
        [Complex::ZERO, Complex::ZERO, Complex::ONE],
    ]);

    // ------------------------------------------------------------------
    // 构造
    // ------------------------------------------------------------------

    /// 按行构造：`new([[a, b, c], [d, e, f], [g, h, i]])`。
    #[inline]
    pub const fn new(m: [[Complex; 3]; 3]) -> Mat3C {
        Mat3C { m }
    }

    /// 按列构造：`from_cols(c0, c1, c2)`，每列为一个 `[Complex; 3]`。
    #[inline]
    pub const fn from_cols(c0: [Complex; 3], c1: [Complex; 3], c2: [Complex; 3]) -> Mat3C {
        Mat3C {
            m: [
                [c0[0], c1[0], c2[0]],
                [c0[1], c1[1], c2[1]],
                [c0[2], c1[2], c2[2]],
            ],
        }
    }

    /// 对角矩阵 `diag(d0, d1, d2)`。
    #[inline]
    pub fn from_diag(d: [Complex; 3]) -> Mat3C {
        Mat3C::new([
            [d[0], Complex::ZERO, Complex::ZERO],
            [Complex::ZERO, d[1], Complex::ZERO],
            [Complex::ZERO, Complex::ZERO, d[2]],
        ])
    }

    /// 标量矩阵 `s·I`。
    #[inline]
    pub fn from_scalar(s: Complex) -> Mat3C {
        Mat3C::from_diag([s; 3])
    }

    /// 向量 `v` 的叉积矩阵（反对称）：`from_cross(v)·w = v × w`。
    ///
    /// 射影几何中用于"两点定直线"：`l = from_cross(p)·q = p × q`。
    #[inline]
    pub fn from_cross(v: [Complex; 3]) -> Mat3C {
        Mat3C::new([
            [Complex::ZERO, -v[2], v[1]],
            [v[2], Complex::ZERO, -v[0]],
            [-v[1], v[0], Complex::ZERO],
        ])
    }

    /// 外积 `u·vᵀ`（列向量约定）：`from_outer(u, v)·w = u·(v·w)`。
    #[inline]
    pub fn from_outer(u: [Complex; 3], v: [Complex; 3]) -> Mat3C {
        Mat3C {
            m: array::from_fn(|r| array::from_fn(|c| u[r] * v[c])),
        }
    }

    // ------------------------------------------------------------------
    // 访问
    // ------------------------------------------------------------------

    /// 读取 `(row, col)` 元素。索引须在 `0..3`，越界会 panic。
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Complex {
        self.m[row][col]
    }

    /// 写入 `(row, col)` 元素。索引须在 `0..3`，越界会 panic。
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: Complex) {
        self.m[row][col] = value;
    }

    /// 取第 `i` 行（`0..3`）。
    #[inline]
    pub fn row(&self, i: usize) -> [Complex; 3] {
        self.m[i]
    }

    /// 取第 `i` 列（`0..3`）。
    #[inline]
    pub fn col(&self, i: usize) -> [Complex; 3] {
        [self.m[0][i], self.m[1][i], self.m[2][i]]
    }

    /// 主对角线。
    #[inline]
    pub fn diag(&self) -> [Complex; 3] {
        [self.m[0][0], self.m[1][1], self.m[2][2]]
    }

    /// 逐元素映射，返回新矩阵。
    #[inline]
    pub fn map(&self, mut f: impl FnMut(Complex) -> Complex) -> Mat3C {
        Mat3C {
            m: array::from_fn(|r| array::from_fn(|c| f(self.m[r][c]))),
        }
    }

    // ------------------------------------------------------------------
    // 基本算术（矩阵）
    // ------------------------------------------------------------------

    /// `self + other`
    #[inline]
    #[must_use]
    pub fn add(&self, other: &Mat3C) -> Mat3C {
        Mat3C {
            m: array::from_fn(|r| array::from_fn(|c| self.m[r][c] + other.m[r][c])),
        }
    }

    /// `self − other`
    #[inline]
    #[must_use]
    pub fn sub(&self, other: &Mat3C) -> Mat3C {
        Mat3C {
            m: array::from_fn(|r| array::from_fn(|c| self.m[r][c] - other.m[r][c])),
        }
    }

    /// `self * other`（矩阵乘法）
    #[inline]
    #[must_use]
    pub fn mul(&self, other: &Mat3C) -> Mat3C {
        let mut out = Mat3C::ZERO;
        for r in 0..3 {
            for c in 0..3 {
                let mut acc = Complex::ZERO;
                for k in 0..3 {
                    acc += self.m[r][k] * other.m[k][c];
                }
                out.m[r][c] = acc;
            }
        }
        out
    }

    /// `−self`
    #[inline]
    #[must_use]
    pub fn neg(&self) -> Mat3C {
        self.map(|x| -x)
    }

    // ------------------------------------------------------------------
    // 基本算术（标量）
    // ------------------------------------------------------------------

    /// `self * s`（复标量）
    #[inline]
    #[must_use]
    pub fn mul_complex(&self, s: Complex) -> Mat3C {
        self.map(|x| x * s)
    }

    /// `self / s`（复标量）
    #[inline]
    #[must_use]
    pub fn div_complex(&self, s: Complex) -> Mat3C {
        self.map(|x| x / s)
    }

    /// `self * s`（实标量）
    #[inline]
    #[must_use]
    pub fn mul_f64(&self, s: f64) -> Mat3C {
        self.map(|x| x.mul_f64(s))
    }

    /// `self / s`（实标量）
    #[inline]
    #[must_use]
    pub fn div_f64(&self, s: f64) -> Mat3C {
        self.map(|x| x.div_f64(s))
    }

    // ------------------------------------------------------------------
    // 矩阵 × 向量
    // ------------------------------------------------------------------

    /// `self·v`（列向量约定，`v: [Complex; 3]`）。
    #[inline]
    #[must_use]
    pub fn mul_vec(&self, v: [Complex; 3]) -> [Complex; 3] {
        let m = &self.m;
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    }

    /// `selfᵀ·v`，等价于行向量运算 `v·self`。
    #[inline]
    #[must_use]
    pub fn transpose_mul_vec(&self, v: [Complex; 3]) -> [Complex; 3] {
        let m = &self.m;
        [
            m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
            m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
            m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
        ]
    }

    // ------------------------------------------------------------------
    // 转置 / 共轭
    // ------------------------------------------------------------------

    /// 转置 `selfᵀ`。
    #[inline]
    #[must_use]
    pub fn transpose(&self) -> Mat3C {
        let m = &self.m;
        Mat3C::new([
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ])
    }

    /// 逐元素共轭。
    #[inline]
    #[must_use]
    pub fn conj(&self) -> Mat3C {
        self.map(Complex::conj)
    }

    /// 共轭转置（Hermite 伴随）`self† = conj(self)ᵀ`。
    #[inline]
    #[must_use]
    pub fn adjoint(&self) -> Mat3C {
        self.conj().transpose()
    }

    // ------------------------------------------------------------------
    // 行列式 / 迹 / 逆
    // ------------------------------------------------------------------

    /// 行列式（沿第 0 行展开的 Sarrus 公式）。
    #[inline]
    #[must_use]
    pub fn det(&self) -> Complex {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// 迹 `Σ m[i][i]`。
    #[inline]
    #[must_use]
    pub fn trace(&self) -> Complex {
        self.m[0][0] + self.m[1][1] + self.m[2][2]
    }

    /// 余子式：删去 `(row, col)` 后的 2×2 行列式。
    #[inline]
    #[must_use]
    pub fn minor(&self, row: usize, col: usize) -> Complex {
        let mut sub = [Complex::ZERO; 4];
        let mut idx = 0;
        for r in 0..3 {
            if r == row {
                continue;
            }
            for c in 0..3 {
                if c == col {
                    continue;
                }
                sub[idx] = self.m[r][c];
                idx += 1;
            }
        }
        sub[0] * sub[3] - sub[1] * sub[2]
    }

    /// 代数余子式 `(−1)^(row+col)·minor(row, col)`。
    #[inline]
    #[must_use]
    pub fn cofactor(&self, row: usize, col: usize) -> Complex {
        if (row + col).is_multiple_of(2) {
            self.minor(row, col)
        } else {
            -self.minor(row, col)
        }
    }

    /// 伴随矩阵 `adj(self)`：满足 `self·adj(self) = det(self)·I`。
    ///
    /// 射影几何中即"差一个标量因子的逆"，对奇异矩阵仍有意义。
    #[inline]
    #[must_use]
    pub fn adjugate(&self) -> Mat3C {
        let m = &self.m;
        let (a, b, c) = (m[0][0], m[0][1], m[0][2]);
        let (d, e, f) = (m[1][0], m[1][1], m[1][2]);
        let (g, h, i) = (m[2][0], m[2][1], m[2][2]);
        Mat3C::new([
            [e * i - f * h, c * h - b * i, b * f - c * e],
            [f * g - d * i, a * i - c * g, c * d - a * f],
            [d * h - e * g, b * g - a * h, a * e - b * d],
        ])
    }

    /// 逆矩阵 `self⁻¹ = adj(self)/det(self)`。
    ///
    /// 行列式为零、NaN 或无穷时返回 `None`。
    #[inline]
    #[must_use]
    pub fn try_inv(&self) -> Option<Mat3C> {
        let det = self.det();
        if det == Complex::ZERO || !det.is_finite() {
            return None;
        }
        Some(self.adjugate().div_complex(det))
    }

    /// 解线性方程组 `self·x = b`（Cramer 法则：`x = adj(self)·b / det(self)`）。
    ///
    /// 不可逆时返回 `None`。
    #[inline]
    #[must_use]
    pub fn solve(&self, b: [Complex; 3]) -> Option<[Complex; 3]> {
        let det = self.det();
        if det == Complex::ZERO || !det.is_finite() {
            return None;
        }
        let x = self.adjugate().mul_vec(b);
        Some([x[0] / det, x[1] / det, x[2] / det])
    }

    // ------------------------------------------------------------------
    // 范数
    // ------------------------------------------------------------------

    /// Frobenius 范数平方 `Σ|mᵢⱼ|²`。元素极大时可能溢出为 `inf`，
    /// 此时请改用 [`Mat3C::frobenius_norm`]。
    #[inline]
    #[must_use]
    pub fn frobenius_norm_sqr(&self) -> f64 {
        self.m.iter().flatten().map(|x| x.norm_sqr()).sum()
    }

    /// Frobenius 范数 `√(Σ|mᵢⱼ|²)`。用 `hypot` 累积，无中间上溢/下溢。
    #[inline]
    #[must_use]
    pub fn frobenius_norm(&self) -> f64 {
        let mut n = 0.0f64;
        for row in &self.m {
            for x in row {
                n = n.hypot(x.abs());
            }
        }
        n
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 是否为零矩阵。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.m.iter().flatten().all(|x| *x == Complex::ZERO)
    }

    /// 是否（精确地）为单位矩阵。
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Mat3C::IDENTITY
    }

    /// 是否只有主对角线非零。
    #[inline]
    pub fn is_diagonal(&self) -> bool {
        for r in 0..3 {
            for c in 0..3 {
                if r != c && self.m[r][c] != Complex::ZERO {
                    return false;
                }
            }
        }
        true
    }

    /// 是否对称（`self == selfᵀ`）。
    #[inline]
    pub fn is_symmetric(&self) -> bool {
        for r in 0..3 {
            for c in 0..3 {
                if self.m[r][c] != self.m[c][r] {
                    return false;
                }
            }
        }
        true
    }

    /// 是否 Hermite（`self == self†`）。
    #[inline]
    pub fn is_hermitian(&self) -> bool {
        for r in 0..3 {
            for c in 0..3 {
                if self.m[r][c] != self.m[c][r].conj() {
                    return false;
                }
            }
        }
        true
    }

    /// 是否可逆（行列式非零且有限）。
    #[inline]
    pub fn is_invertible(&self) -> bool {
        let det = self.det();
        det != Complex::ZERO && det.is_finite()
    }

    /// 逐元素近似相等：`|self − other| ≤ eps·(1 + |self| + |other|)`（Frobenius 范数）。
    #[inline]
    pub fn approx_eq(&self, other: &Mat3C, eps: f64) -> bool {
        (0..3).all(|r| (0..3).all(|c| self.m[r][c].approx_eq(other.m[r][c], eps)))
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Mat3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Mat3C-----\n[{}, {}, {}]", self.m[0][0], self.m[0][1], self.m[0][2])?;
        writeln!(f, "[{}, {}, {}]", self.m[1][0], self.m[1][1], self.m[1][2])?;
        write!(f, "[{}, {}, {}]\n----------", self.m[2][0], self.m[2][1], self.m[2][2])
    }
}

impl fmt::Debug for Mat3C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mat3C({self})")
    }
}

// ---------------------------------------------------------------------------
// 索引 `m[(row, col)]`
// ---------------------------------------------------------------------------

impl Index<(usize, usize)> for Mat3C {
    type Output = Complex;
    #[inline]
    fn index(&self, (row, col): (usize, usize)) -> &Complex {
        &self.m[row][col]
    }
}

impl IndexMut<(usize, usize)> for Mat3C {
    #[inline]
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Complex {
        &mut self.m[row][col]
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：矩阵 ⊛ 矩阵
// ---------------------------------------------------------------------------

impl Add for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn add(self, rhs: Mat3C) -> Mat3C {
        Mat3C::add(&self, &rhs)
    }
}

impl Sub for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn sub(self, rhs: Mat3C) -> Mat3C {
        Mat3C::sub(&self, &rhs)
    }
}

impl Mul for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: Mat3C) -> Mat3C {
        Mat3C::mul(&self, &rhs)
    }
}

impl Neg for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn neg(self) -> Mat3C {
        Mat3C::neg(&self)
    }
}

impl AddAssign for Mat3C {
    #[inline]
    fn add_assign(&mut self, rhs: Mat3C) {
        *self = Mat3C::add(self, &rhs);
    }
}

impl SubAssign for Mat3C {
    #[inline]
    fn sub_assign(&mut self, rhs: Mat3C) {
        *self = Mat3C::sub(self, &rhs);
    }
}

impl MulAssign for Mat3C {
    #[inline]
    fn mul_assign(&mut self, rhs: Mat3C) {
        *self = Mat3C::mul(self, &rhs);
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：矩阵 × 向量
// ---------------------------------------------------------------------------

impl Mul<[Complex; 3]> for Mat3C {
    type Output = [Complex; 3];
    #[inline]
    fn mul(self, rhs: [Complex; 3]) -> [Complex; 3] {
        self.mul_vec(rhs)
    }
}

// ---------------------------------------------------------------------------
// 运算符重载：与标量混合
// ---------------------------------------------------------------------------

impl Mul<f64> for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: f64) -> Mat3C {
        self.mul_f64(rhs)
    }
}

impl Mul<Mat3C> for f64 {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: Mat3C) -> Mat3C {
        rhs.mul_f64(self)
    }
}

impl Div<f64> for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn div(self, rhs: f64) -> Mat3C {
        self.div_f64(rhs)
    }
}

impl Mul<Complex> for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: Complex) -> Mat3C {
        self.mul_complex(rhs)
    }
}

impl Mul<Mat3C> for Complex {
    type Output = Mat3C;
    #[inline]
    fn mul(self, rhs: Mat3C) -> Mat3C {
        rhs.mul_complex(self)
    }
}

impl Div<Complex> for Mat3C {
    type Output = Mat3C;
    #[inline]
    fn div(self, rhs: Complex) -> Mat3C {
        self.div_complex(rhs)
    }
}

impl MulAssign<f64> for Mat3C {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        *self = self.mul_f64(rhs);
    }
}

impl MulAssign<Complex> for Mat3C {
    #[inline]
    fn mul_assign(&mut self, rhs: Complex) {
        *self = self.mul_complex(rhs);
    }
}

impl DivAssign<f64> for Mat3C {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        *self = self.div_f64(rhs);
    }
}

impl DivAssign<Complex> for Mat3C {
    #[inline]
    fn div_assign(&mut self, rhs: Complex) {
        *self = self.div_complex(rhs);
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Mat3C;
    use crate::pakoo_forest::calc::Complex;

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    /// A = [[1,2,3],[4,5,6],[7,8,10]]，det = −3
    fn mat_a() -> Mat3C {
        Mat3C::new([
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
            [c(7.0, 0.0), c(8.0, 0.0), c(10.0, 0.0)],
        ])
    }

    /// M = [[i,1,0],[0,i,1],[1,0,i]]，det = 1 − i
    fn mat_m() -> Mat3C {
        Mat3C::new([
            [c(0.0, 1.0), c(1.0, 0.0), c(0.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 1.0), c(1.0, 0.0)],
            [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 1.0)],
        ])
    }

    /// 幂零矩阵 N（N³ = 0）
    fn mat_n() -> Mat3C {
        Mat3C::new([
            [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        ])
    }

    fn assert_approx(a: &Mat3C, b: &Mat3C) {
        assert!(
            a.approx_eq(b, EPS),
            "矩阵近似断言失败：\n{a}\n≈\n{b}\n误差：\n{}",
            a.sub(b)
        );
    }

    fn assert_vec_approx(a: [Complex; 3], b: [Complex; 3]) {
        for i in 0..3 {
            assert!(
                a[i].approx_eq(b[i], EPS),
                "向量分量 {i} 近似断言失败：{} ≈ {}",
                a[i],
                b[i]
            );
        }
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let m = mat_a();
        assert_eq!(m.get(1, 2), c(6.0, 0.0));
        assert_eq!(m[(1, 2)], c(6.0, 0.0));
        assert_eq!(m.row(0), [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]);
        assert_eq!(m.col(1), [c(2.0, 0.0), c(5.0, 0.0), c(8.0, 0.0)]);
        assert_eq!(m.diag(), [c(1.0, 0.0), c(5.0, 0.0), c(10.0, 0.0)]);

        let mut n = m;
        n.set(0, 0, c(9.0, 9.0));
        assert_eq!(n.get(0, 0), c(9.0, 9.0));
        n[(2, 1)] = c(8.0, 0.0);
        assert_eq!(n.get(2, 1), c(8.0, 0.0));

        let cols = Mat3C::from_cols(
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
            [c(7.0, 0.0), c(8.0, 0.0), c(9.0, 0.0)],
        );
        assert_eq!(cols.m[0], [c(1.0, 0.0), c(4.0, 0.0), c(7.0, 0.0)]);
        assert_eq!(cols.m[1][0], c(2.0, 0.0));

        let d = Mat3C::from_diag([c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]);
        assert!(d.is_diagonal());
        assert_eq!(
            Mat3C::from_scalar(c(5.0, 0.0)),
            Mat3C::IDENTITY.mul_f64(5.0)
        );
    }

    // ---- 零矩阵 / 单位矩阵 ----

    #[test]
    fn zero_identity() {
        assert!(Mat3C::ZERO.is_zero());
        assert!(Mat3C::IDENTITY.is_identity());
        assert_eq!(Mat3C::ZERO.mul(&mat_a()), Mat3C::ZERO);
        assert_eq!(Mat3C::IDENTITY.mul(&mat_a()), mat_a());
        assert_eq!(mat_a().mul(&Mat3C::IDENTITY), mat_a());
        assert_eq!(mat_a().add(&Mat3C::ZERO), mat_a());
        let v = [c(2.0, 1.0), c(-3.0, 0.0), c(0.0, 4.0)];
        assert_eq!(Mat3C::IDENTITY.mul_vec(v), v);
    }

    // ---- 加减 / 取负 ----

    #[test]
    fn add_sub_neg() {
        let a = mat_a();
        let m = mat_m();
        assert_eq!(a.add(&a).sub(&a), a); // (A+A)−A = A
        assert_eq!(a.sub(&a), Mat3C::ZERO);
        assert_eq!(a.add(&a.neg()), Mat3C::ZERO);
        assert_eq!(a.add(&m), m.add(&a)); // 加法交换律
        let s = a.add(&m);
        assert_eq!(s.sub(&m), a);
        assert_eq!(-a, a.mul_f64(-1.0));
        assert_eq!(a + m, a.add(&m)); // 运算符与命名方法一致
        assert_eq!(a - m, a.sub(&m));
    }

    // ---- 矩阵乘法：已知值 ----

    #[test]
    fn mul_known_values() {
        // A²（整数矩阵，精确）
        assert_eq!(
            mat_a().mul(&mat_a()),
            Mat3C::new([
                [c(30.0, 0.0), c(36.0, 0.0), c(45.0, 0.0)],
                [c(66.0, 0.0), c(81.0, 0.0), c(102.0, 0.0)],
                [c(109.0, 0.0), c(134.0, 0.0), c(169.0, 0.0)],
            ])
        );
        // M²（复矩阵，精确）
        assert_eq!(
            mat_m().mul(&mat_m()),
            Mat3C::new([
                [c(-1.0, 0.0), c(0.0, 2.0), c(1.0, 0.0)],
                [c(1.0, 0.0), c(-1.0, 0.0), c(0.0, 2.0)],
                [c(0.0, 2.0), c(1.0, 0.0), c(-1.0, 0.0)],
            ])
        );
        // 幂零：N²、N³
        let n = mat_n();
        assert_eq!(
            n.mul(&n),
            Mat3C::new([
                [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
                [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
                [c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
            ])
        );
        assert_eq!(n.mul(&n).mul(&n), Mat3C::ZERO);
    }

    // ---- 矩阵乘法：代数律 ----

    #[test]
    fn mul_algebraic_laws() {
        let a = mat_a();
        let m = mat_m();
        let d = Mat3C::from_diag([c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]);
        // 结合律 (AB)C = A(BC)
        assert_approx(&a.mul(&m).mul(&d), &a.mul(&m.mul(&d)));
        // 分配律 A(B+C) = AB + AC
        assert_approx(&a.mul(&m.add(&d)), &a.mul(&m).add(&a.mul(&d)));
        // 交换律一般不成立（验证非交换性，M 与 d 不交换）
        assert!(m.mul(&d) != d.mul(&m));
    }

    // ---- 标量运算 ----

    #[test]
    fn scalar_ops() {
        let a = mat_a();
        assert_eq!(a.mul_f64(2.0), a.map(|x| x.mul_f64(2.0)));
        assert_eq!(2.0 * a, a.mul_f64(2.0));
        assert_eq!(a * 2.0, a.mul_f64(2.0));
        assert_eq!(a / 2.0, a.mul_f64(0.5));
        assert_eq!(a.mul_complex(Complex::I), a.map(|x| x * Complex::I));
        assert_eq!(Complex::I * a, a.mul_complex(Complex::I));
        assert_eq!(a * Complex::I, a.mul_complex(Complex::I));
        assert_eq!(a.div_complex(c(2.0, 0.0)), a.mul_f64(0.5));
        // (s·A)·v == s·(A·v)
        let v = [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)];
        assert_vec_approx(
            a.mul_f64(3.0).mul_vec(v),
            a.mul_vec(v).map(|z| z.mul_f64(3.0)),
        );
    }

    #[test]
    fn assign_ops() {
        let a = mat_a();
        let mut x = a;
        x += a;
        assert_eq!(x, a.mul_f64(2.0));
        x -= a;
        assert_eq!(x, a);
        x *= 2.0;
        assert_eq!(x, a.mul_f64(2.0));
        x /= 2.0;
        assert_eq!(x, a);
        x *= Complex::I;
        assert_eq!(x, a.mul_complex(Complex::I));
        x /= Complex::I;
        assert_eq!(x, a);
        let mut y = Mat3C::IDENTITY;
        y *= a;
        assert_eq!(y, a);
    }

    // ---- 矩阵 × 向量 ----

    #[test]
    fn mul_vec() {
        let d = Mat3C::from_diag([c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]);
        let v = [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)];
        assert_eq!(d.mul_vec(v), [c(4.0, 0.0), c(10.0, 0.0), c(18.0, 0.0)]);
        assert_eq!(d * v, d.mul_vec(v)); // 运算符一致
        // 旋转：R(θ)·[1,0,1] = [cosθ, sinθ, 1]
        let (s, co) = 0.3f64.sin_cos();
        let r = Mat3C::new([
            [c(co, 0.0), c(-s, 0.0), c(0.0, 0.0)],
            [c(s, 0.0), c(co, 0.0), c(0.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
        ]);
        assert_vec_approx(
            r.mul_vec([c(1.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)]),
            [c(co, 0.0), c(s, 0.0), c(1.0, 0.0)],
        );
        // 转置乘 == 先转置再乘
        assert_vec_approx(
            r.transpose_mul_vec([c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)]),
            r.transpose()
                .mul_vec([c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)]),
        );
    }

    // ---- 行列式 ----

    #[test]
    fn det_values() {
        assert_eq!(mat_a().det(), c(-3.0, 0.0));
        assert_eq!(mat_m().det(), c(1.0, -1.0));
        assert_eq!(Mat3C::IDENTITY.det(), Complex::ONE);
        assert_eq!(Mat3C::ZERO.det(), Complex::ZERO);
        assert_eq!(
            Mat3C::from_diag([c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)]).det(),
            c(24.0, 0.0)
        );
        // 两行成比例 → 奇异
        let singular = Mat3C::new([
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(2.0, 0.0), c(4.0, 0.0), c(6.0, 0.0)],
            [c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)],
        ]);
        assert_eq!(singular.det(), Complex::ZERO);
        // det(Aᵀ) = det(A)
        assert_eq!(mat_a().transpose().det(), mat_a().det());
        // det(c·A) = c³·det(A)
        assert_eq!(mat_a().mul_f64(2.0).det(), mat_a().det().mul_f64(8.0));
        // det(AB) = det(A)·det(B)
        let dab = mat_m().mul(&mat_a()).det();
        let dadb = mat_m().det() * mat_a().det();
        assert!(dab.approx_eq(dadb, EPS), "det(AB)={dab} ≠ detA·detB={dadb}");
    }

    // ---- 转置 / 共轭 / 伴随 ----

    #[test]
    fn transpose_conj_adjoint() {
        let a = mat_a();
        let m = mat_m();
        assert_eq!(a.transpose().transpose(), a);
        assert_eq!(a.conj(), a); // 实矩阵共轭不变
        assert_eq!(m.conj().get(0, 0), c(0.0, -1.0));
        assert_eq!(m.conj().get(0, 1), c(1.0, 0.0));
        // (AB)ᵀ = BᵀAᵀ
        assert_approx(&m.mul(&a).transpose(), &a.transpose().mul(&m.transpose()));
        // (AB)† = B†A†
        assert_approx(&m.mul(&a).adjoint(), &a.adjoint().mul(&m.adjoint()));
        // 转置与共轭可交换
        assert_eq!(m.conj().transpose(), m.transpose().conj());
        // 伴随 == 共轭转置
        assert_eq!(m.adjoint(), m.conj().transpose());
    }

    // ---- 迹 ----

    #[test]
    fn trace_values() {
        assert_eq!(mat_a().trace(), c(16.0, 0.0)); // 1+5+10
        assert_eq!(mat_m().trace(), c(0.0, 3.0)); // 3i
        assert_eq!(Mat3C::IDENTITY.trace(), c(3.0, 0.0));
        // tr(AB) = tr(BA)
        let t1 = mat_m().mul(&mat_a()).trace();
        let t2 = mat_a().mul(&mat_m()).trace();
        assert!(t1.approx_eq(t2, EPS), "tr(AB)={t1} ≠ tr(BA)={t2}");
    }

    // ---- 余子式 / 代数余子式 ----

    #[test]
    fn minor_cofactor() {
        let a = mat_a();
        assert_eq!(a.minor(0, 0), c(2.0, 0.0)); // 5·10 − 6·8
        assert_eq!(a.minor(0, 1), c(-2.0, 0.0)); // 4·10 − 6·7
        assert_eq!(a.cofactor(0, 0), c(2.0, 0.0));
        assert_eq!(a.cofactor(0, 1), c(2.0, 0.0));
        assert_eq!(a.cofactor(0, 2), c(-3.0, 0.0));
        // 沿第 0 行展开 == det
        let expansion = a.get(0, 0) * a.cofactor(0, 0)
            + a.get(0, 1) * a.cofactor(0, 1)
            + a.get(0, 2) * a.cofactor(0, 2);
        assert_eq!(expansion, a.det());
    }

    // ---- 伴随矩阵 ----

    #[test]
    fn adjugate_values() {
        // adj(A) 已知值（整数，精确）
        assert_eq!(
            mat_a().adjugate(),
            Mat3C::new([
                [c(2.0, 0.0), c(4.0, 0.0), c(-3.0, 0.0)],
                [c(2.0, 0.0), c(-11.0, 0.0), c(6.0, 0.0)],
                [c(-3.0, 0.0), c(6.0, 0.0), c(-3.0, 0.0)],
            ])
        );
        assert_eq!(Mat3C::IDENTITY.adjugate(), Mat3C::IDENTITY);
        // A·adj(A) = det(A)·I（整数，精确）
        assert_eq!(
            mat_a().mul(&mat_a().adjugate()),
            Mat3C::from_scalar(c(-3.0, 0.0))
        );
        // M·adj(M) = det(M)·I
        assert_approx(
            &mat_m().mul(&mat_m().adjugate()),
            &Mat3C::from_scalar(c(1.0, -1.0)),
        );
    }

    // ---- 逆矩阵 ----

    #[test]
    fn try_inv() {
        assert_eq!(Mat3C::IDENTITY.try_inv(), Some(Mat3C::IDENTITY));
        assert_eq!(
            Mat3C::from_diag([c(2.0, 0.0), c(4.0, 0.0), c(8.0, 0.0)]).try_inv(),
            Some(Mat3C::from_diag([c(0.5, 0.0), c(0.25, 0.0), c(0.125, 0.0)]))
        );
        let a = mat_a();
        let inv = a.try_inv().unwrap();
        assert_approx(&a.mul(&inv), &Mat3C::IDENTITY);
        assert_approx(&inv.mul(&a), &Mat3C::IDENTITY);
        let m = mat_m();
        assert_approx(&m.mul(&m.try_inv().unwrap()), &Mat3C::IDENTITY);
        // 奇异矩阵 → None
        assert_eq!(Mat3C::ZERO.try_inv(), None);
        assert_eq!(mat_n().try_inv(), None);
        // (AB)⁻¹ = B⁻¹A⁻¹
        let ab_inv = m.mul(&a).try_inv().unwrap();
        assert_approx(&ab_inv, &a.try_inv().unwrap().mul(&m.try_inv().unwrap()));
        // is_invertible
        assert!(a.is_invertible());
        assert!(!mat_n().is_invertible());
        assert!(!Mat3C::ZERO.is_invertible());
    }

    // ---- 解线性方程组 ----

    #[test]
    fn solve_linear() {
        let d = Mat3C::from_diag([c(2.0, 0.0), c(4.0, 0.0), c(8.0, 0.0)]);
        assert_eq!(
            d.solve([c(2.0, 0.0), c(4.0, 0.0), c(8.0, 0.0)]),
            Some([c(1.0, 0.0), c(1.0, 0.0), c(1.0, 0.0)])
        );
        // 复系数往返：A·x = b ⇒ solve(b) = x
        let m = mat_m();
        let x = [c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.0)];
        let b = m.mul_vec(x);
        assert_vec_approx(m.solve(b).unwrap(), x);
        // 奇异 → None
        assert_eq!(
            Mat3C::ZERO.solve([c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)]),
            None
        );
        assert_eq!(mat_n().solve([c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)]), None);
    }

    // ---- 叉积矩阵 / 外积 ----

    #[test]
    fn cross_outer() {
        let v = [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)];
        let w = [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)];
        let cx = Mat3C::from_cross(v);
        // v × w = [2·6−3·5, 3·4−1·6, 1·5−2·4] = [−3, 6, −3]
        assert_eq!(cx.mul_vec(w), [c(-3.0, 0.0), c(6.0, 0.0), c(-3.0, 0.0)]);
        // v × v = 0
        assert_eq!(cx.mul_vec(v), [Complex::ZERO; 3]);
        // 反对称
        assert_eq!(cx.transpose(), cx.neg());
        // 外积 (u·vᵀ)·w = u·(v·w)
        let u = [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)];
        let v2 = [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)];
        let w2 = [c(1.0, 0.0), c(-1.0, 0.0), c(2.0, 0.0)];
        let outer = Mat3C::from_outer(u, v2);
        assert_eq!(outer.get(0, 0), c(4.0, 0.0));
        assert_eq!(outer.get(0, 1), c(5.0, 0.0));
        assert_eq!(outer.get(1, 2), c(12.0, 0.0));
        // v2·w2 = 4−5+12 = 11 → u·11
        assert_eq!(
            outer.mul_vec(w2),
            [c(11.0, 0.0), c(22.0, 0.0), c(33.0, 0.0)]
        );
    }

    // ---- 谓词 ----

    #[test]
    fn predicates() {
        assert!(Mat3C::from_diag([c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]).is_diagonal());
        assert!(!mat_a().is_diagonal());
        assert!(!mat_a().is_zero());
        assert!(!mat_a().is_identity());

        let sym = Mat3C::new([
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(2.0, 0.0), c(4.0, 0.0), c(5.0, 0.0)],
            [c(3.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
        ]);
        assert!(sym.is_symmetric());
        assert!(sym.is_hermitian()); // 实对称必为 Hermite
        assert!(!mat_m().is_symmetric());

        let herm = Mat3C::new([
            [c(2.0, 0.0), c(1.0, 1.0), c(2.0, -1.0)],
            [c(1.0, -1.0), c(3.0, 0.0), c(0.0, 1.0)],
            [c(2.0, 1.0), c(0.0, -1.0), c(1.0, 0.0)],
        ]);
        assert!(herm.is_hermitian());
        assert!(!herm.is_symmetric());
        // A + A† 恒为 Hermite（精确成立：共轭与加法逐元素交换）
        let m = mat_m();
        assert!(m.add(&m.adjoint()).is_hermitian());
    }

    // ---- Frobenius 范数 ----

    #[test]
    fn frobenius() {
        let d = Mat3C::from_diag([c(3.0, 0.0), c(0.0, 4.0), c(0.0, 0.0)]);
        assert_eq!(d.frobenius_norm_sqr(), 25.0);
        assert_eq!(d.frobenius_norm(), 5.0);
        assert_eq!(Mat3C::ZERO.frobenius_norm(), 0.0);
        assert_eq!(Mat3C::IDENTITY.frobenius_norm_sqr(), 3.0);
        // hypot 累积与直接 sqrt(3) 可能有 1 ulp 差异，用近似比较
        assert!((Mat3C::IDENTITY.frobenius_norm() - 3f64.sqrt()).abs() < 1e-12);
    }

    // ---- map / 近似比较 ----

    #[test]
    fn map_approx() {
        let m = mat_m();
        assert_eq!(m.map(|z| z.conj()), m.conj());
        assert_eq!(m.map(|z| z.mul_f64(2.0)), m.mul_f64(2.0));
        assert!(mat_a().approx_eq(&mat_a(), 0.0));
        let mut perturbed = mat_a();
        perturbed.set(0, 0, c(1.0000001, 0.0));
        assert!(mat_a().approx_eq(&perturbed, 1e-6));
        assert!(!mat_a().approx_eq(&perturbed, 1e-9));
    }

    // ---- 字符串输出 ----

    #[test]
    fn display_debug() {
        assert_eq!(
            Mat3C::IDENTITY.to_string(),
            "[1, 0, 0]\n[0, 1, 0]\n[0, 0, 1]"
        );
        let m = Mat3C::new([
            [c(3.0, 4.0), c(0.0, 5.0), c(-3.0, 0.0)],
            [c(1.0, -2.0), Complex::ZERO, c(0.0, -5.0)],
            [c(2.0, 0.0), c(-1.0, 0.5), Complex::ZERO],
        ]);
        let expected = "[3 + 4i, 5i, -3]\n[1 - 2i, 0, -5i]\n[2, -1 + 0.5i, 0]";
        assert_eq!(m.to_string(), expected);
        assert_eq!(format!("{m:?}"), format!("Mat3C({expected})"));
    }
}
