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
//! # 数值路径
//!
//! `eps = 0`：精确判零（适合整数/精确构造的输入）；
//! `eps > 0`：先归一化（正实因子，不改变符号），再用容差判零。

use std::fmt;

use crate::pakoo_forest::calc::{Complex, Mat3C, Vec3C};
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;
use crate::pakoo_forest::geo::x_line::XLine;

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

    /// 导出实系数矩阵（带去全局复相位与虚部容差检测）。
    ///
    /// 若存在非有限分量或（去相位后）仍有大于 `eps * max_abs` 的虚部，返回 `None`。
    pub fn try_to_3x3f64(self, eps: f64) -> Option<[[f64; 3]; 3]> {
        let m = &self.q.m;
        let mut max_rc = (0usize, 0usize);
        let mut max_abs = 0.0f64;
        for (r, row) in m.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                if !v.is_finite() {
                    return None;
                }
                let a = v.abs();
                if a > max_abs {
                    max_abs = a;
                    max_rc = (r, c);
                }
            }
        }
        if max_abs == 0.0 {
            return Some([[0.0; 3]; 3]);
        }
        let tol = eps * max_abs;
        let already_real = m.iter().flatten().all(|v| v.im.abs() <= tol);
        if already_real {
            return Some([
                [m[0][0].re, m[0][1].re, m[0][2].re],
                [m[1][0].re, m[1][1].re, m[1][2].re],
                [m[2][0].re, m[2][1].re, m[2][2].re],
            ]);
        }
        let phase_conj = m[max_rc.0][max_rc.1].conj().div_f64(max_abs);
        let mut out = [[0.0; 3]; 3];
        for r in 0..3 {
            for c in 0..3 {
                let z = m[r][c] * phase_conj;
                if z.im.abs() > tol {
                    return None;
                }
                out[r][c] = z.re;
            }
        }
        Some(out)
    }

    /// `[[A, B, D], [B, C, E], [D, E, F]]`（默认容差 `1e-12`；非射影实曲线返回全 `NaN` 矩阵）。
    #[inline]
    pub fn to_3x3f64(self) -> [[f64; 3]; 3] {
        self.try_to_3x3f64(1e-12).unwrap_or([[f64::NAN; 3]; 3])
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

    /// 谓词：点是否在曲线上（与曲线/点的整体缩放无关）。`eps = 0` 时精确判定。
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

    /// 直线与二次曲线求交，返回骈点 `Ddot`。
    ///
    /// 采用 Richter-Gebert 反对称秩-1 分解：
    /// `B = [l]×ᵀ · Q · [l]×`，取 `M = B + α·[l]×` 使其秩降至 1，
    /// 则 `M` 的最大模长列与最大模长行即所求两个交点。
    ///
    /// 无坐标系与基底选择假设，兼容水平/垂直线、`Line::INF`、迷向直线、切线及超大/超小尺度。
    /// 若曲线或直线无效（或直线完全包含于退化曲线中），返回含零点的 `Ddot`。
    #[must_use]
    pub fn intersect_line(&self, l: &Line) -> Ddot {
        let zero = Ddot::from_vec3cs(Vec3C::ZERO, Vec3C::ZERO);
        let qn = match self.normalize() {
            Some(n) => n,
            None => return zero,
        };
        let ln = match l.normalize() {
            Some(n) => n,
            None => return zero,
        };

        let lv = Vec3C::from_array(ln.to_array());
        let lx = Mat3C::from_cross(lv);
        // [l]×ᵀ = -[l]×，故 B = [l]×ᵀ · Q · [l]× 对称且满足 B·l = 0
        let b = lx.transpose().mul(&qn.q).mul(&lx);

        // 选取模长最大的分量 k，避免除零（包括迷向直线 a²+b²+c²=0 情形）
        let mut k = 0usize;
        let mut max_l = lv.v[0].abs();
        for i in 1..3 {
            let a = lv.v[i].abs();
            if a > max_l {
                max_l = a;
                k = i;
            }
        }
        if max_l == 0.0 || !max_l.is_finite() {
            return zero;
        }

        let alpha = (-b.cofactor(k, k)).sqrt() / lv.v[k];
        let m = b.add(&lx.mul_complex(alpha));

        // 从秩-1 矩阵 M = x1·x2ᵀ 中选取最大模长元，其对应列/行必非零
        let mut r_max = 0usize;
        let mut c_max = 0usize;
        let mut max_m = 0.0f64;
        for r in 0..3 {
            for c in 0..3 {
                let a = m.m[r][c].abs();
                if a > max_m {
                    max_m = a;
                    r_max = r;
                    c_max = c;
                }
            }
        }
        if max_m == 0.0 || !max_m.is_finite() {
            return zero;
        }

        let x1 = m.col_vec(c_max).normalize().unwrap_or(Vec3C::ZERO);
        let x2 = m
            .row_cov(r_max)
            .transpose()
            .normalize()
            .unwrap_or(Vec3C::ZERO);
        Ddot::from_vec3cs(x1, x2)
    }

    // ------------------------------------------------------------------
    // 退化类型判断与快速转换（无分支）
    // ------------------------------------------------------------------

    /// 是否为二重直线（秩 1）。
    #[inline]
    pub fn is_line(&self, eps: f64) -> bool {
        self.rank(eps) == 1
    }

    /// 按二重直线直接提取直线（不校验秩，调用方保证数学前提）。
    ///
    /// 对秩-1 矩阵 `Q = lᵀ·l`，取模长最大行即为 `l`（至多差非零标量）。
    #[inline]
    #[must_use]
    pub fn as_line(&self) -> Line {
        let mut r_max = 0usize;
        let mut max_n = 0.0f64;
        for r in 0..3 {
            let n = self.q.row_cov(r).norm();
            if n > max_n {
                max_n = n;
                r_max = r;
            }
        }
        self.q
            .row_cov(r_max)
            .normalize()
            .map(Line::from_cov3c)
            .unwrap_or(Line::from_cov3c(self.q.row_cov(0)))
    }

    /// 是否为两条相异直线（交叉直线，秩 2）。
    #[inline]
    pub fn is_xline(&self, eps: f64) -> bool {
        self.rank(eps) == 2
    }

    /// 按退化两直线直接分解为 `XLine`（不强制校验，失败时返回零 `XLine`）。
    #[inline]
    #[must_use]
    pub fn as_xline(&self) -> XLine {
        let (l1, l2) = self.decompose_lines(1e-12).unwrap_or((
            Line::new(Complex::ZERO, Complex::ZERO, Complex::ZERO),
            Line::new(Complex::ZERO, Complex::ZERO, Complex::ZERO),
        ));
        XLine::new(l1, l2)
    }

    /// 是否为单点/点圆（射影实系数、秩 2、非平行且半定：实轨迹为唯一有限点）。
    #[inline]
    pub fn is_dot(&self, eps: f64) -> bool {
        if self.rank(eps) != 2 {
            return false;
        }
        let qn = if eps == 0.0 {
            *self
        } else {
            match self.normalize() {
                Some(n) => n,
                None => return false,
            }
        };
        let real_m = match qn.try_to_3x3f64(eps) {
            Some(m) => m,
            None => return false,
        };
        let (a, b, c) = (real_m[0][0], real_m[0][1], real_m[1][1]);
        let (d, e, f) = (real_m[0][2], real_m[1][2], real_m[2][2]);
        let delta = b * b - a * c;
        if delta.abs() <= eps {
            return false;
        }
        let m = (a * c - b * b) + (a * f - d * d) + (c * f - e * e);
        m > eps
    }

    /// 提取退化二次曲线的唯一奇异点（对“单点/点圆”即该点本身，对“交叉直线”即其交点；不校验秩）。
    ///
    /// 对秩-2 矩阵 `Q`，`adj(Q) ∝ p·pᵀ`，取 `adj(Q)` 模长最大列即为核向量 `p`。
    #[inline]
    #[must_use]
    pub fn as_dot(&self) -> Dot {
        let adj = self.q.adjugate();
        let mut c_max = 0usize;
        let mut max_n = 0.0f64;
        for c in 0..3 {
            let n = adj.col_vec(c).norm();
            if n > max_n {
                max_n = n;
                c_max = c;
            }
        }
        let v = adj.col_vec(c_max);
        Dot::from_vec3c(v.normalize().unwrap_or(v))
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

    /// 射影意义相等：`Q` 与 `Q'` 的 9 个分量构成的 2×9 矩阵全部 2×2 子式消失。
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
        )
    }
}

impl fmt::Debug for Conic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod stage1_conic_tests {
    use super::Conic;
    use crate::pakoo_forest::calc::Complex;
    use crate::pakoo_forest::geo::dot::Dot;
    use crate::pakoo_forest::geo::line::Line;
    use crate::pakoo_forest::geo::x_line::XLine;

    const EPS: f64 = 1e-11;

    #[test]
    fn intersect_line_all_axes_and_edge_cases() {
        let unit = Conic::UNIT_CIRCLE;

        // X 轴 (0:1:0)
        let dx = unit.intersect_line(&Line::X_AXIS);
        assert!(unit.contains(&dx.p(), EPS) && unit.contains(&dx.q(), EPS));
        assert!(Line::X_AXIS.contains(&dx.p(), EPS) && Line::X_AXIS.contains(&dx.q(), EPS));

        // Y 轴 (1:0:0)
        let dy = unit.intersect_line(&Line::Y_AXIS);
        assert!(unit.contains(&dy.p(), EPS) && unit.contains(&dy.q(), EPS));
        assert!(Line::Y_AXIS.contains(&dy.p(), EPS) && Line::Y_AXIS.contains(&dy.q(), EPS));

        // 无穷远直线 (0:0:1) → 虚圆点 {I, J}
        let dinf = unit.intersect_line(&Line::INF);
        assert!(dinf.contains(&Dot::I, EPS) && dinf.contains(&Dot::J, EPS));

        // 相切：x = 1 → (1:0:1) 重根
        let tan_l = Line::from_f64s(1.0, 0.0, -1.0);
        let dtan = unit.intersect_line(&tan_l);
        assert!(dtan.contains(&Dot::from_f64s(1.0, 0.0, 1.0), EPS));

        // 相离：y = 2 → ( ±i√3 : 2 : 1 )
        let far_l = Line::from_f64s(0.0, 1.0, -2.0);
        let dfar = unit.intersect_line(&far_l);
        assert!(unit.contains(&dfar.p(), EPS) && unit.contains(&dfar.q(), EPS));
        assert!(far_l.contains(&dfar.p(), EPS) && far_l.contains(&dfar.q(), EPS));

        // 迷向直线 x + iy = 0 与圆相交
        let iso = Line::new(Complex::ONE, Complex::I, Complex::ZERO);
        let diso = unit.intersect_line(&iso);
        assert!(unit.contains(&diso.p(), EPS) && unit.contains(&diso.q(), EPS));
        assert!(iso.contains(&diso.p(), EPS) && iso.contains(&diso.q(), EPS));
    }

    #[test]
    fn conic_dot_line_xline_conversions() {
        // Line <-> Conic
        let l = Line::from_f64s(2.0, -3.0, 5.0);
        let lc = l.as_conic();
        assert!(lc.is_line(EPS));
        assert!(lc.as_line().approx_eq(&l, EPS));

        // Dot <-> Conic (点圆)
        let p = Dot::from_f64s(3.0, -4.0, 2.0);
        let pc = p.as_conic();
        assert!(pc.is_dot(EPS));
        assert!(pc.as_dot().p.cross(p.p).norm() <= EPS);

        // XLine <-> Conic
        let l1 = Line::from_f64s(1.0, 2.0, -3.0);
        let l2 = Line::from_f64s(4.0, -1.0, 2.0);
        let xl = XLine::new(l1, l2);
        let xlc = xl.as_conic();
        assert!(xlc.is_xline(EPS));
        assert!(xlc.as_xline().approx_eq(&xl, EPS));
        assert!(xlc.as_dot().p.cross(xl.center().p).norm() <= EPS);

        // 去相位实导出
        let phased = Conic::UNIT_CIRCLE.scaled(Complex::I);
        let mat = phased.try_to_3x3f64(EPS).unwrap();
        assert!((mat[0][0] - 1.0).abs() < EPS);
        assert!((mat[2][2] + 1.0).abs() < EPS);

        let truly_complex = Conic::new(
            Complex::ONE,
            Complex::I,
            Complex::ONE,
            Complex::ZERO,
            Complex::ZERO,
            Complex::NEG_ONE,
        );
        assert!(truly_complex.try_to_3x3f64(EPS).is_none());
    }
}
