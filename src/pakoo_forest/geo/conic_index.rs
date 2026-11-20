use crate::pakoo_forest::geo::Conic;


use std::fmt;

use crate::pakoo_forest::calc::{Complex, Cov3C, Mat3C, Vec3C};
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;
//
//
/// 二次曲线参数化类别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConicParamKind {
    /// 非退化圆锥曲线（`r = 3`）：Veronese 映射 `s²·v₀ + st·v₁ + t²·v₂`
    NonSingular,
    /// 两条相异直线（`r = 2`）：`[-1, 1]` 走第一条线，`|t| > 1` 走第二条线
    TwoLines,
    /// 二重直线（`r = 1`）：线性射影映射 `s·p₀ + t·q₁`
    DoubleLine,
    /// 零曲线或含非有限分量
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ParamRepr {
    NonSingular {
        v0: Vec3C,
        v1: Vec3C,
        v2: Vec3C,
        inv: Mat3C,
    },
    TwoLines {
        p0: Vec3C,
        q1: Vec3C,
        q2: Vec3C,
        l1: Line,
        l2: Line,
    },
    DoubleLine {
        p0: Vec3C,
        q1: Vec3C,
        line: Line,
    },
    Invalid,
}

/// 缓存的二次曲线有理/分段参数化句柄。
///
/// 建议在需要多次采样（如绘制或连续追踪）时通过 [`Conic::parametrization`] 预计算。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConicParam {
    kind: ConicParamKind,
    repr: ParamRepr,
}

impl ConicParam {
    /// 参数化类别。
    #[inline]
    pub const fn kind(&self) -> ConicParamKind {
        self.kind
    }

    /// 当参数化为退化两直线（`r = 2`）时，返回分解得到的两条单位直线 `(l1, l2)`。
    #[inline]
    pub const fn lines(&self) -> Option<(Line, Line)> {
        match self.repr {
            ParamRepr::TwoLines { l1, l2, .. } => Some((l1, l2)),
            _ => None,
        }
    }

    /// 当参数化为二重直线（`r = 1`）时，返回该单位直线。
    #[inline]
    pub const fn double_line(&self) -> Option<Line> {
        match self.repr {
            ParamRepr::DoubleLine { line, .. } => Some(line),
            _ => None,
        }
    }

    /// 由齐次参数 `(s : t) ∈ ℂℙ¹` 计算曲线上的点（返回单位范数代表元；无效时返回零点）。
    #[must_use]
    pub fn point_at_uv(&self, s: Complex, t: Complex) -> Dot {
        let n = s.abs().hypot(t.abs());
        if n == 0.0 || !n.is_finite() {
            return Dot::from_vec3c(Vec3C::ZERO);
        }
        let s = s.div_f64(n);
        let t = t.div_f64(n);
        let v = match self.repr {
            ParamRepr::NonSingular { v0, v1, v2, .. } => {
                let ss = s * s;
                let st = s * t;
                let tt = t * t;
                v0.mul_complex(ss)
                    .add(v1.mul_complex(st))
                    .add(v2.mul_complex(tt))
            }
            ParamRepr::TwoLines { p0, q1, q2, .. } => {
                if s.abs() >= t.abs() {
                    let c_q = s * s - t * t;
                    let c_p = (s * t).mul_f64(2.0);
                    q1.mul_complex(c_q).add(p0.mul_complex(c_p))
                } else {
                    let c_q = t * t - s * s;
                    let c_p = (s * t).mul_f64(2.0);
                    q2.mul_complex(c_q).add(p0.mul_complex(c_p))
                }
            }
            ParamRepr::DoubleLine { p0, q1, .. } => p0.mul_complex(s).add(q1.mul_complex(t)),
            ParamRepr::Invalid => return Dot::from_vec3c(Vec3C::ZERO),
        };
        Dot::from_vec3c(v.normalize().unwrap_or(Vec3C::ZERO))
    }

    /// 由仿射/扩充复参数 `t ∈ ℂ ∪ {∞}` 计算曲线上的点。
    ///
    /// - 当 `t` 为无穷大时对应齐次参数 `(0 : 1)`；
    /// - 当 `|t| ≤ 1` 时采用 `(1 : t)`，当 `|t| > 1` 时自动切换至 `(1/t : 1)` 以防上溢。
    #[inline]
    #[must_use]
    pub fn point_at(&self, t: Complex) -> Dot {
        if t.is_nan() || matches!(self.repr, ParamRepr::Invalid) {
            return Dot::from_vec3c(Vec3C::ZERO);
        }
        if t.is_infinite() {
            return self.point_at_uv(Complex::ZERO, Complex::ONE);
        }
        if t.abs() <= 1.0 {
            self.point_at_uv(Complex::ONE, t)
        } else {
            self.point_at_uv(t.recip(), Complex::ONE)
        }
    }

    /// 由实参数 `t ∈ ℝ ∪ {±∞}` 计算曲线上的点。
    #[inline]
    #[must_use]
    pub fn point_at_f64(&self, t: f64) -> Dot {
        if t.is_nan() || matches!(self.repr, ParamRepr::Invalid) {
            return Dot::from_vec3c(Vec3C::ZERO);
        }
        if t.is_infinite() {
            return self.point_at_uv(Complex::ZERO, Complex::ONE);
        }
        if t.abs() <= 1.0 {
            self.point_at_uv(Complex::ONE, Complex::from_f64(t))
        } else {
            self.point_at_uv(Complex::from_f64(1.0 / t), Complex::ONE)
        }
    }

    /// 反求曲线上一点 `p` 对应的齐次参数 `(s : t)`（单位化）。
    ///
    /// 若 `p` 偏离曲线，自动正交投影至对应参数；曲线/点无效时返回 `None`。
    pub fn uv_of(&self, p: &Dot) -> Option<(Complex, Complex)> {
        if p.is_zero() || !p.is_finite() {
            return None;
        }
        let pn = p.p.normalize()?;
        match self.repr {
            ParamRepr::NonSingular { inv, .. } => {
                let u = inv.mul_vec(pn);
                let (s, t) = if u.v[0].abs() >= u.v[2].abs() {
                    if u.v[0].is_zero() && u.v[1].is_zero() {
                        (Complex::ZERO, Complex::ONE)
                    } else {
                        (u.v[0], u.v[1])
                    }
                } else {
                    (u.v[1], u.v[2])
                };
                normalize_uv(s, t)
            }
            ParamRepr::TwoLines { p0, q1, q2, l1, l2 } => {
                let d1 = l1.evaluate(&Dot::from_vec3c(pn)).abs();
                let d2 = l2.evaluate(&Dot::from_vec3c(pn)).abs();
                if d1 <= d2 {
                    let (alpha, beta) = gram2_coords(q1, p0, pn);
                    solve_two_line_uv(alpha, beta)
                } else {
                    let (alpha, beta) = gram2_coords(q2, p0, pn);
                    let (sl, tl) = solve_two_line_uv(alpha, beta)?;
                    Some((tl, sl))
                }
            }
            ParamRepr::DoubleLine { p0, q1, .. } => {
                // p0 与 q1 已经 Hermite 正交
                let s = p0.inner(pn);
                let t = q1.inner(pn);
                normalize_uv(s, t)
            }
            ParamRepr::Invalid => None,
        }
    }

    /// 反求曲线上一点 `p` 对应的标量参数 `t = t / s`。
    ///
    /// 当参数位于无穷远点 `(0 : 1)`（即 `s == 0`）或结果非有限时返回 `None`；
    /// 需要无奇点表示请改用 [`ConicParam::uv_of`]。
    #[inline]
    pub fn param_of(&self, p: &Dot) -> Option<Complex> {
        let (s, t) = self.uv_of(p)?;
        if s.is_zero() {
            return None;
        }
        let out = t / s;
        if out.is_finite() { Some(out) } else { None }
    }
}

#[inline]
fn normalize_uv(s: Complex, t: Complex) -> Option<(Complex, Complex)> {
    let n = s.abs().hypot(t.abs());
    if n == 0.0 || !n.is_finite() {
        return None;
    }
    Some((s.div_f64(n), t.div_f64(n)))
}

/// 在线性无关单位基 `(q, p0)` 上求 `pn ≈ α·q + β·p0` 的最小二乘齐次坐标 `(α, β)`。
#[inline]
fn gram2_coords(q: Vec3C, p0: Vec3C, pn: Vec3C) -> (Complex, Complex) {
    let g = q.inner(p0);
    let rq = q.inner(pn);
    let rp = p0.inner(pn);
    let alpha = rq - g * rp;
    let beta = rp - g.conj() * rq;
    (alpha, beta)
}

/// 求解 `(s² - t²):2st = α:β` 且满足 `|t| ≤ |s|` 的分支 `(s, t)`。
#[inline]
fn solve_two_line_uv(alpha: Complex, beta: Complex) -> Option<(Complex, Complex)> {
    if alpha.is_zero() && beta.is_zero() {
        return None;
    }
    let disc = (alpha * alpha + beta * beta).sqrt();
    let p_plus = alpha + disc;
    let p_minus = alpha - disc;
    let s = if p_plus.abs() >= p_minus.abs() {
        p_plus
    } else {
        p_minus
    };
    normalize_uv(s, beta)
}

#[inline]
fn diag_swap(
    a: &mut [[Complex; 3]; 3],
    m: &mut [[Complex; 3]; 3],
    m_inv: &mut [[Complex; 3]; 3],
    i: usize,
    j: usize,
) {
    if i == j {
        return;
    }
    a.swap(i, j);
    for row in a.iter_mut() {
        row.swap(i, j);
    }
    for row in m.iter_mut() {
        row.swap(i, j);
    }
    m_inv.swap(i, j);
}

/// 正交 45° 旋转 `R_{ij}(π/4)` 做对称合同 `A ← Rᵀ·A·R`, `M ← M·R`, `M⁻¹ ← Rᵀ·M⁻¹`。
#[inline]
fn diag_rot45(
    a: &mut [[Complex; 3]; 3],
    m: &mut [[Complex; 3]; 3],
    m_inv: &mut [[Complex; 3]; 3],
    i: usize,
    j: usize,
) {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    for k in 0..3 {
        let (x, y) = (a[i][k], a[j][k]);
        a[i][k] = (x + y).mul_f64(c);
        a[j][k] = (y - x).mul_f64(c);
    }
    for row in a.iter_mut() {
        let (x, y) = (row[i], row[j]);
        row[i] = (x + y).mul_f64(c);
        row[j] = (y - x).mul_f64(c);
    }
    for row in m.iter_mut() {
        let (x, y) = (row[i], row[j]);
        row[i] = (x + y).mul_f64(c);
        row[j] = (y - x).mul_f64(c);
    }
    for k in 0..3 {
        let (x, y) = (m_inv[i][k], m_inv[j][k]);
        m_inv[i][k] = (x + y).mul_f64(c);
        m_inv[j][k] = (y - x).mul_f64(c);
    }
}

#[inline]
fn line_basis_pair(line: Line) -> Option<(Vec3C, Vec3C)> {
    let (a, b, c) = (line.x(), line.y(), line.z());
    let ab = a.abs().hypot(b.abs());
    if ab <= 1e-12 {
        // 无穷远直线
        return Some((Vec3C::E0, Vec3C::E1));
    }
    let q1 = Vec3C::new(b, -a, Complex::ZERO).normalize()?;
    let p0 = Vec3C::new(-a.conj() * c, -b.conj() * c, Complex::from_f64(ab * ab)).normalize()?;
    Some((p0, q1))
}

#[inline]
fn phase_clean_real_diag(d: &mut [Complex; 3], strict_real: bool) -> bool {
    if strict_real {
        return true;
    }
    if d[0].is_zero() {
        return false;
    }
    let u = d[0].div_f64(d[0].abs());
    let c0 = Complex::from_f64(d[0].abs());
    let c1 = d[1] / u;
    let c2 = d[2] / u;
    let real1 = c1.im.abs() <= 1e-14 * c1.abs();
    let real2 = c2.im.abs() <= 1e-14 * c2.abs();
    if real1 && real2 {
        d[0] = c0;
        d[1] = Complex::from_f64(c1.re);
        d[2] = Complex::from_f64(c2.re);
        true
    } else {
        false
    }
}
impl Conic {
    // ------------------------------------------------------------------
    // 连续参数化 / 索引
    // ------------------------------------------------------------------

    /// 构造连续有理/分段参数化句柄。
    ///
    /// - `r = 3`（非退化）：对称合同对角化 + Veronese 映射拉回；
    /// - `r = 2`（两条相异直线）：连续分段覆盖 `(-∞,-1), (-1,0), (0,1), (1,+∞)`；
    /// - `r = 1`（二重直线）：线性射影覆盖；
    /// - `r = 0` 或非有限：返回 `ConicParamKind::Invalid`。
    pub fn parametrization(&self, eps: f64) -> ConicParam {
        const INVALID: ConicParam = ConicParam {
            kind: ConicParamKind::Invalid,
            repr: ParamRepr::Invalid,
        };
        if !self.is_finite() {
            return INVALID;
        }
        let qn = match self.normalize() {
            Some(n) => n,
            None => return INVALID,
        };
        let rank = self.rank(eps);
        if rank == 0 {
            return INVALID;
        }

        // 防御性对称化（若外部直接修改过 pub q）
        let mut a = qn.q.add(&qn.q.transpose()).mul_f64(0.5).m;
        let mut m = Mat3C::IDENTITY.m;
        let mut m_inv = Mat3C::IDENTITY.m;

        // ---- 第 1 步合同消除 ----
        let mut pd = 0;
        let mut md = a[0][0].abs();
        for i in 1..3 {
            let v = a[i][i].abs();
            if v > md {
                md = v;
                pd = i;
            }
        }
        let off_pairs = [(0usize, 1usize), (0, 2), (1, 2)];
        let (mut pi, mut pj) = (0, 1);
        let mut mo = a[0][1].abs();
        for &(i, j) in &off_pairs[1..] {
            let v = a[i][j].abs();
            if v > mo {
                mo = v;
                pi = i;
                pj = j;
            }
        }
        if md == 0.0 && mo == 0.0 {
            return INVALID;
        }
        if md >= 0.5 * mo {
            diag_swap(&mut a, &mut m, &mut m_inv, 0, pd);
        } else {
            match (pi, pj) {
                (0, 1) => {}
                (0, 2) => diag_swap(&mut a, &mut m, &mut m_inv, 1, 2),
                _ => {
                    diag_swap(&mut a, &mut m, &mut m_inv, 0, 1);
                    diag_swap(&mut a, &mut m, &mut m_inv, 1, 2);
                }
            }
            diag_rot45(&mut a, &mut m, &mut m_inv, 0, 1);
        }

        let d0 = a[0][0];
        if d0.is_zero() || !d0.is_finite() {
            return INVALID;
        }
        let l1 = -a[0][1] / d0;
        let l2 = -a[0][2] / d0;
        a[1][1] += l1 * a[0][1];
        a[1][2] += l1 * a[0][2];
        a[2][1] = a[1][2];
        a[2][2] += l2 * a[0][2];
        for r in 0..3 {
            m[r][1] += l1 * m[r][0];
            m[r][2] += l2 * m[r][0];
        }
        for k in 0..3 {
            m_inv[0][k] -= l1 * m_inv[1][k] + l2 * m_inv[2][k];
        }

        let make_double_line = |m_inv: &[[Complex; 3]; 3]| -> ConicParam {
            let line = match Line::from_cov3c(Mat3C::new(*m_inv).row_cov(0)).normalize() {
                Some(l) => l,
                None => return INVALID,
            };
            match line_basis_pair(line) {
                Some((p0, q1)) => ConicParam {
                    kind: ConicParamKind::DoubleLine,
                    repr: ParamRepr::DoubleLine { p0, q1, line },
                },
                None => INVALID,
            }
        };

        if rank == 1 {
            return make_double_line(&m_inv);
        }

        // ---- 第 2 步合同消除（右下 2×2 块） ----
        let md2 = a[1][1].abs().max(a[2][2].abs());
        let mo2 = a[1][2].abs();
        if md2 == 0.0 && mo2 == 0.0 {
            return make_double_line(&m_inv);
        }
        if md2 >= 0.5 * mo2 {
            if a[2][2].abs() > a[1][1].abs() {
                diag_swap(&mut a, &mut m, &mut m_inv, 1, 2);
            }
        } else {
            diag_rot45(&mut a, &mut m, &mut m_inv, 1, 2);
        }

        let d1 = a[1][1];
        if d1.is_zero() || !d1.is_finite() {
            return make_double_line(&m_inv);
        }
        let l12 = -a[1][2] / d1;
        let d2 = a[2][2] + l12 * a[1][2];
        for r in 0..3 {
            m[r][2] += l12 * m[r][1];
        }
        for k in 0..3 {
            m_inv[1][k] -= l12 * m_inv[2][k];
        }

        let strict_real = self.is_real();
        let make_two_lines =
            |d0: Complex,
             d1: Complex,
             m: &[[Complex; 3]; 3],
             m_inv: &[[Complex; 3]; 3]|
             -> ConicParam {
                let mut d_pair = [d0, d1, Complex::ZERO];
                let is_real_pair = phase_clean_real_diag(&mut d_pair, strict_real);
                let (a0, b0) = if is_real_pair && d_pair[0].re * d_pair[1].re < 0.0 {
                    if d_pair[0].re < 0.0 {
                        ((-d_pair[0]).sqrt(), d_pair[1].sqrt())
                    } else {
                        (d_pair[0].sqrt(), (-d_pair[1]).sqrt())
                    }
                } else {
                    (d_pair[0].sqrt(), Complex::I * d_pair[1].sqrt())
                };
                let mi = Mat3C::new(*m_inv);
                let r0 = mi.row_cov(0);
                let r1 = mi.row_cov(1);
                let line1 = match Line::from_cov3c(r0.mul_complex(a0).add(r1.mul_complex(b0)))
                    .normalize()
                {
                    Some(l) => l,
                    None => return INVALID,
                };
                let line2 = match Line::from_cov3c(r0.mul_complex(a0).sub(r1.mul_complex(b0)))
                    .normalize()
                {
                    Some(l) => l,
                    None => return INVALID,
                };
                let mut p0 = match Mat3C::new(*m).col_vec(2).normalize() {
                    Some(v) => v,
                    None => return INVALID,
                };
                if p0.v[2].re < 0.0 {
                    p0 = p0.neg();
                }

                let (q1, q2) = if p0.v[2].abs() > 1e-6 {
                    // 交点在有限处：取各自无穷远点，使 (-∞,-1), (-1,0), (0,1), (1,+∞) 恰好为四条欧氏射线
                    let q1 = match line1.inf_point().p.normalize() {
                        Some(v) => v,
                        None => return INVALID,
                    };
                    let q2 = match line2.inf_point().p.normalize() {
                        Some(v) => v,
                        None => return INVALID,
                    };
                    (q1, q2)
                } else {
                    // 平行或近平行：取 Hermite 正交锚点，避开无穷远处的奇异抵消
                    let p0_conj = p0.conj();
                    let l1_col = Vec3C::from_array(line1.to_array());
                    let l2_col = Vec3C::from_array(line2.to_array());
                    let q1 = match p0_conj.cross(l1_col).normalize() {
                        Some(v) => v,
                        None => return INVALID,
                    };
                    let q2 = match p0_conj.cross(l2_col).normalize() {
                        Some(v) => v,
                        None => return INVALID,
                    };
                    (q1, q2)
                };

                ConicParam {
                    kind: ConicParamKind::TwoLines,
                    repr: ParamRepr::TwoLines {
                        p0,
                        q1,
                        q2,
                        l1: line1,
                        l2: line2,
                    },
                }
            };

        if rank == 2 || d2.is_zero() || !d2.is_finite() {
            return make_two_lines(d0, d1, &m, &m_inv);
        }

        // ---- r = 3：符号规约并拉回 Veronese 映射 ----
        let mut d = [d0, d1, d2];
        if phase_clean_real_diag(&mut d, strict_real) {
            let neg_count = d.iter().filter(|x| x.re < 0.0).count();
            if neg_count >= 2 {
                for x in &mut d {
                    *x = -*x;
                }
            }
            if let Some(neg_idx) = d.iter().position(|x| x.re < 0.0) {
                if neg_idx != 2 {
                    d.swap(neg_idx, 2);
                    for row in m.iter_mut() {
                        row.swap(neg_idx, 2);
                    }
                    m_inv.swap(neg_idx, 2);
                }
            }
        }

        let s0 = d[0].sqrt();
        let s1 = d[1].sqrt();
        let s2 = (-d[2]).sqrt();
        if s0.is_zero() || s1.is_zero() || s2.is_zero() {
            return make_two_lines(d[0], d[1], &m, &m_inv);
        }

        let m_mat = Mat3C::new(m);
        let m_inv_mat = Mat3C::new(m_inv);
        let m0 = m_mat.col_vec(0);
        let m1 = m_mat.col_vec(1);
        let m2 = m_mat.col_vec(2);

        let inv_2s0 = (s0.mul_f64(2.0)).recip();
        let inv_2s2 = (s2.mul_f64(2.0)).recip();
        let v0 = m2.mul_complex(inv_2s2).sub(m0.mul_complex(inv_2s0));
        let v1 = m1.div_complex(s1);
        let v2 = m2.mul_complex(inv_2s2).add(m0.mul_complex(inv_2s0));

        let r0 = m_inv_mat.row_cov(0);
        let r1 = m_inv_mat.row_cov(1);
        let r2 = m_inv_mat.row_cov(2);
        let w0 = r2.mul_complex(s2).sub(r0.mul_complex(s0));
        let w1 = r1.mul_complex(s1);
        let w2 = r2.mul_complex(s2).add(r0.mul_complex(s0));

        let max_v = v0.norm().max(v1.norm()).max(v2.norm());
        if max_v == 0.0 || !max_v.is_finite() {
            return INVALID;
        }
        let v0 = v0.div_f64(max_v);
        let v1 = v1.div_f64(max_v);
        let v2 = v2.div_f64(max_v);
        let inv = Mat3C::from_3cov(w0, w1, w2).mul_f64(max_v);

        ConicParam {
            kind: ConicParamKind::NonSingular,
            repr: ParamRepr::NonSingular { v0, v1, v2, inv },
        }
    }

    /// 按齐次参数 `(s : t)` 取曲线上的点（一次性调用）。
    #[inline]
    #[must_use]
    pub fn point_at_uv(&self, s: Complex, t: Complex, eps: f64) -> Dot {
        self.parametrization(eps).point_at_uv(s, t)
    }

    /// 按扩充复参数 `t` 取曲线上的点（一次性调用）。
    #[inline]
    #[must_use]
    pub fn point_at(&self, t: Complex, eps: f64) -> Dot {
        self.parametrization(eps).point_at(t)
    }

    /// 按实参数 `t` 取曲线上的点（一次性调用）。
    #[inline]
    #[must_use]
    pub fn point_at_f64(&self, t: f64, eps: f64) -> Dot {
        self.parametrization(eps).point_at_f64(t)
    }

    /// 反求曲线上一点 `p` 对应的标量参数 `t`（一次性调用）。
    #[inline]
    pub fn param_of(&self, p: &Dot, eps: f64) -> Option<Complex> {
        self.parametrization(eps).param_of(p)
    }
}