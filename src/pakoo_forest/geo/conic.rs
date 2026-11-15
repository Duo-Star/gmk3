// src/pakoo_forest/geo/conic.rs
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

use crate::pakoo_forest::calc::{Complex, Mat3C};
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;

/// Schur 补 `S = F − (C·D² − 2·B·D·E + A·E²)/(A·C − B²)`。
///
/// 仅当 `Δ∞ = B² − AC < 0`（`Q∞` 正定或负定）时有意义；
/// 调用方需先按 `Q∞ > 0` 归一化（`A < 0` 时整体乘 `−1`）。
#[inline]
fn schur_complement(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> f64 {
    f - (c * d * d - 2.0 * b * d * e + a * e * e) / (a * c - b * b)
}

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
             [ self.q.m[0][0].re,  self.q.m[0][1].re,  self.q.m[0][2].re],
             [ self.q.m[1][0].re,  self.q.m[1][1].re,  self.q.m[1][2].re],
             [ self.q.m[2][0].re,  self.q.m[2][1].re,  self.q.m[2][2].re],
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
    pub fn tangent(&self, p: &Dot, eps: f64) -> Option<Line> {
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

    /// 单位 Frobenius 范数代表元；零曲线或非有限时返回 `None`。
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

    /// 射影相等（与整体缩放无关）：`Q` 与 `Q'` 的 9 个分量为行构成的
    /// 2×9 矩阵全部 2×2 子式消失。`eps = 0` 时精确判定。
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

    // ------------------------------------------------------------------
    // 分类
    // ------------------------------------------------------------------

    /// 复射影分类。`eps = 0` 精确判零；`eps > 0` 归一化后容差判零。
    ///
    /// 输出："零型" | "二重直线" | "平行直线型" | "交叉直线型" |
    /// "圆" | "抛物线" | "横截型" | "无效型"（含 NaN/无穷）。
    pub fn classify_complex(&self, eps: f64) -> &'static str {
        if !self.is_finite() {
            return "无效型";
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let (a, b, c) = (q.a(), q.b(), q.c());
        let delta = b * b - a * c;
        let circle_like = (a - c).abs() <= eps && b.abs() <= eps;
        match self.rank(eps) {
            0 => "零型",
            1 => "二重直线",
            2 => {
                if delta.abs() <= eps {
                    "平行直线型"
                } else {
                    "交叉直线型"
                }
            }
            _ => {
                if circle_like {
                    "圆"
                } else if delta.abs() <= eps {
                    "抛物线"
                } else {
                    "横截型"
                }
            }
        }
    }

    /// 实系数分类（调用方保证系数为实或虚部可忽略；[`Conic::classify`] 自动分派）。
    ///
    /// "实圆" | "虚圆" |
    /// "实椭圆" | "虚椭圆" |
    /// "实抛物线" |
    /// "实双曲线" | "等轴双曲线" |
    /// "两条实平行线" | "两条实相交线" |
    /// "两条实垂直相交线" | "单点" | "点圆" | "仿射空集（两共轭复平行线）" |
    /// "二重实直线" | "零型" | "无效型"。
    pub fn classify_real(&self, eps: f64) -> &'static str {
        if !self.is_finite() {
            return "无效型";
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let (a, b, c) = (q.a().re, q.b().re, q.c().re);
        let (d, e, f) = (q.d().re, q.e().re, q.f().re);

        let delta = b * b - a * c;
        let parallel = delta.abs() <= eps;
        let circle_like = (a - c).abs() <= eps && b.abs() <= eps;
        let ac_zero = (a + c).abs() <= eps;

        match self.rank(eps) {
            0 => "零型",
            1 => "二重实直线",
            2 => {
                // 主 2×2 子式之和 = 两个非零特征值之积：
                // 负 → 不定 (1,1,1)；正 → 定号 (2,0,1)/(0,2,1)
                let m = (a * c - b * b) + (a * f - d * d) + (c * f - e * e);
                if m < -eps {
                    // 不定：两条实直线
                    if parallel {
                        "两条实平行线"
                    } else if ac_zero {
                        "两条实垂直相交线"
                    } else {
                        "两条实相交线"
                    }
                } else if parallel {
                    // 定号且核在无穷远直线上：两共轭复平行线，实射影为空
                    "仿射空集（两共轭复平行线）"
                } else if circle_like {
                    "点圆"
                } else {
                    "单点"
                }
            }
            _ => {
                // 秩 3
                if circle_like {
                    let (a2, b2, c2, d2, e2, f2) = if a < 0.0 {
                        (-a, -b, -c, -d, -e, -f)
                    } else {
                        (a, b, c, d, e, f)
                    };
                    let s = schur_complement(a2, b2, c2, d2, e2, f2);
                    if s < -eps {
                        "实圆"
                    } else if s > eps {
                        "虚圆"
                    } else {
                        "圆" // S ≈ 0 的临界情形（秩 3 时理论上不出现）
                    }
                } else if parallel {
                    "实抛物线"
                } else if delta < -eps {
                    // Δ∞ < 0：椭圆族
                    let (a2, b2, c2, d2, e2, f2) = if a < 0.0 {
                        (-a, -b, -c, -d, -e, -f)
                    } else {
                        (a, b, c, d, e, f)
                    };
                    let s = schur_complement(a2, b2, c2, d2, e2, f2);
                    if s < -eps {
                        "实椭圆"
                    } else if s > eps {
                        "虚椭圆"
                    } else {
                        "椭圆"
                    }
                } else if ac_zero {
                    // Δ∞ > 0 且 A + C = 0
                    "等轴双曲线"
                } else {
                    "实双曲线"
                }
            }
        }
    }

    /// 自动分派：实系数（`eps = 0` 时精确判定，否则按容差）走实分类，
    /// 否则走复射影分类。
    pub fn classify(&self, eps: f64) -> &'static str {
        let real = if eps == 0.0 {
            self.is_real()
        } else {
            let q = self.normalize().unwrap_or(*self);
            q.q.m.iter().flatten().all(|x| x.im.abs() <= eps)
        };
        if real {
            self.classify_real(eps)
        } else {
            self.classify_complex(eps)
        }
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
            "Conic(A: {}, B: {}, C: {}, D: {}, E: {}, F: {})",
            self.a(),
            self.b(),
            self.c(),
            self.d(),
            self.e(),
            self.f()
        )
    }
}

impl fmt::Debug for Conic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Conic;
    use crate::pakoo_forest::calc::{Complex, Mat3C};
    use crate::pakoo_forest::geo::{Dot, Line};

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let q = Conic::new(
            c(1.0, 0.0),
            c(2.0, 0.0),
            c(3.0, 0.0),
            c(4.0, 0.0),
            c(5.0, 0.0),
            c(6.0, 0.0),
        );
        assert_eq!(q.a(), c(1.0, 0.0));
        assert_eq!(q.b(), c(2.0, 0.0));
        assert_eq!(q.c(), c(3.0, 0.0));
        assert_eq!(q.d(), c(4.0, 0.0));
        assert_eq!(q.e(), c(5.0, 0.0));
        assert_eq!(q.f(), c(6.0, 0.0));
        // 矩阵布局：Q = [[A,B,D],[B,C,E],[D,E,F]]，对称
        assert_eq!(q.q.m[0][2], c(4.0, 0.0));
        assert_eq!(q.q.m[2][0], c(4.0, 0.0));
        assert_eq!(q.q.m[1][2], c(5.0, 0.0));
        assert!(q.is_symmetric());
        assert_eq!(Conic::from_f64s(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), q);
        // from_matrix 自动对称化：(Q + Qᵀ)/2
        let asym = Mat3C::new([
            [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)],
            [c(4.0, 0.0), c(5.0, 0.0), c(6.0, 0.0)],
            [c(7.0, 0.0), c(8.0, 0.0), c(9.0, 0.0)],
        ]);
        let sym = Conic::from_matrix(asym);
        assert!(sym.is_symmetric());
        assert_eq!(
            sym.q,
            Mat3C::new([
                [c(1.0, 0.0), c(3.0, 0.0), c(5.0, 0.0)],
                [c(3.0, 0.0), c(5.0, 0.0), c(7.0, 0.0)],
                [c(5.0, 0.0), c(7.0, 0.0), c(9.0, 0.0)],
            ])
        );
        assert_eq!(q.to_mat3c(), q.q);
        assert_eq!(
            Conic::UNIT_CIRCLE,
            Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, -1.0)
        );
        assert!(Conic::ZERO.is_zero());
        assert_eq!(Conic::default(), Conic::ZERO);
    }

    // ---- 取值 / 关联 ----

    #[test]
    fn evaluate_contains() {
        let unit = Conic::UNIT_CIRCLE; // x² + y² − z² = 0
        let p = Dot::from_f64s(1.0, 0.0, 1.0);
        assert_eq!(unit.evaluate(&p), Complex::ZERO);
        assert!(unit.contains(&p, 0.0));
        // 尺度无关：q(λp) = λ²·q(p)
        assert!(unit.contains(&p.scaled(c(2.0, 3.0)), 0.0));
        assert!(!unit.contains(&Dot::from_f64s(2.0, 0.0, 1.0), EPS));
        // 虚圆点在单位圆上
        assert_eq!(unit.evaluate(&Dot::I), Complex::ZERO);
        assert_eq!(unit.evaluate(&Dot::J), Complex::ZERO);
        // 容差
        let near = Dot::from_f64s(1.0 + 1e-10, 0.0, 1.0);
        assert!(unit.contains(&near, 1e-8));
        assert!(!unit.contains(&near, 1e-14));
    }

    // ---- 极线 / 切线 ----

    #[test]
    fn polar_tangent() {
        let unit = Conic::UNIT_CIRCLE;
        let p = Dot::from_f64s(1.0, 0.0, 1.0);
        // 切线 x − z = 0
        let l = unit.polar(&p);
        assert_eq!(
            (l.x(), l.y(), l.z()),
            (Complex::ONE, Complex::ZERO, Complex::NEG_ONE)
        );
        assert!(l.contains(&p, 0.0));
        let t = unit.tangent(&p, 0.0).unwrap();
        assert!(t.approx_eq(&l, 0.0));
        // 不在曲线上的点无切线
        assert_eq!(unit.tangent(&Dot::from_f64s(2.0, 0.0, 1.0), 0.0), None);
        // 圆心的极线是无穷远直线
        let center = Dot::ZERO; // (0:0:1)
        assert!(unit.polar(&center).approx_eq(&Line::INF, 0.0));
        // 抛物线 y² − 2xz 与无穷远直线相切于 (1:0:0)，该处切线即 L∞
        let par = Conic::from_f64s(0.0, 0.0, 1.0, -1.0, 0.0, 0.0);
        let inf_point = Dot::from_f64s(1.0, 0.0, 0.0);
        assert_eq!(par.evaluate(&inf_point), Complex::ZERO);
        let t_inf = par.tangent(&inf_point, 0.0).unwrap();
        assert!(t_inf.approx_eq(&Line::INF, 0.0));
    }

    // ---- 对偶 ----

    #[test]
    fn dual() {
        // 圆自对偶：adj(diag(1,1,−1)) = diag(−1,−1,1) = −Q
        assert!(
            Conic::UNIT_CIRCLE
                .dual()
                .approx_eq(&Conic::UNIT_CIRCLE, 0.0)
        );
        // adj(adj(Q)) = det(Q)·Q（整数精确）
        let q = Conic::from_f64s(2.0, 0.0, 3.0, 0.0, 0.0, 5.0);
        assert!(q.dual().dual().approx_eq(&q, 0.0));
        // 退化：两条相交直线的对偶是二重直线（对偶点为交点）
        let pair = Conic::from_f64s(1.0, 0.0, -1.0, 0.0, 0.0, 0.0); // x² − y²
        assert_eq!(pair.rank(0.0), 2);
        assert_eq!(pair.dual().rank(0.0), 1);
    }

    // ---- 秩 / 退化 / 圆 ----

    #[test]
    fn rank_deg_circle() {
        assert_eq!(Conic::ZERO.rank(0.0), 0);
        assert_eq!(Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).rank(0.0), 1); // x²
        assert_eq!(Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 0.0).rank(0.0), 2); // x²+y²
        assert_eq!(Conic::UNIT_CIRCLE.rank(0.0), 3);
        assert!(Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 0.0).is_degenerate(0.0));
        assert!(!Conic::UNIT_CIRCLE.is_degenerate(0.0));
        assert!(Conic::UNIT_CIRCLE.is_circle(0.0));
        assert!(!Conic::from_f64s(1.0, 0.0, 2.0, 0.0, 0.0, -1.0).is_circle(0.0));
        assert!(!Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 0.0).is_circle(0.0)); // 点圆不是非退化圆
        // 容差秩
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 1e-14).rank(0.0),
            3
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 1e-14).rank(1e-10),
            2
        );
    }

    // ---- 复射影分类 ----

    #[test]
    fn classify_complex_cases() {
        assert_eq!(Conic::ZERO.classify_complex(0.0), "零型");
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).classify_complex(0.0),
            "二重直线"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, -1.0).classify_complex(0.0),
            "平行直线型"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, -1.0, 0.0, 0.0, 0.0).classify_complex(0.0),
            "交叉直线型"
        );
        assert_eq!(Conic::UNIT_CIRCLE.classify_complex(0.0), "圆");
        assert_eq!(
            Conic::from_f64s(0.0, 0.0, 1.0, -1.0, 0.0, 0.0).classify_complex(0.0),
            "抛物线"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 2.0, 0.0, 0.0, -1.0).classify_complex(0.0),
            "横截型"
        );
        // 复系数：横截型恰过 I 不过 J（资料脚注的例）
        let q = Conic::new(
            Complex::ZERO,
            Complex::ONE,
            c(0.0, 2.0),
            Complex::ZERO,
            Complex::ZERO,
            Complex::ONE,
        );
        assert_eq!(q.classify_complex(0.0), "横截型");
        assert_eq!(q.evaluate(&Dot::I), Complex::ZERO);
        assert!(q.evaluate(&Dot::J) != Complex::ZERO);
        // 非有限 → 无效型
        assert_eq!(
            Conic::new(
                c(f64::NAN, 0.0),
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO
            )
            .classify_complex(0.0),
            "无效型"
        );
    }

    // ---- 实系数分类：非退化 ----

    #[test]
    fn classify_real_nondegenerate() {
        // 圆
        assert_eq!(Conic::UNIT_CIRCLE.classify_real(0.0), "实圆");
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 1.0).classify_real(0.0),
            "虚圆"
        );
        // Q∞ 负定：整体乘 −1 后判 S
        assert_eq!(
            Conic::from_f64s(-1.0, 0.0, -1.0, 0.0, 0.0, 1.0).classify_real(0.0),
            "实圆"
        );
        // 椭圆
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 2.0, 0.0, 0.0, -1.0).classify_real(0.0),
            "实椭圆"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 2.0, 0.0, 0.0, 1.0).classify_real(0.0),
            "虚椭圆"
        );
        assert_eq!(
            Conic::from_f64s(-1.0, 0.0, -2.0, 0.0, 0.0, 1.0).classify_real(0.0),
            "实椭圆"
        );
        // 抛物线
        assert_eq!(
            Conic::from_f64s(0.0, 0.0, 1.0, -1.0, 0.0, 0.0).classify_real(0.0),
            "实抛物线"
        );
        // 双曲线
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, -2.0, 0.0, 0.0, -1.0).classify_real(0.0),
            "实双曲线"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, -1.0, 0.0, 0.0, -1.0).classify_real(0.0),
            "等轴双曲线"
        );
    }

    // ---- 实系数分类：退化 ----

    #[test]
    fn classify_real_degenerate() {
        // r = 2，不定：两条实直线
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, -1.0).classify_real(0.0),
            "两条实平行线"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, -1.0, 0.0, 0.0, 0.0).classify_real(0.0),
            "两条实垂直相交线"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, -2.0, 0.0, 0.0, 0.0).classify_real(0.0),
            "两条实相交线"
        );
        // r = 2，定号
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 0.0).classify_real(0.0),
            "点圆"
        );
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 2.0, 0.0, 0.0, 0.0).classify_real(0.0),
            "单点"
        );
        // 勘误 1 的例：X² + Z² = 0 → 定号且 p_Z = 0 → 仿射空集
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, 1.0).classify_real(0.0),
            "仿射空集（两共轭复平行线）"
        );
        // r = 1 / r = 0
        assert_eq!(
            Conic::from_f64s(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).classify_real(0.0),
            "二重实直线"
        );
        assert_eq!(Conic::ZERO.classify_real(0.0), "零型");
    }

    // ---- 自动分派与容差 ----

    #[test]
    fn classify_dispatch_tolerance() {
        // 实系数走实分类
        assert_eq!(Conic::UNIT_CIRCLE.classify(0.0), "实圆");
        assert_eq!(Conic::UNIT_CIRCLE.classify_complex(0.0), "圆");
        // 复系数走复分类
        let q = Conic::new(
            c(1.0, 1.0),
            Complex::ZERO,
            c(1.0, 1.0),
            Complex::ZERO,
            Complex::ZERO,
            c(-1.0, 0.0),
        );
        assert_eq!(q.classify(0.0), "圆");
        // 微扰后的圆：精确路径判为椭圆，容差路径判为实圆
        let perturbed = Conic::from_f64s(1.0 + 1e-14, 1e-15, 1.0, 0.0, 0.0, -1.0);
        assert_eq!(perturbed.classify_real(0.0), "实椭圆");
        assert_eq!(perturbed.classify_real(1e-10), "实圆");
        assert_eq!(perturbed.classify(1e-10), "实圆");
    }

    // ---- 代表元运算 / 射影相等 ----

    #[test]
    fn scaled_conj_normalize_approx() {
        let q = Conic::new(
            c(2.0, 0.5),
            c(0.0, -0.25),
            c(3.0, 0.0),
            c(0.1, 0.0),
            c(-0.2, 0.3),
            c(1.0, 0.0),
        );
        // 与整体缩放无关（反号精确；一般复数缩放有舍入，用容差）
        assert!(q.approx_eq(&q.scaled(c(-1.0, 0.0)), 0.0));
        assert!(q.approx_eq(&q.scaled(c(0.5, 0.0)), EPS));
        assert!(q.approx_eq(&q.scaled(c(2.0, -1.0)), EPS));
        assert!(!q.approx_eq(&q.conj(), EPS));
        // 归一化
        let n = q.normalize().unwrap();
        assert!((n.q.frobenius_norm() - 1.0).abs() < EPS);
        assert!(n.approx_eq(&q, EPS));
        assert_eq!(Conic::ZERO.normalize(), None);
        // 共轭
        assert_eq!(
            q.conj(),
            Conic::new(
                c(2.0, -0.5),
                c(0.0, 0.25),
                c(3.0, 0.0),
                c(0.1, 0.0),
                c(-0.2, -0.3),
                c(1.0, 0.0)
            )
        );
    }

    // ---- 谓词 / 输出 ----

    #[test]
    fn predicates_display() {
        assert!(Conic::UNIT_CIRCLE.is_real());
        assert!(
            !Conic::new(
                Complex::I,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO
            )
            .is_real()
        );
        assert!(Conic::UNIT_CIRCLE.is_finite());
        assert!(
            !Conic::new(
                c(f64::INFINITY, 0.0),
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO,
                Complex::ZERO
            )
            .is_finite()
        );
        assert_eq!(
            Conic::UNIT_CIRCLE.to_string(),
            "Conic(A: 1, B: 0, C: 1, D: 0, E: 0, F: -1)"
        );
        assert_eq!(
            format!("{:?}", Conic::ZERO),
            "Conic(A: 0, B: 0, C: 0, D: 0, E: 0, F: 0)"
        );
    }
}
