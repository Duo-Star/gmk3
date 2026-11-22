// src/pakoo_forest/geo/conic/conic.rs
//
//! 二次曲线（圆锥曲线）`Conic` —— 对称 3×3 复矩阵 `Q` 的 Newtype。
//!
//! 方程（齐次）：
//!
//! ```text
//! q = A·x² + 2B·xy + C·y² + 2D·xz + 2E·yz + F·z² = 0，
//! Q = [A B D; B C E; D E F]
//! ```
//!
//! 曲线上的点满足 `pᵀ·Q·p = 0`（[`Conic::evaluate`]）。
//!
//! # 分类
//!
//! [`Conic::classify_complex`] 给出复射影类别（二重直线 / 平行直线型 /
//! 交叉直线型 / 圆 / 抛物线 / 横截型，另含零型与无效型）；
//! [`Conic::classify_real`] 给出实系数下的细分（实圆/虚圆、实椭圆/虚椭圆、
//! 实抛物线、实双曲线/等轴双曲线、两条实平行线、两条实相交线（可垂直）、
//! 单点/点圆、仿射空集（两共轭复平行线）、二重实直线）。
//! [`Conic::classify`] 按系数是否为实自动分派。
//!
//! 判别核心（`Δ∞ = B² − AC`）：
//!
//! - 平行型：`r = 2 且 Δ∞ = 0`；交叉型：`r = 2 且 Δ∞ ≠ 0`；
//! - 圆：`r = 3 且 A = C、B = 0`（过两个虚圆点 `I, J`）；
//! - 抛物线：`r = 3 且 Δ∞ = 0`（与无穷远直线相切）；
//! - 横截型：`r = 3、Δ∞ ≠ 0 且非圆`；实系数下 `Δ∞ < 0` 为椭圆、`Δ∞ > 0` 为双曲线；
//! - 实系数两线相交垂直 / 双曲线等轴：`A + C = 0`。
//!
//! 退化实情形（`r = 2`）用主 2×2 子式之和 `M = (AC−B²) + (AF−D²) + (CF−E²)`
//! （等于两个非零特征值之积）区分不定（两实线）与定号（单点/仿射空集）。
//!
//! # 数值路径
//!
//! `eps = 0`：精确判零（适合整数/精确构造的输入）；
//! `eps > 0`：先归一化（正实因子，不改变符号），再用容差判零。

use std::fmt;

use crate::pakoo_forest::calc::{Complex, Mat3C, Vec3C};
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;

/// 齐次二次曲线，由对称系数矩阵 `Q` 表示，定义到整体非零比例。
#[derive(Clone, Copy, PartialEq)]
pub struct Conic {
    /// 对称系数矩阵（行优先）：`[[A, B, D], [B, C, E], [D, E, F]]`
    pub q: Mat3C,
}

impl Conic {
    /// 零曲线（`Q = 0`）
    pub const ZERO: Conic = Conic { q: Mat3C::ZERO };
    /// 单位圆 `x² + y² − z² = 0`
    pub const UNIT_CIRCLE: Conic = Conic {
        q: Mat3C::new([
            [Complex::ONE, Complex::ZERO, Complex::ZERO],
            [Complex::ZERO, Complex::ONE, Complex::ZERO],
            [Complex::ZERO, Complex::ZERO, Complex::NEG_ONE],
        ]),
    };

    // ------------------------------------------------------------------
    // 构造与分解
    // ------------------------------------------------------------------

    /// 由 6 个系数构造：`A·x² + 2B·xy + C·y² + 2D·xz + 2E·yz + F·z²`。
    #[inline]
    pub const fn new(
        a: Complex,
        b: Complex,
        c: Complex,
        d: Complex,
        e: Complex,
        f: Complex,
    ) -> Conic {
        Conic {
            q: Mat3C::new([[a, b, d], [b, c, e], [d, e, f]]),
        }
    }

    /// 由 6 个实系数构造。
    #[inline]
    pub const fn from_f64s(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Conic {
        Conic::new(
            Complex::from_f64(a),
            Complex::from_f64(b),
            Complex::from_f64(c),
            Complex::from_f64(d),
            Complex::from_f64(e),
            Complex::from_f64(f),
        )
    }

    /// 由矩阵构造，自动对称化 `(Q + Qᵀ)/2`（二次型只依赖对称部分）。
    #[inline]
    pub fn from_matrix(q: Mat3C) -> Conic {
        Conic {
            q: q.add(&q.transpose()).mul_f64(0.5),
        }
    }

    /// 系数矩阵。
    #[inline]
    pub const fn to_mat3c(self) -> Mat3C {
        self.q
    }

    /// [[A, B, D], [B, C, E], [D, E, F]]
    pub const fn to_3x3f64(self) -> [[f64; 3]; 3] {
        [
            [self.q.m[0][0].re, self.q.m[0][1].re, self.q.m[0][2].re],
            [self.q.m[1][0].re, self.q.m[1][1].re, self.q.m[1][2].re],
            [self.q.m[2][0].re, self.q.m[2][1].re, self.q.m[2][2].re],
        ]
    }

    /// 系数 `A`。
    #[inline]
    pub fn a(&self) -> Complex {
        self.q.m[0][0]
    }
    /// 系数 `B`。
    #[inline]
    pub fn b(&self) -> Complex {
        self.q.m[0][1]
    }
    /// 系数 `C`。
    #[inline]
    pub fn c(&self) -> Complex {
        self.q.m[1][1]
    }
    /// 系数 `D`。
    #[inline]
    pub fn d(&self) -> Complex {
        self.q.m[0][2]
    }
    /// 系数 `E`。
    #[inline]
    pub fn e(&self) -> Complex {
        self.q.m[1][2]
    }
    /// 系数 `F`。
    #[inline]
    pub fn f(&self) -> Complex {
        self.q.m[2][2]
    }

    // ------------------------------------------------------------------
    // 几何运算
    // ------------------------------------------------------------------

    /// 曲线在点处的取值 `q(p) = pᵀ·Q·p`（为 0 ⇔ 点在曲线上）。
    #[inline]
    #[must_use]
    pub fn evaluate(&self, p: &Dot) -> Complex {
        let qp = self.q.mul_vec(p.p);
        p.p.transpose().dot(qp)
    }

    /// 谓词  点是否在曲线上（与曲线/点的整体缩放无关）。`eps = 0` 时精确判定。
    #[inline]
    pub fn contains(&self, p: &Dot, eps: f64) -> bool {
        let s = eps * (1.0 + self.q.frobenius_norm() * p.norm() * p.norm());
        self.evaluate(p).abs() <= s
    }

    /// 点 `p` 关于曲线的极线 `Q·p`。`p` 在曲线上时即切线；
    /// `p` 为奇异点（核向量）时得到无效直线。
    #[inline]
    #[must_use]
    pub fn polar(&self, p: &Dot) -> Line {
        Line::from_cov3c(self.q.mul_vec(p.p).transpose())
    }

    /// 点 `p` 处的切线：要求 `p` 在曲线上且非奇异点，否则返回 `None`。
    #[inline]
    pub fn tangent_or_not(&self, p: &Dot, eps: f64) -> Option<Line> {
        if !self.contains(p, eps) {
            return None;
        }
        let l = self.polar(p);
        if l.is_zero() {
            return None;
        }
        Some(l)
    }

    /// 对偶曲线（伴随矩阵）：非退化时等价于 `Q⁻¹`（差标量因子），
    /// 表示曲线的切线包络；退化时描述两直线交点的对偶。
    #[inline]
    #[must_use]
    pub fn dual(&self) -> Conic {
        Conic {
            q: self.q.adjugate(),
        }
    }

    #[must_use]
    pub fn intersect_line(&self, l: &Line) -> Ddot {
        let (a, b, c) = (l.x(), l.y(), l.z());

        // 直线 ax+by+cz=0 上的两点：
        //   P = (-b, a, 0)，R = (-c, 0, a)
        let p = Dot {
            p: Vec3C::new(-b, a, Complex::ZERO),
        };
        let r = Dot {
            p: Vec3C::new(-c, Complex::ZERO, a),
        };

        // 限制到直线的二次型系数
        let qp = self.q.mul_vec(p.p);
        let qr = self.q.mul_vec(r.p);
        let aa = p.p.dot(qp.transpose()); // Pᵀ Q P
        let bb = p.p.dot(qr.transpose()); // Pᵀ Q R
        let cc = r.p.dot(qr.transpose()); // Rᵀ Q R

        // A·u² + 2B·uv + C·v² = 0 的两个参数根：
        //   (u:v) = (-B ± √(B² − A·C) : A)
        let s = (bb * bb - aa * cc).sqrt();
        let x1 = p.p.mul_complex(s - bb).add(r.p.mul_complex(aa));
        let x2 = p.p.mul_complex(-s - bb).add(r.p.mul_complex(aa));

        // [Dot { p: x1 }, Dot { p: x2 }]
        Ddot::from_vec3cs(x1, x2)
    }

    // ------------------------------------------------------------------
    // 代表元运算（不改变射影意义）
    // ------------------------------------------------------------------

    /// 逐系数共轭。
    #[inline]
    #[must_use]
    pub fn conj(&self) -> Conic {
        Conic { q: self.q.conj() }
    }

    /// 整体缩放（同一曲线）。
    #[inline]
    #[must_use]
    pub fn scaled(&self, s: Complex) -> Conic {
        Conic {
            q: self.q.mul_complex(s),
        }
    }

    /// 单位 Frobenius 范数代表元
    /// 零曲线或非有限时返回 `None`。
    #[inline]
    pub fn normalize(&self) -> Option<Conic> {
        let n = self.q.frobenius_norm();
        if n == 0.0 || !n.is_finite() {
            return None;
        }
        Some(Conic {
            q: self.q.mul_f64(1.0 / n),
        })
    }

    /// Frobenius 范数。
    #[inline]
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.q.frobenius_norm()
    }

    // ------------------------------------------------------------------
    // 谓词 / 近似比较
    // ------------------------------------------------------------------

    /// 零曲线（`Q = 0`）。
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.q.is_zero()
    }

    /// 系数矩阵对称（构造路径保证；直接改字段可能破坏）。
    #[inline]
    pub fn is_symmetric(&self) -> bool {
        self.q.is_symmetric()
    }

    /// 所有系数均为实数（实曲线）。
    #[inline]
    pub fn is_real(&self) -> bool {
        self.q.m.iter().flatten().all(|x| x.is_real())
    }

    /// 所有系数均有限。
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.q.m.iter().flatten().all(|x| x.is_finite())
    }

    /// 矩阵秩（0..=3）。`eps = 0` 精确判定；`eps > 0` 归一化后容差判定。
    /// rank(C)=3   非退化
    /// ..=2   两条相异直线
    /// ..=1   重合直线
    #[inline]
    pub fn rank(&self, eps: f64) -> usize {
        let q = &self.q;
        if eps == 0.0 {
            if q.is_zero() {
                return 0;
            }
            if q.det() != Complex::ZERO {
                return 3;
            }
            let all_minors_zero = (0..3).all(|r| (0..3).all(|c| q.minor(r, c) == Complex::ZERO));
            return if all_minors_zero { 1 } else { 2 };
        }
        let qn = match self.normalize() {
            Some(n) => n.q,
            None => return if q.is_zero() { 0 } else { 3 },
        };
        if qn.det().abs() > eps {
            return 3;
        }
        if qn.m.iter().flatten().all(|x| x.abs() <= eps) {
            return 0;
        }
        if (0..3).all(|r| (0..3).all(|c| qn.minor(r, c).abs() <= eps)) {
            return 1;
        }
        2
    }

    /// 是否退化（秩 < 3）。
    #[inline]
    pub fn is_degenerate(&self, eps: f64) -> bool {
        self.rank(eps) < 3
    }

    /// 是否非退化圆（`r = 3` 且 `A = C、B = 0`，即过两个虚圆点）。
    #[inline]
    pub fn is_circle(&self, eps: f64) -> bool {
        if self.rank(eps) != 3 {
            return false;
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let (a, b, c) = (q.a(), q.b(), q.c());
        (a - c).abs() <= eps && b.abs() <= eps
    }

    /// 射影意义相等：`Q` 与 `Q'` 的 9 个分量为行构成的
    /// 与整体缩放无关
    /// 2×9 矩阵全部 2×2 子式消失
    /// `eps = 0` 时精确判定。
    #[inline]
    pub fn approx_eq(&self, other: &Conic, eps: f64) -> bool {
        if self.q.is_zero() || other.q.is_zero() {
            return self.q.is_zero() && other.q.is_zero();
        }
        let scale = 1.0 + self.q.frobenius_norm() * other.q.frobenius_norm();
        let a: Vec<Complex> = self.q.m.iter().flatten().copied().collect();
        let b: Vec<Complex> = other.q.m.iter().flatten().copied().collect();
        for i in 1..9 {
            for j in 0..i {
                let minor = a[i] * b[j] - a[j] * b[i];
                if minor.abs() > eps * scale {
                    return false;
                }
            }
        }
        true
    }
}

impl Default for Conic {
    /// 默认为零曲线。
    fn default() -> Conic {
        Conic::ZERO
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Conic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Conic:(A: {}, B: {}, C: {}, D: {}, E: {}, F: {})",
            self.a(),
            self.b(),
            self.c(),
            self.d(),
            self.e(),
            self.f(),
            // self.classify(1e-12)
        )
    }
}

impl fmt::Debug for Conic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
