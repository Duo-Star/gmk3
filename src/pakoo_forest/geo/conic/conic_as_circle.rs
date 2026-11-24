// src/pakoo_forest/geo/conic/conic_as_circle.rs
//！
//! `q : A·x² + 2B·xy + C·y² + 2D·xz + 2E·yz + F·z² = 0`
//! `Q = [A B D; B C E; D E F]``
//!
//!
//! `Conic` 的「圆与广义圆扩展」
//! `A = C` 且 `B = 0`，即过虚圆点 `I, J` 的二次曲线族
//!
//! # 广义圆统一空间
//!
//! 形如 `A·(x² + y²) + 2D·xz + 2E·yz + F·z² = 0` 的曲线构成 4 维线性子空间：
//!
//! - `A ≠ 0` 且 `AF − D² − E² ≠ 0`：非退化圆（实圆或虚圆）；
//! - `A ≠ 0` 且 `AF − D² − E² = 0`：点圆（半径为 0，退化为过圆心的两条迷向直线）；
//! - `A = 0`：广义直线 `z·(2Dx + 2Ey + Fz) = 0`（直线 `l` 与无穷远直线 `L∞` 之并，过反演中心的圆反演后的像即落入此类）。
//!
//! # 设计约定
//!
//! - 全程齐次与复数兼容、无除法与条件分支（除极少数防退化的无分支缩放外）；
//! - 圆上一点构造、圆幂、正交、反演均使用**全纯复二次型**，确保复圆心/复半径/复点
//!   同样严格满足关联与反演对合性。
//!

use crate::pakoo_forest::calc::Complex;
use crate::pakoo_forest::geo::conic::Conic;
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::dot::Dot;
use crate::pakoo_forest::geo::line::Line;

#[inline]
fn det3(r0: [Complex; 3], r1: [Complex; 3], r2: [Complex; 3]) -> Complex {
    r0[0] * (r1[1] * r2[2] - r1[2] * r2[1]) - r0[1] * (r1[0] * r2[2] - r1[2] * r2[0])
        + r0[2] * (r1[0] * r2[1] - r1[1] * r2[0])
}

impl Conic {
    // ------------------------------------------------------------------
    // 谓词与广义圆—直线桥接
    // ------------------------------------------------------------------

    /// 是否属于广义圆空间（`A = C` 且 `B = 0`，允许 `A = C = 0` 即广义直线）
    /// generalized 广义的
    #[inline]
    pub fn is_generalized_circle(&self, eps: f64) -> bool {
        if !self.is_finite() || self.is_zero() {
            return false;
        }
        let q = if eps == 0.0 {
            *self
        } else {
            match self.normalize() {
                Some(n) => n,
                None => return false,
            }
        };
        (q.a() - q.c()).abs() <= eps && q.b().abs() <= eps
    }

    /// 是否为广义圆意义下的直线（`A = C = B = 0`，即 `z·(2Dx + 2Ey + Fz) = 0`）
    #[inline]
    pub fn is_generalized_line(&self, eps: f64) -> bool {
        if !self.is_generalized_circle(eps) {
            return false;
        }
        let q = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        q.a().abs() <= eps
    }

    /// 从广义直线 `z·(2Dx + 2Ey + Fz) = 0` 直接提取仿射/射影直线 `(2D : 2E : F)`（不校验）。
    #[inline]
    #[must_use]
    pub fn generalized_circle_as_line(&self) -> Line {
        Line::new(self.d().mul_f64(2.0), self.e().mul_f64(2.0), self.f())
    }

    /// 将直线 `l = (a:b:c)` 嵌入广义圆空间：`z·(ax + by + cz) = 0`。
    #[inline]
    #[must_use]
    pub fn line_as_generalized_circle(l: &Line) -> Conic {
        Conic::new(
            Complex::ZERO,
            Complex::ZERO,
            Complex::ZERO,
            l.x().mul_f64(0.5),
            l.y().mul_f64(0.5),
            l.z(),
        )
    }

    // ------------------------------------------------------------------
    // 圆构造器
    // ------------------------------------------------------------------

    /// 由圆心 `center = (X₀:Y₀:Z₀)` 与复半径 `r` 构造圆（全纯齐次，无需对 `Z₀` 作除法）。
    ///
    /// 若 `center` 在无穷远（`Z₀ = 0`），自然退化为 `(X₀²+Y₀²)·z² = 0`。
    #[inline]
    #[must_use]
    pub fn circle_from_center_r_complex(center: Dot, r: Complex) -> Conic {
        let (x0, y0, z0) = (center.x(), center.y(), center.z());
        let z02 = z0 * z0;
        let r2 = r * r;
        Conic::new(
            z02,
            Complex::ZERO,
            z02,
            -x0 * z0,
            -y0 * z0,
            x0 * x0 + y0 * y0 - r2 * z02,
        )
    }

    /// 由圆心 `center` 与实半径 `r` 构造圆。
    #[inline]
    #[must_use]
    pub fn circle_from_center_r_f64(center: Dot, r: f64) -> Conic {
        Conic::circle_from_center_r_complex(center, Complex::from_f64(r))
    }

    /// 由圆心 `center = (X₀:Y₀:Z₀)` 与圆上一点 `on_circle = (X₁:Y₁:Z₁)` 构造圆。
    ///
    /// 已约去公共因子 `Z₀`：当圆心位于无穷远（`Z₀ = 0`）时，平滑退化为过 `on_circle`
    /// 且法向为 `(X₀:Y₀)` 的广义直线。
    /// cir_cd
    #[inline]
    #[must_use]
    pub fn circle_from_center_point(center: Dot, on_circle: Dot) -> Conic {
        let (x0, y0, z0) = (center.x(), center.y(), center.z());
        let (x1, y1, z1) = (on_circle.x(), on_circle.y(), on_circle.z());
        let z1_2 = z1 * z1;
        let a = z0 * z1_2;
        let d = -x0 * z1_2;
        let e = -y0 * z1_2;
        let f = (x0 * x1 + y0 * y1) * z1.mul_f64(2.0) - z0 * (x1 * x1 + y1 * y1);
        Conic::new(a, Complex::ZERO, a, d, e, f)
    }

    /// 由直径两端点 `endpoints = {P₁, P₂}` 构造泰勒斯圆（对 `P₁, P₂` 严格对称）。
    /// cir_dd
    #[inline]
    #[must_use]
    pub fn circle_from_diameter(endpoints: Ddot) -> Conic {
        let (p1, p2) = (endpoints.p(), endpoints.q());
        let (x1, y1, z1) = (p1.x(), p1.y(), p1.z());
        let (x2, y2, z2) = (p2.x(), p2.y(), p2.z());
        let a = z1 * z2;
        let d = -(x1 * z2 + x2 * z1).mul_f64(0.5);
        let e = -(y1 * z2 + y2 * z1).mul_f64(0.5);
        let f = x1 * x2 + y1 * y2;
        Conic::new(a, Complex::ZERO, a, d, e, f)
    }

    /// 三点定圆（齐次 4×4 行列式闭式解）。
    ///
    /// 三点共线时 `A = C = 0`，自动退化为该直线与 `L∞` 组成的广义直线；
    /// 存在重合点时行列式全为零，返回 `Conic::ZERO`。
    /// cir_d3
    #[must_use]
    pub fn circle_from_3_dots(p1: Dot, p2: Dot, p3: Dot) -> Conic {
        let n1 = p1.normalize().unwrap_or(p1);
        let n2 = p2.normalize().unwrap_or(p2);
        let n3 = p3.normalize().unwrap_or(p3);

        let w = [
            n1.x() * n1.x() + n1.y() * n1.y(),
            n2.x() * n2.x() + n2.y() * n2.y(),
            n3.x() * n3.x() + n3.y() * n3.y(),
        ];
        let u = [n1.x() * n1.z(), n2.x() * n2.z(), n3.x() * n3.z()];
        let v = [n1.y() * n1.z(), n2.y() * n2.z(), n3.y() * n3.z()];
        let t = [n1.z() * n1.z(), n2.z() * n2.z(), n3.z() * n3.z()];

        let a = det3(u, v, t);
        let d = -det3(w, v, t).mul_f64(0.5);
        let e = det3(w, u, t).mul_f64(0.5);
        let f = -det3(w, u, v);
        Conic::new(a, Complex::ZERO, a, d, e, f)
    }

    // ------------------------------------------------------------------
    // 圆的几何量与操作
    // ------------------------------------------------------------------

    /// 圆心 `(-D : -E : A)`（对广义直线 `A = 0` 给出其法方向上的无穷远点）。
    /// cir_center
    #[inline]
    #[must_use]
    pub fn circle_center(&self) -> Dot {
        Dot::new(-self.d(), -self.e(), self.a())
    }

    /// 复半径平方 `r² = (D² + E² − A·F) / A²`（对虚圆给出负数或复数；广义直线返回非有限值）。
    #[inline]
    #[must_use]
    pub fn circle_radius_sqr(&self) -> Complex {
        let (a, d, e, f) = (self.a(), self.d(), self.e(), self.f());
        (d * d + e * e - a * f) / (a * a)
    }

    /// 复半径 `√(r²)`（取 `Complex::sqrt` 主值分支）。
    #[inline]
    #[must_use]
    pub fn circle_radius(&self) -> Complex {
        self.circle_radius_sqr().sqrt()
    }

    /// 点 `p` 对圆的圆幂 `(pᵀ·Q·p) / (A·z²)`（与圆和点的整体缩放无关）。
    #[inline]
    #[must_use]
    pub fn circle_power(&self, p: &Dot) -> Complex {
        self.evaluate(p) / (self.a() * p.z() * p.z())
    }

    /// 两圆是否正交（无分支双线性配对 `2(D₁D₂ + E₁E₂) − A₁F₂ − A₂F₁ = 0`）。
    ///
    /// 当其中之一为广义直线（`A = 0`）时，自动等价于“该直线过另一圆的圆心”。
    #[inline]
    pub fn circle_orthogonal(&self, other: &Conic, eps: f64) -> bool {
        let q1 = if eps == 0.0 {
            *self
        } else {
            self.normalize().unwrap_or(*self)
        };
        let q2 = if eps == 0.0 {
            *other
        } else {
            other.normalize().unwrap_or(*other)
        };
        let val =
            (q1.d() * q2.d() + q1.e() * q2.e()).mul_f64(2.0) - q1.a() * q2.f() - q2.a() * q1.f();
        let scale = if eps == 0.0 {
            0.0
        } else {
            eps * (1.0 + q1.norm() * q2.norm())
        };
        val.abs() <= scale
    }

    /// 两圆的根轴（等幂轴）：`A₂·Q₁ − A₁·Q₂` 消去二次项后提取的一次直线。
    ///
    /// - 同心圆自然给出无穷远直线 `Line::INF`；
    /// - 重合圆返回零直线 `(0:0:0)`；
    /// - 与广义直线（`A₂ = 0`）求根轴即得该直线本身。
    /// cir_axis
    #[inline]
    #[must_use]
    pub fn circle_radical_axis(&self, other: &Conic) -> Line {
        let q1 = self.normalize().unwrap_or(*self);
        let q2 = other.normalize().unwrap_or(*other);
        let (a1, a2) = (q1.a(), q2.a());
        Line::new(
            (a2 * q1.d() - a1 * q2.d()).mul_f64(2.0),
            (a2 * q1.e() - a1 * q2.e()).mul_f64(2.0),
            a2 * q1.f() - a1 * q2.f(),
        )
    }

    /// 三个圆的根心。
    /// cir3_center
    #[inline]
    #[must_use]
    pub fn circle_radical_center(&self, c2: &Conic, c3: &Conic) -> Dot {
        self.circle_radical_axis(c2)
            .meet(self.circle_radical_axis(c3))
    }

    /// 两圆求交，返回骈点 `Ddot`。
    ///
    /// 先求两圆根轴，再求根轴与 `self` 的交点：
    /// 统一覆盖相交、相切（重根）、相离（共轭复点对）与同心（返回 `Ddot::CYCLIC = {I, J}`）。
    /// cir2_ins
    #[inline]
    #[must_use]
    pub fn circle_intersect_circle(&self, other: &Conic) -> Ddot {
        let axis = self.circle_radical_axis(other);
        self.intersect_line(&axis)
    }

    // ------------------------------------------------------------------
    // 反演（点反演 & 广义圆反演）
    // ------------------------------------------------------------------

    /// 点 `p` 关于当前圆 `Q₀` 的反演像。
    ///
    /// - 采用中心化齐次多项式：令 `dx = A₀·X + D₀·Z`, `dy = A₀·Y + E₀·Z`, `k₀ = D₀² + E₀² − A₀·F₀`，
    ///   当 `p` 恰好落在圆心 `(-D₀:-E₀:A₀)` 上时，`dx = dy = 0`，严格无 1-ULP 残差地返回 `(0:0:0)`
    ///   （射影几何下圆心炸开为整条 `Line::INF`，是 Cremona 二次变换的基点）；
    /// - 无穷远点 `(X:Y:0)`（非 `I, J`）反演回圆心；
    /// - 当 `A₀ = 0`（退化为广义直线）时自动退化为关于该直线的镜像反射。
    #[must_use]
    pub fn circle_invert_dot(&self, p: &Dot) -> Dot {
        let q0 = self.normalize().unwrap_or(*self);
        let pn = p.normalize().unwrap_or(*p);
        let (a0, d0, e0, f0) = (q0.a(), q0.d(), q0.e(), q0.f());
        let (x, y, z) = (pn.x(), pn.y(), pn.z());
        let k0 = d0 * d0 + e0 * e0 - a0 * f0;

        let out = if a0.is_zero() {
            // 广义直线退化：反射公式
            let p_q_p = d0 * x * z.mul_f64(2.0) + e0 * y * z.mul_f64(2.0) + f0 * z * z;
            Dot::new(k0 * z * x - d0 * p_q_p, k0 * z * y - e0 * p_q_p, k0 * z * z)
        } else {
            let dx = a0 * x + d0 * z;
            let dy = a0 * y + e0 * z;
            let n2 = dx * dx + dy * dy;
            Dot::new(k0 * z * dx - d0 * n2, k0 * z * dy - e0 * n2, a0 * n2)
        };
        out.normalize().unwrap_or(out)
    }

    /// 广义圆 `target`（包括普通圆与 `line_as_generalized_circle` 构造的直线）关于当前圆 `self` 的反演像。
    ///
    /// 全纯多项式闭式变换（零除法、零分支）：
    /// - 不过反演中心的圆 → 不过反演中心的圆；
    /// - 过反演中心的圆 → `A' = 0` 的广义直线（可用 [`Conic::generalized_circle_as_line`] 提取）；
    /// - 不过反演中心的直线 → 过反演中心的圆；
    /// - 过反演中心的直线 → 自身；
    /// - 当反演基 `self` 为广义直线（`A₀ = 0`）时，自动退化为关于该直线的镜像反射。
    #[must_use]
    pub fn circle_invert_conic_as_circle(&self, target: &Conic) -> Conic {
        let q0 = self.normalize().unwrap_or(*self);
        let qt = target.normalize().unwrap_or(*target);
        let (a0, d0, e0, f0) = (q0.a(), q0.d(), q0.e(), q0.f());
        let (a, d, e, f) = (qt.a(), qt.d(), qt.e(), qt.f());

        let s0 = d0 * d0 + e0 * e0;
        let k0 = s0 - a0 * f0;
        let t = d0 * d + e0 * e;

        let a_out = s0 * a - a0 * t.mul_f64(2.0) + a0 * a0 * f;
        let d_out =
            f0 * d0 * a + (k0 - d0 * d0.mul_f64(2.0)) * d - d0 * e0 * e.mul_f64(2.0) + a0 * d0 * f;
        let e_out =
            f0 * e0 * a - d0 * e0 * d.mul_f64(2.0) + (k0 - e0 * e0.mul_f64(2.0)) * e + a0 * e0 * f;
        let f_out = f0 * f0 * a - f0 * t.mul_f64(2.0) + s0 * f;

        let out = Conic::new(a_out, Complex::ZERO, a_out, d_out, e_out, f_out);
        out.normalize().unwrap_or(out)
    }

    /// 直线 `l` 关于当前圆 `self` 的反演像（返回广义圆：不过圆心时为过圆心的圆，过圆心时为自身 `l ∪ L∞`）。
    #[inline]
    #[must_use]
    pub fn circle_invert_line(&self, l: &Line) -> Conic {
        self.circle_invert_conic_as_circle(&Conic::line_as_generalized_circle(l))
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

    const EPS: f64 = 1e-11;

    #[test]
    fn constructors_and_metrics() {
        // 圆心 + 实半径（含齐次非 1 缩放）
        let c1 = Conic::circle_from_center_r_f64(Dot::from_f64s(4.0, -6.0, 2.0), 5.0);
        assert!(c1.is_circle(EPS));
        assert!(c1.is_generalized_circle(EPS));
        let ctr = c1.circle_center().to_affine_f64().unwrap();
        assert!((ctr.0 - 2.0).abs() < EPS && (ctr.1 + 3.0).abs() < EPS);
        assert!((c1.circle_radius().re - 5.0).abs() < EPS);

        // 圆心 + 复半径（虚圆 x² + y² + 4 = 0）
        let c_imag = Conic::circle_from_center_r_complex(Dot::ZERO, Complex::new(0.0, 2.0));
        assert!(c_imag.is_circle(EPS));
        assert_eq!(c_imag.classify(EPS), "虚圆");
        assert!((c_imag.circle_radius_sqr().re + 4.0).abs() < EPS);

        // 圆心 + 圆上一点（含复点）
        let ctr_c = Dot::new(
            Complex::new(1.0, 1.0),
            Complex::new(0.0, -1.0),
            Complex::ONE,
        );
        let on_c = Dot::new(
            Complex::new(2.0, -1.0),
            Complex::new(3.0, 1.0),
            Complex::ONE,
        );
        let c_pt = Conic::circle_from_center_point(ctr_c, on_c);
        assert!(c_pt.contains(&on_c, EPS));
        assert!(c_pt.is_generalized_circle(EPS));

        // 圆心在无穷远 → 退化为过 on_circle 的直线
        let inf_ctr = Dot::from_f64s(1.0, 0.0, 0.0);
        let c_inf = Conic::circle_from_center_point(inf_ctr, Dot::from_f64s(3.0, 4.0, 1.0));
        assert!(c_inf.is_generalized_line(EPS));
        assert!(
            c_inf
                .generalized_circle_as_line()
                .approx_eq(&Line::from_f64s(1.0, 0.0, -3.0), EPS)
        );

        // 直径构造
        let d_end = Ddot::new(
            Dot::from_f64s(-1.0, 0.0, 1.0),
            Dot::from_f64s(1.0, 0.0, 1.0),
        );
        assert!(Conic::circle_from_diameter(d_end).approx_eq(&Conic::UNIT_CIRCLE, EPS));

        // 三点定圆 & 共线退化
        let c3 = Conic::circle_from_3_dots(
            Dot::from_f64s(1.0, 0.0, 1.0),
            Dot::from_f64s(0.0, 1.0, 1.0),
            Dot::from_f64s(-1.0, 0.0, 1.0),
        );
        assert!(c3.approx_eq(&Conic::UNIT_CIRCLE, EPS));

        let c_col = Conic::circle_from_3_dots(
            Dot::from_f64s(0.0, 1.0, 1.0),
            Dot::from_f64s(2.0, 1.0, 1.0),
            Dot::from_f64s(5.0, 1.0, 1.0),
        );
        assert!(c_col.is_generalized_line(EPS));
        assert!(
            c_col
                .generalized_circle_as_line()
                .approx_eq(&Line::from_f64s(0.0, 1.0, -1.0), EPS)
        );
    }

    #[test]
    fn power_orthogonal_radical_and_intersection() {
        let c1 = Conic::UNIT_CIRCLE; // x² + y² = 1
        // 过 (5/3, 0) 半径 4/3 → (5/3)² = 1 + (4/3)² 正交于单位圆
        let c2 = Conic::circle_from_center_r_f64(Dot::from_f64s(5.0 / 3.0, 0.0, 1.0), 4.0 / 3.0);
        assert!(c1.circle_orthogonal(&c2, EPS));

        // 圆幂验证：缩放圆和点不改圆幂值
        let p = Dot::from_f64s(3.0, 4.0, 1.0);
        assert!(
            (c1.scaled(Complex::new(2.0, 3.0))
                .circle_power(&p.scaled(Complex::new(0.0, 5.0)))
                .re
                - 24.0)
                .abs()
                < EPS
        );

        // 两圆求交（相交、相离复交点、同心）
        let c3 = Conic::circle_from_center_r_f64(Dot::from_f64s(1.0, 0.0, 1.0), 1.0);
        let d_int = c1.circle_intersect_circle(&c3);
        assert!(c1.contains(&d_int.p(), EPS) && c3.contains(&d_int.p(), EPS));
        assert!(c1.contains(&d_int.q(), EPS) && c3.contains(&d_int.q(), EPS));
        let axis = c1.circle_radical_axis(&c3);
        assert!(axis.approx_eq(&Line::from_f64s(2.0, 0.0, -1.0), EPS));
        assert!(axis.contains(&d_int.p(), EPS) && axis.contains(&d_int.q(), EPS));

        // 相离实圆 → 复共轭交点仍精确落在两圆与根轴上
        let c_far = Conic::circle_from_center_r_f64(Dot::from_f64s(5.0, 0.0, 1.0), 1.0);
        let d_far = c1.circle_intersect_circle(&c_far);
        assert!(c1.contains(&d_far.p(), EPS) && c_far.contains(&d_far.p(), EPS));
        assert!(c1.contains(&d_far.q(), EPS) && c_far.contains(&d_far.q(), EPS));

        // 同心圆 → 根轴为 L∞，交点为 {I, J}
        let c_con = Conic::circle_from_center_r_f64(Dot::ZERO, 3.0);
        assert!(c1.circle_radical_axis(&c_con).approx_eq(&Line::INF, EPS));
        assert!(
            c1.circle_intersect_circle(&c_con)
                .approx_eq(&Ddot::CYCLIC, EPS)
        );

        // 根心等幂验证
        let c_a = Conic::circle_from_center_r_f64(Dot::from_f64s(0.0, 0.0, 1.0), 2.0);
        let c_b = Conic::circle_from_center_r_f64(Dot::from_f64s(4.0, 0.0, 1.0), 1.0);
        let c_c = Conic::circle_from_center_r_f64(Dot::from_f64s(1.0, 3.0, 1.0), 1.5);
        let rc = c_a.circle_radical_center(&c_b, &c_c);
        let pa = c_a.circle_power(&rc);
        let pb = c_b.circle_power(&rc);
        let pc = c_c.circle_power(&rc);
        assert!((pa - pb).abs() < EPS && (pb - pc).abs() < EPS);
    }

    #[test]
    fn inversion_dot_line_and_circle() {
        let base = Conic::circle_from_center_r_f64(Dot::from_f64s(1.0, 2.0, 1.0), 3.0);
        let ctr = base.circle_center();

        // 圆心反演严格为零点（无 1-ULP 残留）
        assert!(base.circle_invert_dot(&ctr).is_zero());

        // 无穷远点反演回圆心
        let p_inf = Dot::from_f64s(2.0, -5.0, 0.0);
        assert!(base.circle_invert_dot(&p_inf).p.cross(ctr.p).norm() < EPS);

        // 对合性（含复点）
        let p = Dot::new(
            Complex::new(2.5, 0.5),
            Complex::new(-1.0, 0.3),
            Complex::ONE,
        );
        let p_inv = base.circle_invert_dot(&p);
        let p_inv2 = base.circle_invert_dot(&p_inv);
        assert!(p_inv2.p.cross(p.p).norm() < EPS);

        // 圆不过圆心 → 仍为圆，两次反演还原
        let target = Conic::circle_from_center_r_f64(Dot::from_f64s(5.0, 2.0, 1.0), 2.0);
        let inv_target = base.circle_invert_conic_as_circle(&target);
        assert!(inv_target.is_circle(EPS));
        assert!(
            base.circle_invert_conic_as_circle(&inv_target)
                .approx_eq(&target, EPS)
        );

        // 过圆心的圆 → 反演为直线，再反演回原圆
        let through_ctr = Conic::circle_from_center_point(Dot::from_f64s(4.0, 2.0, 1.0), ctr);
        let inv_line_conic = base.circle_invert_conic_as_circle(&through_ctr);
        assert!(inv_line_conic.is_generalized_line(EPS));
        let extracted_line = inv_line_conic.generalized_circle_as_line();
        let back = base.circle_invert_line(&extracted_line);
        assert!(back.approx_eq(&through_ctr, EPS));

        // 反演保正交（正交于基圆的圆反演到自身）
        let ortho = Conic::circle_from_center_r_f64(Dot::from_f64s(6.0, 2.0, 1.0), 4.0); // 5² = 3² + 4²
        assert!(base.circle_orthogonal(&ortho, EPS));
        assert!(
            base.circle_invert_conic_as_circle(&ortho)
                .approx_eq(&ortho, EPS)
        );

        // 基为广义直线时退化为镜像反射
        let y_axis_circle = Conic::line_as_generalized_circle(&Line::Y_AXIS);
        let refl = y_axis_circle.circle_invert_conic_as_circle(&target);
        let expected_refl = Conic::circle_from_center_r_f64(Dot::from_f64s(-5.0, 2.0, 1.0), 2.0);
        assert!(refl.approx_eq(&expected_refl, EPS));
    }
}
