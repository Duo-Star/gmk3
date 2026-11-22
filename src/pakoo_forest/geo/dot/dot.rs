// src/pakoo_forest/geo/dot/dot.rs
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
use crate::pakoo_forest::geo::conic::Conic;
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

    /// 仿射坐标（实数），要求虚部在容差 `eps` 内可忽略；无穷远或虚点返回 `None`。
    #[inline]
    pub fn try_to_affine_f64(&self, eps: f64) -> Option<(f64, f64)> {
        let (ax, ay) = self.to_affine()?;
        if !ax.is_finite() || !ay.is_finite() {
            return None;
        }
        let scale = 1.0 + ax.abs().max(ay.abs());
        if ax.im.abs() > eps * scale || ay.im.abs() > eps * scale {
            return None;
        }
        Some((ax.re, ay.re))
    }
    /// 仿射坐标（实数，默认容差 `1e-12`）；无穷远或虚点返回 `None`。
    #[inline]
    pub fn to_affine_f64(&self) -> Option<(f64, f64)> {
        self.try_to_affine_f64(1e-12)
    }


    /// 导出实齐次坐标（带去全局复相位与虚部容差检测）；非射影实点或含非有限分量返回 `None`。
    pub fn try_to_3f64(&self, eps: f64) -> Option<[f64; 3]> {
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

    /// 仅当点为射影实点（默认容差 `1e-12`）时返回实坐标，否则返回全 `NaN`。
    #[inline]
    pub fn to_3f64(&self) -> [f64; 3] {
        self.try_to_3f64(1e-12).unwrap_or([f64::NAN; 3])
    }

    /// 嵌入为点圆二次曲线（圆心在 `self`、半径为 0）：
    /// `(x·z₀ − x₀·z)² + (y·z₀ − y₀·z)² = 0`。
    #[inline]
    #[must_use]
    pub fn as_conic(&self) -> Conic {
        let (x, y, z) = (self.x(), self.y(), self.z());
        let z2 = z * z;
        Conic::new(z2, Complex::ZERO, z2, -x * z, -y * z, x * x + y * y)
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

    /// 计算与另一点的实数模长距离（仿射差向量的欧氏范数 `sqrt(|dx|^2 + |dy|^2)`）。
    ///
    /// 与两点的齐次缩放无关；若任一点在无穷远（`z == 0`），返回非有限值（`NaN` / `inf`）[cite: 1]。
    #[inline]
    pub fn distance_f64(&self, other: &Dot) -> f64 {
        let (p, q) = (&self.p.v, &other.p.v);
        let (z1, z2) = (p[2], q[2]);
        let dx = p[0] * z2 - q[0] * z1;
        let dy = p[1] * z2 - q[1] * z1;
        let dz = z1 * z2;
        let d2 = dx.norm_sqr() + dy.norm_sqr();
        d2.sqrt() / dz.abs()
    }

    /// 计算与另一点的复平方距离 `(dx)^2 + (dy)^2`（全纯二次型，无 `sqrt` 分支切割）。
    ///
    /// 与两点的齐次缩放无关；若任一点在无穷远（`z == 0`），返回非有限值[cite: 1]。
    #[inline]
    pub fn distance_sqr_complex(&self, other: &Dot) -> Complex {
        let (p, q) = (&self.p.v, &other.p.v);
        let (z1, z2) = (p[2], q[2]);
        let dx = p[0] * z2 - q[0] * z1;
        let dy = p[1] * z2 - q[1] * z1;
        let dz = z1 * z2;
        (dx * dx + dy * dy) / (dz * dz)
    }

    /// 计算与另一点的复距离 `sqrt((dx)^2 + (dy)^2)`（取 `Complex::sqrt()` 主值分支）。
    ///
    /// 与两点的齐次缩放无关；若任一点在无穷远（`z == 0`），返回非有限值[cite: 1]。
    #[inline]
    pub fn distance_complex(&self, other: &Dot) -> Complex {
        self.distance_sqr_complex(other).sqrt()
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
        write!(f, "Dot:({} : {} : {})", self.x(), self.y(), self.z())
    }
}

impl fmt::Debug for Dot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dot:Dot({} : {} : {})", self.x(), self.y(), self.z())
    }
}
