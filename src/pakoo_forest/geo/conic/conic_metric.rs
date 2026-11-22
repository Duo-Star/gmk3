// src/pakoo_forest/geo/conic/conic_metric.rs
//! 通用二次曲线度量与射影构造/提取模块（统一覆盖椭圆、双曲线、抛物线与复曲线）。
//!
//! # 核心思想
//!
//! 在 `ℂℙ²` 中引入无穷远直线 `L∞ = (0:0:1)` 与虚圆点对偶退化矩阵
//! `C∞* = diag(1, 1, 0)` 后，椭圆、双曲线、抛物线不再需要拆分为三套代码，
//! 而是通过同一套全纯齐次代数/对偶束公式统一处理：
//!
//! - **五点 / 五切线定曲线**：纯 3×3 铅笔线性组合 + 伴随，无需 6×6 线性方程组；
//! - **焦点-准线-离心率**：`ab²·focus.as_conic() − e²·zf²·directrix.as_conic()`；
//! - **渐近线 + 一点**：`P_z²·asy.as_conic() − (Pᵀ Q_asy P)·L∞²`，统一覆盖实双曲线与虚渐近线椭圆；
//! - **主轴**：`B·dx² − (A − C)·dx·dy − B·dy² = 0` 解出正交共轭方向再取极线，中心/抛物线通用；
//! - **焦点与准线**：求解对偶束 `det(Q* − λ·C∞*) = 0` 的齐次根后复用 `decompose_lines` 拆分。
///////////////////////////////////////////////////////////////////////////////////////
//!
// #### 1. 这些构造在复数域上都自然吗？复杂吗？
// **完全自然、几乎零额外复杂度，甚至比实数域还简单。** 原因在于我们不用数值分解或坐标系旋转，全部走齐次二次型/线性铅笔（Pencil）代数：
//
// 1. **五点定圆锥曲线（`conic_from_5_dots`）**：
// 不需要解 $6\times 6$ 线性方程组！取前四点组成两条退化直线对 $Q_1 = (P_1 P_2)\cup(P_3 P_4)$ 与 $Q_2 = (P_1 P_3)\cup(P_2 P_4)$，任意过这四点的曲线都在铅笔 $\lambda Q_1 + \mu Q_2$ 中，代入第五点立即得出：
// $$Q = (P_5^T Q_2 P_5)\,Q_1 - (P_5^T Q_1 P_5)\,Q_2$$
// 仅需 4 次叉积 + 2 次外积 + 2 次求值，全纯复数精确成立，且若 3 点共线会自动退化为该直线与另两点连线的并。
// 2. **五切线定圆锥曲线（`conic_from_5_lines`）**：
// **完全可行，而且几乎零代码成本！** 根据射影对偶性，把 5 条 `Line` 当作对偶平面上的 5 个“点”套用完全相同的铅笔公式得到对偶矩阵 $Q^*$，再取一次伴随矩阵 $Q = \text{adj}(Q^*)$ 即得原切线二次曲线（不需要求逆与除行列式）。
// 3. **已知渐近线 `XLine` + 曲线上一点 `Dot`**：
// 非常自然，而且**只用 1 行铅笔公式**：
// 实交叉直线对应双曲线，共轭复交叉直线对应椭圆，一般复交叉直线对应一般复横截二次曲线——它们全都在由 `asy.as_conic()` 与二重无穷远直线 $z^2 = 0$ 张成的线性铅笔中：
// $$Q = P_z^2\, Q_{\text{asy}} - (P^T Q_{\text{asy}} P)\, (L_\infty^T L_\infty)$$
// 4. **中心 + 两个共轭直径代表点**：
// **你的直觉完全正确，必须区分椭圆型（$+1$）和双曲线型（$-1$）**（或者传一个复系数 $s \in \mathbb{C}$）。原因在于：若中心为 $O$、两共轭半直径向量为 $u, v$，在仿射基 $\{O; u, v\}$ 下，椭圆方程是 $\xi^2 + \eta^2 = 1$（$P_1, P_2$ 都在曲线上），而双曲线方程是 $\xi^2 - \eta^2 = 1$（$P_1$ 在本曲线上，$P_2$ 在**共轭双曲线**上，即 $O + i v$ 才在原复双曲线上）。用齐次基矩阵 $M$ 和伴随变换 $Q = \text{adj}(M)^T \text{diag}(1, s, -1)\, \text{adj}(M)$ 即可无除法统一覆盖 $s = +1$（椭圆）、$s = -1$（双曲线）乃至任意复 $s$。
// 5. **两焦点 + 曲线上一点**：
// 同样**必须区分椭圆与双曲线**（在欧氏几何与复几何中，过同一点 $P$ 且共焦 $F_1, F_2$ 的曲线恰好有一条椭圆和一条双曲线，它们互为正交共焦族！）。
// 并且不管用 `Ddot` 还是独立两点 `(f1, f2)`，公式对 $F_1, F_2$ 完全对称无序：用复距离算出 $\lambda = \frac{1}{4}\big((d_1 \pm d_2)^2 - d_{12}^2\big)$，代入对偶共焦铅笔 $Q^*(\lambda) = -\frac{1}{2}(F_1 F_2^T + F_2 F_1^T) + \lambda z_1 z_2 C_\infty^*$ 再取伴随矩阵，直接得精确曲线（包括两焦点为共轭虚点时也能反向重构出椭圆，两焦点重合时退化为圆！）。
//
// #### 2. 抛物线的焦点到底是什么模型？
// 在复射影几何里，任意圆锥曲线的焦点是**过两虚圆点 $I, J$ 作曲线的四条切线的两两交点**。
// 对于**抛物线**，它与无穷远直线 $L_\infty$ 相切，而 $L_\infty$ 本身恰好同时穿过 $I$ 和 $J$！因此 $L_\infty$ 同时占据了从 $I$ 出发和从 $J$ 出发的一条切线，剩下的两条切线交于唯一**有限焦点 $F$**。
//
// 而在我们的对偶铅笔 $\det(Q^* - \lambda C_\infty^*) = 0$ 中（其中 $C_\infty^* = \text{diag}(1, 1, 0)$），抛物线 $\Delta_\infty = 0$ 使 $\lambda^2$ 系数为零，两对焦点自然退化为：
// - **第一对（主焦点对 `Ddot`）**：`{F, P_inf}` —— 有限焦点 $F$ 与抛物线对称轴在无穷远处的切点 $P_\infty$（**恰好也是抛物线与 $L_\infty$ 的唯一交点**，而且 `{F, P_inf}.line()` 直接就是抛物线的对称轴！）；
// - **第二对（虚焦点对 `Ddot`）**：`Ddot::CYCLIC = {I, J}`。
//
// 所以用统一对偶束无需任何特判，抛物线自动给出 `{F, P_inf}` 与 `{I, J}`；而由“焦点 + 准线”构造抛物线时，只需传单点 `F`（本质是离心率 $e = 1$ 的焦点-准线构造）。
//
// #### 3. 为什么“每个主直径只交一点的 `Ddot`”必然跳变？
// 这是个纯拓扑障碍（$\pi$-单值性 / spin-$1/2$ 截面问题）：当椭圆连续旋转 $90^\circ$ 或退化经过圆时，无状态的纯函数 `Conic -> Ddot` 在 $\mathbb{RP}^1$ 上无法做到全局无跳变地从每根主轴挑出“唯一一侧”的顶点。因此像你判断的那样，内核保留无对称破缺的 `conic_axes(&self) -> XLine` 和 `conic_vertices(&self) -> Qdot`（每条主直径各取一个 `Ddot`，共 4 个顶点组成 `Qdot`），把状态连续性（按上一帧就近追踪）留给上层 UI（如果需要的话），性能与数学干净度最优。


use crate::pakoo_forest::calc::{Complex, Mat3C, Vec3C};
use crate::pakoo_forest::geo::conic::Conic;
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::d_x_line::DXLine;
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;
use crate::pakoo_forest::geo::q_dot::Qdot;
use crate::pakoo_forest::geo::x_line::XLine;

/// 虚圆点对 `{I, J}` 的对偶退化二次曲线矩阵 `C∞* = ½(I·Jᵀ + J·Iᵀ) = diag(1, 1, 0)`。
const C_INF_DUAL: Mat3C = Mat3C::new([
    [Complex::ONE, Complex::ZERO, Complex::ZERO],
    [Complex::ZERO, Complex::ONE, Complex::ZERO],
    [Complex::ZERO, Complex::ZERO, Complex::ZERO],
]);

/// 无穷远二重直线矩阵 `L∞ᵀ·L∞ = diag(0, 0, 1)`。
const L_INF_SQR: Mat3C = Mat3C::new([
    [Complex::ZERO, Complex::ZERO, Complex::ZERO],
    [Complex::ZERO, Complex::ZERO, Complex::ZERO],
    [Complex::ZERO, Complex::ZERO, Complex::ONE],
]);

#[inline]
fn dot_to_line_coords(p: &Dot) -> Line {
    Line::new(p.x(), p.y(), p.z())
}

#[inline]
fn line_to_dot_coords(l: &Line) -> Dot {
    Dot::new(l.x(), l.y(), l.z())
}

/// 求解二次方程 `c2·λ² + c1·λ + c0 = 0` 的两个齐次根 `(λ_num, λ_den)`，
/// 满足 `λ_den·M − λ_num·C∞*` 对 `c2 → 0`（抛物线）平滑无除零。
#[inline]
fn solve_quadratic_homogeneous(c2: Complex, c1: Complex, c0: Complex) -> ((Complex, Complex), (Complex, Complex)) {
    let disc = (c1 * c1 - c2 * c0.mul_f64(4.0)).sqrt();
    let s = if (c1.conj() * disc).re >= 0.0 {
        c1 + disc
    } else {
        c1 - disc
    };
    if s.is_zero() {
        // c1 == 0 且 c2*c0 == 0
        if c2.is_zero() {
            return ((Complex::ONE, Complex::ZERO), (Complex::ONE, Complex::ZERO));
        }
        let r = (-c0 / c2).sqrt();
        return ((r, Complex::ONE), (-r, Complex::ONE));
    }
    ((c0.mul_f64(2.0), -s), (-s, c2.mul_f64(2.0)))
}

/// 用对偶束法从 `foci = {F₁, F₂}` 与 `λ` 重构原二次曲线：
/// `Q*(λ) = −½(F₁·F₂ᵀ + F₂·F₁ᵀ) + λ·z₁z₂·C∞*`, `Q = adj(Q*)`。
#[inline]
fn conic_from_confocal_lambda(f1: Dot, f2: Dot, lambda: Complex) -> Conic {
    let f1n = f1.normalize().unwrap_or(f1);
    let f2n = f2.normalize().unwrap_or(f2);
    let outer = Mat3C::from_outer(f1n.p, f2n.p.transpose());
    let s_mat = outer.add(&outer.transpose()).mul_f64(0.5);
    let z12 = f1n.z() * f2n.z();
    let q_dual = C_INF_DUAL.mul_complex(lambda * z12).sub(&s_mat);
    let q = Conic {
        q: q_dual.adjugate(),
    };
    q.normalize().unwrap_or(q)
}

impl Conic {
    // ------------------------------------------------------------------
    // 通用构造器
    // ------------------------------------------------------------------

    /// 五点定二次曲线（铅笔闭式构造，O(1) 无线性方程组求解）。
    ///
    /// 令 `Q₁ = (P₀P₁)∪(P₂P₃)`, `Q₂ = (P₀P₂)∪(P₁P₃)`，
    /// 则 `Q = (P₄ᵀ Q₂ P₄)·Q₁ − (P₄ᵀ Q₁ P₄)·Q₂`。
    /// 存在四点共线或点重合时退化或返回 `Conic::ZERO`。
    #[must_use]
    pub fn conic_from_5_dots(p0: Dot, p1: Dot, p2: Dot, p3: Dot, p4: Dot) -> Conic {
        let n0 = p0.normalize().unwrap_or(p0);
        let n1 = p1.normalize().unwrap_or(p1);
        let n2 = p2.normalize().unwrap_or(p2);
        let n3 = p3.normalize().unwrap_or(p3);
        let n4 = p4.normalize().unwrap_or(p4);

        let q1 = XLine::new(n0.join(n1), n2.join(n3)).as_conic();
        let q2 = XLine::new(n0.join(n2), n1.join(n3)).as_conic();

        let e1 = q1.evaluate(&n4);
        let e2 = q2.evaluate(&n4);
        let out = Conic {
            q: q1.q.mul_complex(e2).sub(&q2.q.mul_complex(e1)),
        };
        out.normalize().unwrap_or(out)
    }

    /// 五切线定二次曲线（严格射影对偶：先在对偶平面求过 5 条线的 `Q*`，再取 `adj(Q*)`）。
    #[must_use]
    pub fn conic_from_5_lines(l0: Line, l1: Line, l2: Line, l3: Line, l4: Line) -> Conic {
        let dual = Conic::conic_from_5_dots(
            line_to_dot_coords(&l0),
            line_to_dot_coords(&l1),
            line_to_dot_coords(&l2),
            line_to_dot_coords(&l3),
            line_to_dot_coords(&l4),
        );
        let out = Conic {
            q: dual.q.adjugate(),
        };
        out.normalize().unwrap_or(out)
    }

    /// 焦点 + 准线 + 复离心率 `ecc` 构造圆锥曲线：
    /// `(a² + b²)·[(x·z_f − x_f·z)² + (y·z_f − y_f·z)²] − e²·z_f²·(ax + by + cz)² = 0`。
    ///
    /// - `e = 0`：点圆；
    /// - `0 < e < 1`（实）：椭圆；
    /// - `e = 1`：抛物线；
    /// - `e > 1`（实）：双曲线；
    /// - 任意复数 `e`：一般复圆锥曲线。
    #[inline]
    #[must_use]
    pub fn conic_from_focus_directrix_ecc(focus: Dot, directrix: Line, ecc: Complex) -> Conic {
        let fn_ = focus.normalize().unwrap_or(focus);
        let ln = directrix.normalize().unwrap_or(directrix);
        let ab2 = ln.x() * ln.x() + ln.y() * ln.y();
        let zf2 = fn_.z() * fn_.z();
        let q_pt = fn_.as_conic();
        let q_ln = ln.as_conic();
        let out = Conic {
            q: q_pt.q.mul_complex(ab2).sub(&q_ln.q.mul_complex(ecc * ecc * zf2)),
        };
        out.normalize().unwrap_or(out)
    }

    /// 焦点 + 准线 + 实离心率 `ecc` 构造圆锥曲线。
    #[inline]
    #[must_use]
    pub fn conic_from_focus_directrix_ecc_f64(focus: Dot, directrix: Line, ecc: f64) -> Conic {
        Conic::conic_from_focus_directrix_ecc(focus, directrix, Complex::from_f64(ecc))
    }

    /// 焦点 + 准线构造抛物线（`e = 1`）。
    #[inline]
    #[must_use]
    pub fn conic_from_parabola_focus_directrix(focus: Dot, directrix: Line) -> Conic {
        Conic::conic_from_focus_directrix_ecc(focus, directrix, Complex::ONE)
    }

    /// 中心 + 两个共轭直径代表点 `p1, p2` + 类型参数 `s` 构造中心圆锥曲线。
    ///
    /// 在以 `center` 为原点、`p1 − center` 和 `p2 − center` 为基的仿射坐标系下，
    /// 曲线方程为 `ξ² + s·η² = 1`：
    /// - `s = +1`：椭圆（`p1, p2` 均在曲线上）；
    /// - `s = −1`：双曲线（`p1` 在曲线上，`p2` 在其共轭双曲线上，即 `center + i·(p2 − center)` 在原复曲线上）。
    #[must_use]
    pub fn conic_from_center_conjugate(center: Dot, p1: Dot, p2: Dot, s: Complex) -> Conic {
        let c = center.normalize().unwrap_or(center);
        let q1 = p1.normalize().unwrap_or(p1);
        let q2 = p2.normalize().unwrap_or(p2);
        let (zc, z1, z2) = (c.z(), q1.z(), q2.z());

        let u = q1.p.mul_complex(zc).sub(c.p.mul_complex(z1)).mul_complex(z2);
        let v = q2.p.mul_complex(zc).sub(c.p.mul_complex(z2)).mul_complex(z1);
        let o = c.p.mul_complex(z1 * z2);

        let m = Mat3C::from_3vec(u, v, o);
        let adj = m.adjugate();
        let d = Mat3C::from_diag([Complex::ONE, s, Complex::NEG_ONE]);
        let out = Conic {
            q: adj.transpose().mul(&d).mul(&adj),
        };
        out.normalize().unwrap_or(out)
    }

    /// 中心 + 两共轭半直径端点构造椭圆（`p1, p2` 均在椭圆上）。
    #[inline]
    #[must_use]
    pub fn conic_from_center_conjugate_ellipse(center: Dot, p1: Dot, p2: Dot) -> Conic {
        Conic::conic_from_center_conjugate(center, p1, p2, Complex::ONE)
    }

    /// 中心 + 两共轭半直径端点构造双曲线（`p1` 在主双曲线上，`p2` 在共轭双曲线上）。
    #[inline]
    #[must_use]
    pub fn conic_from_center_conjugate_hyperbola(center: Dot, p1: Dot, p2: Dot) -> Conic {
        Conic::conic_from_center_conjugate(center, p1, p2, Complex::NEG_ONE)
    }

    /// 焦点骈点 `foci = {F₁, F₂}` + 曲线上一点 `p` 构造椭圆（支持复焦点与复点）。
    #[inline]
    #[must_use]
    pub fn conic_from_foci_point_ellipse(foci: Ddot, p: Dot) -> Conic {
        Conic::conic_from_2_foci_point_ellipse(foci.p(), foci.q(), p)
    }

    /// 焦点骈点 `foci = {F₁, F₂}` + 曲线上一点 `p` 构造双曲线（支持复焦点与复点）。
    #[inline]
    #[must_use]
    pub fn conic_from_foci_point_hyperbola(foci: Ddot, p: Dot) -> Conic {
        Conic::conic_from_2_foci_point_hyperbola(foci.p(), foci.q(), p)
    }

    /// 两个焦点 `f1, f2` + 曲线上一点 `p` 构造椭圆。
    #[must_use]
    pub fn conic_from_2_foci_point_ellipse(f1: Dot, f2: Dot, p: Dot) -> Conic {
        let d1 = f1.distance_complex(&p);
        let d2 = f2.distance_complex(&p);
        let d12_sqr = f1.distance_sqr_complex(&f2);
        let sum = d1 + d2;
        let lambda = (sum * sum - d12_sqr).mul_f64(0.25);
        conic_from_confocal_lambda(f1, f2, lambda)
    }

    /// 两个焦点 `f1, f2` + 曲线上一点 `p` 构造双曲线。
    #[must_use]
    pub fn conic_from_2_foci_point_hyperbola(f1: Dot, f2: Dot, p: Dot) -> Conic {
        let d1 = f1.distance_complex(&p);
        let d2 = f2.distance_complex(&p);
        let d12_sqr = f1.distance_sqr_complex(&f2);
        let diff = d1 - d2;
        let lambda = (diff * diff - d12_sqr).mul_f64(0.25);
        conic_from_confocal_lambda(f1, f2, lambda)
    }

    /// 渐近线 `asy`（实两线 → 双曲线；共轭复两线 → 椭圆）+ 曲线上一点 `p` 构造圆锥曲线：
    /// `Q = P_z²·Q_asy − (Pᵀ Q_asy P)·L∞²`。
    #[inline]
    #[must_use]
    pub fn conic_from_asymptotes_dot(asy: XLine, p: Dot) -> Conic {
        let q_asy = asy.as_conic();
        let q_asy = q_asy.normalize().unwrap_or(q_asy);
        let pn = p.normalize().unwrap_or(p);
        let pz2 = pn.z() * pn.z();
        let eval = q_asy.evaluate(&pn);
        let out = Conic {
            q: q_asy.q.mul_complex(pz2).sub(&L_INF_SQR.mul_complex(eval)),
        };
        out.normalize().unwrap_or(out)
    }

    // ------------------------------------------------------------------
    // 几何提取与性质
    // ------------------------------------------------------------------

    /// 曲线中心（即无穷远直线 `Line::INF` 的极点 `Q* · L∞`）。
    ///
    /// - 椭圆/双曲线/圆：返回有限仿射中心；
    /// - 抛物线：返回其与 `Line::INF` 的切点（轴方向的无穷远点）。
    #[inline]
    #[must_use]
    pub fn conic_center(&self) -> Dot {
        let v = self.q.adjugate().mul_vec(Vec3C::E2);
        Dot::from_vec3c(v.normalize().unwrap_or(v))
    }

    /// 曲线与无穷远直线 `Line::INF` 的两个交点（骈点）。
    #[inline]
    #[must_use]
    pub fn conic_inf_dots(&self) -> Ddot {
        self.intersect_line(&Line::INF)
    }

    /// 曲线的渐近线对（双曲线给出两实渐近线；椭圆给出共轭复渐近线；抛物线重合为 `Line::INF`）。
    #[inline]
    #[must_use]
    pub fn conic_asymptotes(&self) -> XLine {
        let d_inf = self.conic_inf_dots();
        XLine::new(self.polar(&d_inf.p()), self.polar(&d_inf.q()))
    }

    /// 过任意点 `p` 向曲线作的两条切线（退化切线对矩阵 `(Pᵀ Q P)·Q − (QP)(QP)ᵀ` 的直线分解）。
    ///
    /// - `p` 在曲线外（实）：两条实切线；
    /// - `p` 在曲线内（实）：两条共轭复切线；
    /// - `p` 在曲线上：该点切线的二重线。
    #[inline]
    #[must_use]
    pub fn conic_tangents_from(&self, p: &Dot) -> XLine {
        let qn = self.normalize().unwrap_or(*self);
        let pn = p.normalize().unwrap_or(*p);
        let qp = qn.q.mul_vec(pn.p);
        let pqp = pn.p.transpose().dot(qp);
        let deg = Conic {
            q: qn.q.mul_complex(pqp).sub(&Mat3C::from_outer(qp, qp.transpose())),
        };
        deg.as_xline()
    }

    /// 方向平行于直线 `l` 的共轭直径（即 `l` 的无穷远点关于曲线的极线）。
    #[inline]
    #[must_use]
    pub fn conic_conjugate_diameter(&self, l: &Line) -> Line {
        self.polar(&l.inf_point())
    }

    /// 两条直线 `l1, l2` 的方向是否关于当前曲线互相共轭（`d₁ᵀ·Q·d₂ = 0`）。
    #[inline]
    pub fn conic_are_conjugate_directions(&self, l1: &Line, l2: &Line, eps: f64) -> bool {
        let qn = self.normalize().unwrap_or(*self);
        let d1 = l1.inf_point().p.normalize().unwrap_or(l1.inf_point().p);
        let d2 = l2.inf_point().p.normalize().unwrap_or(l2.inf_point().p);
        let val = d1.transpose().dot(qn.q.mul_vec(d2));
        val.abs() <= eps * (1.0 + qn.norm())
    }

    /// 与当前中心二次曲线共渐近线的共轭双曲线（不改渐近线、翻转常数项符号）：
    /// `Q_conj = O_z²·Q − 2·(Oᵀ Q O)·L∞²`。
    #[inline]
    #[must_use]
    pub fn conic_conjugate_hyperbola(&self) -> Conic {
        let qn = self.normalize().unwrap_or(*self);
        let o = qn.conic_center();
        let oz2 = o.z() * o.z();
        let oqo = qn.evaluate(&o);
        let out = Conic {
            q: qn.q.mul_complex(oz2).sub(&L_INF_SQR.mul_complex(oqo.mul_f64(2.0))),
        };
        out.normalize().unwrap_or(out)
    }

    /// 复/实离心率平方 `e²`（统一覆盖圆 `0`、椭圆 `(0,1)`、抛物线 `1`、双曲线 `>1`、虚椭圆与复曲线）。
    #[must_use]
    pub fn conic_eccentricity_sqr(&self) -> Complex {
        let qn = match self.normalize() {
            Some(n) => n,
            None => return Complex::new(f64::NAN, f64::NAN),
        };
        let (a, b, c) = (qn.a(), qn.b(), qn.c());
        let delta = b * b - a * c;
        if delta.abs() <= 1e-13 {
            // 抛物线（或极接近抛物线）
            return Complex::ONE;
        }
        let diff = a - c;
        let t = a + c;
        let r = (diff * diff + b * b.mul_f64(4.0)).sqrt();
        if r.is_zero() {
            return Complex::ZERO;
        }

        // 射影实曲线（含带全局复相位的实曲线）由 det(Q) 符号唯一确定主轴与 e²；
        // 非实复曲线取非零较大分母分支以保证解析连续。
        let denom = if let Some(mr) = qn.try_to_3x3f64(1e-11) {
            let det_r = mr[0][0] * (mr[1][1] * mr[2][2] - mr[1][2] * mr[2][1])
                - mr[0][1] * (mr[1][0] * mr[2][2] - mr[1][2] * mr[2][0])
                + mr[0][2] * (mr[1][0] * mr[2][1] - mr[1][1] * mr[2][0]);
            if det_r > 0.0 {
                r - t
            } else {
                r + t
            }
        } else if (r + t).abs() >= (r - t).abs() {
            r + t
        } else {
            r - t
        };

        r.mul_f64(2.0) / denom
    }

    /// 离心率 `e = √(e²)`（主值分支）。
    #[inline]
    #[must_use]
    pub fn conic_eccentricity(&self) -> Complex {
        self.conic_eccentricity_sqr().sqrt()
    }

    /// 求解对偶二次曲线束 `det(Q* − λ·C∞*) = 0`，返回两对焦点 `(主/实焦点对, 副/虚焦点对)`。
    ///
    /// - 椭圆 / 双曲线：`( {F₁, F₂}实, {G₁, G₂}虚 )`；
    /// - 抛物线：`({F, P_inf}, {I, J})`；
    /// - 圆：`({O, O}, {O, O})`；
    /// - 复曲线：返回两个对偶退化分解所得的焦点对。
    #[must_use]
    pub fn conic_foci_pair(&self) -> (Ddot, Ddot) {
        let zero = Ddot::new(Dot::from_vec3c(Vec3C::ZERO), Dot::from_vec3c(Vec3C::ZERO));
        let q_dual = match (Conic {
            q: self.q.adjugate(),
        })
            .normalize()
        {
            Some(n) => n.q,
            None => return (zero, zero),
        };

        let m = &q_dual.m;
        let (u00, u01, u02) = (m[0][0], m[0][1], m[0][2]);
        let (u11, u12, u22) = (m[1][1], m[1][2], m[2][2]);

        let c2 = u22;
        let c1 = -(u22 * (u00 + u11) - u02 * u02 - u12 * u12);
        let c0 = q_dual.det();

        let ((ln1, ld1), (ln2, ld2)) = solve_quadratic_homogeneous(c2, c1, c0);
        let m1 = Conic {
            q: q_dual.mul_complex(ld1).sub(&C_INF_DUAL.mul_complex(ln1)),
        };
        let m2 = Conic {
            q: q_dual.mul_complex(ld2).sub(&C_INF_DUAL.mul_complex(ln2)),
        };

        let split_to_ddot = |mc: Conic| -> Ddot {
            match mc.decompose_lines(1e-12) {
                Some((la, lb)) => Ddot::new(line_to_dot_coords(&la), line_to_dot_coords(&lb)),
                None => zero,
            }
        };

        let d1 = split_to_ddot(m1);
        let d2 = split_to_ddot(m2);

        let is_ddot_real = |d: &Ddot| -> bool {
            d.p().try_to_3f64(1e-9).is_some() && d.q().try_to_3f64(1e-9).is_some()
        };
        if !is_ddot_real(&d1) && is_ddot_real(&d2) {
            (d2, d1)
        } else {
            (d1, d2)
        }
    }

    /// 四个焦点构成的合点 `Qdot`。
    #[inline]
    #[must_use]
    pub fn conic_foci(&self) -> Qdot {
        let (d1, d2) = self.conic_foci_pair();
        Qdot::from_ddot2(d1, d2)
    }

    /// 主（实）焦点对（抛物线返回 `{F, P_inf}`）。
    #[inline]
    #[must_use]
    pub fn conic_real_foci(&self) -> Ddot {
        self.conic_foci_pair().0
    }

    /// 全部四条准线组成的骈交叉直线 `DXLine`（各焦点关于曲线的极线）。
    #[inline]
    #[must_use]
    pub fn conic_directrices(&self) -> DXLine {
        let (d1, d2) = self.conic_foci_pair();
        DXLine::new(
            XLine::new(self.polar(&d1.p()), self.polar(&d1.q())),
            XLine::new(self.polar(&d2.p()), self.polar(&d2.q())),
        )
    }

    /// 主（实）准线对（抛物线返回 `{directrix, Line::INF}`）。
    #[inline]
    #[must_use]
    pub fn conic_real_directrices(&self) -> XLine {
        let d1 = self.conic_real_foci();
        XLine::new(self.polar(&d1.p()), self.polar(&d1.q()))
    }

    /// 曲线的主直径（对称轴对）`XLine`。
    ///
    /// 由正交共轭方向方程 `B·dx² − (A − C)·dx·dy − B·dy² = 0` 解出两个主方向后直接取极线：
    /// 无需中心点作连接，抛物线自动给出 `{axis, Line::INF}`，圆给出过圆心的正交坐标轴。
    #[must_use]
    pub fn conic_axes(&self) -> XLine {
        let qn = self.normalize().unwrap_or(*self);
        let (a, b, c) = (qn.a(), qn.b(), qn.c());
        let diff = a - c;

        let (d1, d2) = if diff.abs() <= 1e-12 && b.abs() <= 1e-12 {
            // 圆或退化至各向同性对称：取标准正交方向
            (
                Dot::new(Complex::ONE, Complex::ZERO, Complex::ZERO),
                Dot::new(Complex::ZERO, Complex::ONE, Complex::ZERO),
            )
        } else {
            let b2 = b.mul_f64(2.0);
            let r_raw = (diff * diff + b2 * b2).sqrt();
            let r = if (diff.conj() * r_raw).re >= 0.0 {
                r_raw
            } else {
                -r_raw
            };
            let u1 = diff + r;
            let v1 = b2;
            (
                Dot::new(u1, v1, Complex::ZERO),
                Dot::new(-v1, u1, Complex::ZERO),
            )
        };

        let l1 = qn.polar(&d2);
        let l2 = qn.polar(&d1);
        XLine::new(
            l1.normalize().unwrap_or(l1),
            l2.normalize().unwrap_or(l2),
        )
    }

    /// 主直径与曲线的四个交点（顶点），以合点 `Qdot` 表示（无对称破缺与旋转跳变）。
    ///
    /// - 椭圆：4 个实顶点；
    /// - 双曲线：2 个实顶点 + 2 个虚共轭顶点；
    /// - 抛物线：`{vertex, P_inf}` 与 `{I, J}`。
    #[inline]
    #[must_use]
    pub fn conic_vertices(&self) -> Qdot {
        let axes = self.conic_axes();
        Qdot::from_ddot2(
            self.intersect_line(&axes.lm()),
            self.intersect_line(&axes.ln()),
        )
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::pakoo_forest::calc::Complex;
    use crate::pakoo_forest::geo::conic::Conic;
    use crate::pakoo_forest::geo::d_dot::Ddot;
    use crate::pakoo_forest::geo::dot::Dot;
    use crate::pakoo_forest::geo::line::Line;
    use crate::pakoo_forest::geo::x_line::XLine;

    const EPS: f64 = 1e-10;

    fn affine(x: f64, y: f64) -> Dot {
        Dot::from_f64s(x, y, 1.0)
    }

    #[test]
    fn from_5_dots_and_5_lines() {
        // 椭圆 x²/25 + y²/16 = 1 上的 5 个点
        let pts = [
            affine(5.0, 0.0),
            affine(-5.0, 0.0),
            affine(0.0, 4.0),
            affine(3.0, 16.0 / 5.0),
            affine(-4.0, -12.0 / 5.0),
        ];
        let c_5p = Conic::conic_from_5_dots(pts[0], pts[1], pts[2], pts[3], pts[4]);
        let expected = Conic::from_f64s(16.0, 0.0, 25.0, 0.0, 0.0, -400.0);
        assert!(c_5p.approx_eq(&expected, EPS));
        for p in &pts {
            assert!(c_5p.contains(p, EPS));
        }

        // 5 条切线（直接由曲线上 5 点的切线生成）反推原曲线
        let tangents = pts.map(|p| expected.polar(&p));
        let c_5l = Conic::conic_from_5_lines(
            tangents[0],
            tangents[1],
            tangents[2],
            tangents[3],
            tangents[4],
        );
        assert!(c_5l.approx_eq(&expected, EPS));

        // 复数 5 点定复二次曲线
        let cpts = [
            Dot::new(Complex::new(1.0, 1.0), Complex::ZERO, Complex::ONE),
            Dot::new(Complex::ZERO, Complex::new(2.0, -1.0), Complex::ONE),
            Dot::new(Complex::new(-1.0, 0.5), Complex::new(1.0, 1.0), Complex::ONE),
            Dot::new(Complex::new(2.0, 0.0), Complex::new(-1.0, 0.5), Complex::ONE),
            Dot::new(Complex::new(0.5, -1.0), Complex::new(0.5, 2.0), Complex::ONE),
        ];
        let c_complex = Conic::conic_from_5_dots(cpts[0], cpts[1], cpts[2], cpts[3], cpts[4]);
        for p in &cpts {
            assert!(c_complex.contains(p, EPS));
        }
    }

    #[test]
    fn focus_directrix_and_asymptotes_constructors() {
        // 抛物线 y² = 4x：焦点 (1,0)，准线 x = -1
        let par = Conic::conic_from_parabola_focus_directrix(
            affine(1.0, 0.0),
            Line::from_f64s(1.0, 0.0, 1.0),
        );
        let par_expected = Conic::from_f64s(0.0, 0.0, 1.0, -2.0, 0.0, 0.0);
        assert!(par.approx_eq(&par_expected, EPS));
        assert!((par.conic_eccentricity().re - 1.0).abs() < EPS);

        // 离心率 e = 0.6 构造椭圆（焦点 (3,0)，准线 x = 25/3 → x²/25 + y²/16 = 1）
        let ell = Conic::conic_from_focus_directrix_ecc_f64(
            affine(3.0, 0.0),
            Line::from_f64s(3.0, 0.0, -25.0),
            0.6,
        );
        let ell_expected = Conic::from_f64s(16.0, 0.0, 25.0, 0.0, 0.0, -400.0);
        assert!(ell.approx_eq(&ell_expected, EPS));
        assert!((ell.conic_eccentricity().re - 0.6).abs() < EPS);

        // 渐近线 + 一点：实渐近线 → 双曲线
        let asy_real = XLine::new(Line::from_f64s(1.0, -1.0, 0.0), Line::from_f64s(1.0, 1.0, 0.0));
        let hyp = Conic::conic_from_asymptotes_dot(asy_real, affine(2.0, 1.0));
        assert!(hyp.approx_eq(&Conic::from_f64s(1.0, 0.0, -1.0, 0.0, 0.0, -3.0), EPS));

        // 虚渐近线 → 椭圆 (x/5 ± i·y/4 = 0)
        let asy_imag = XLine::new(
            Line::new(Complex::from_f64(0.2), Complex::new(0.0, 0.25), Complex::ZERO),
            Line::new(Complex::from_f64(0.2), Complex::new(0.0, -0.25), Complex::ZERO),
        );
        let ell_from_asy = Conic::conic_from_asymptotes_dot(asy_imag, affine(5.0, 0.0));
        assert!(ell_from_asy.approx_eq(&ell_expected, EPS));
    }

    #[test]
    fn center_conjugate_and_foci_point_constructors() {
        let ctr = affine(1.0, -2.0);
        // 旋转共轭半直径构造椭圆与双曲线
        let p1 = affine(4.0, -1.0); // u = (3, 1)
        let p2 = affine(0.0, 0.0);  // v = (-1, 2)
        let ell = Conic::conic_from_center_conjugate_ellipse(ctr, p1, p2);
        assert!(ell.contains(&p1, EPS) && ell.contains(&p2, EPS));
        assert!(ell.conic_center().p.cross(ctr.p).norm() < EPS);
        assert!(ell.conic_are_conjugate_directions(&ctr.join(p1), &ctr.join(p2), EPS));

        let hyp = Conic::conic_from_center_conjugate_hyperbola(ctr, p1, p2);
        assert!(hyp.contains(&p1, EPS));
        assert!(!hyp.contains(&p2, EPS));
        assert!(hyp.conic_conjugate_hyperbola().contains(&p2, EPS));
        assert!(hyp.conic_are_conjugate_directions(&ctr.join(p1), &ctr.join(p2), EPS));

        // 两焦点 + 一点构造椭圆 & 共焦双曲线
        let f1 = affine(-3.0, 0.0);
        let f2 = affine(3.0, 0.0);
        let p = affine(4.0, 12.0 / 5.0);
        let ell_f = Conic::conic_from_2_foci_point_ellipse(f1, f2, p);
        assert!(ell_f.approx_eq(&Conic::from_f64s(16.0, 0.0, 25.0, 0.0, 0.0, -400.0), EPS));

        let p_hyp = affine(5.0, 4.0); // 在 x²/5 - y²/4 = 1 上（焦距仍是 c = 3）
        let hyp_f = Conic::conic_from_foci_point_hyperbola(Ddot::new(f1, f2), p_hyp);
        assert!(hyp_f.approx_eq(&Conic::from_f64s(4.0, 0.0, -5.0, 0.0, 0.0, -20.0), EPS));

        // 复焦点（取自 x²/3 + y²/4 = 1 的虚焦点 ±i）反推实椭圆
        let cf1 = Dot::new(Complex::I, Complex::ZERO, Complex::ONE);
        let cf2 = Dot::new(Complex::NEG_I, Complex::ZERO, Complex::ONE);
        let ell_cf = Conic::conic_from_2_foci_point_ellipse(cf1, cf2, affine(0.0, 2.0));
        assert!(ell_cf.approx_eq(&Conic::from_f64s(4.0, 0.0, 3.0, 0.0, 0.0, -12.0), EPS));
    }

    #[test]
    fn extraction_foci_directrices_axes_vertices_ecc_tangents() {
        // 1. 倾斜椭圆：将 x²/25 + y²/16 = 1 旋转 30° 并平移到 (2, -1)
        let theta = std::f64::consts::FRAC_PI_6;
        let (s, c) = theta.sin_cos();
        // 构建原椭圆并手动做刚体变换测试
        let f1 = affine(2.0 + 3.0 * c, -1.0 + 3.0 * s);
        let f2 = affine(2.0 - 3.0 * c, -1.0 - 3.0 * s);
        let vertex = affine(2.0 + 5.0 * c, -1.0 + 5.0 * s);
        let ell = Conic::conic_from_2_foci_point_ellipse(f1, f2, vertex);

        assert!((ell.conic_eccentricity().re - 0.6).abs() < EPS);
        assert!(ell.conic_real_foci().approx_eq(&Ddot::new(f1, f2), EPS));
        let axes = ell.conic_axes();
        assert!(axes.lm().is_vert(axes.ln(), EPS));
        let verts = ell.conic_vertices();
        assert!(verts.ddot_1().contains(&vertex, EPS) || verts.ddot_2().contains(&vertex, EPS));

        // 2. 双曲线 x²/9 - y²/16 = 1 → e = 5/3，渐近线 4x ± 3y = 0
        let hyp = Conic::from_f64s(16.0, 0.0, -9.0, 0.0, 0.0, -144.0);
        assert!((hyp.conic_eccentricity().re - 5.0 / 3.0).abs() < EPS);
        assert!(hyp.conic_asymptotes().approx_eq(
            &XLine::new(Line::from_f64s(4.0, -3.0, 0.0), Line::from_f64s(4.0, 3.0, 0.0)),
            EPS
        ));
        assert!(hyp.conic_real_foci().approx_eq(
            &Ddot::new(affine(5.0, 0.0), affine(-5.0, 0.0)),
            EPS
        ));
        assert!(hyp.conic_real_directrices().approx_eq(
            &XLine::new(Line::from_f64s(5.0, 0.0, -9.0), Line::from_f64s(5.0, 0.0, 9.0)),
            EPS
        ));

        // 3. 旋转抛物线 (x - y)² = 4(x + y) → 焦点 (1/2, 1/2)，对称轴 x - y = 0
        let par = Conic::from_f64s(1.0, -1.0, 1.0, -2.0, -2.0, 0.0);
        assert!((par.conic_eccentricity().re - 1.0).abs() < EPS);
        let (pf_real, pf_imag) = par.conic_foci_pair();
        assert!(pf_real.approx_eq(&Ddot::new(affine(0.5, 0.5), Dot::from_f64s(1.0, 1.0, 0.0)), EPS));
        assert!(pf_imag.approx_eq(&Ddot::CYCLIC, EPS));
        let par_axes = par.conic_axes();
        assert!(par_axes.approx_eq(
            &XLine::new(Line::from_f64s(1.0, -1.0, 0.0), Line::INF),
            EPS
        ));

        // 4. 切线对：验证 bubo.md 的正交焦弦/极线性质
        let ext = affine(6.0, 5.0);
        let tangents = ell.conic_tangents_from(&ext);
        assert!(tangents.lm().contains(&ext, EPS) && tangents.ln().contains(&ext, EPS));
        // 两切线与曲线各恰交于单点（切点）
        let d_t1 = ell.intersect_line(&tangents.lm());
        let d_t2 = ell.intersect_line(&tangents.ln());
        assert!(d_t1.p().p.cross(d_t1.q().p).norm() < 1e-6);
        assert!(d_t2.p().p.cross(d_t2.q().p).norm() < 1e-6);
        // 切点连线等于 ext 关于 ell 的极线
        assert!(d_t1.p().join(d_t2.p()).approx_eq(&ell.polar(&ext), 1e-7));
    }
}