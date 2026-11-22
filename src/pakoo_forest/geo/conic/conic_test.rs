// src/pakoo_forest/geo/conic/conic_test.rs
// ---------------------------------------------------------------------------
// Conic 单元测试
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::pakoo_forest::calc::{Complex, Mat3C};
    use crate::pakoo_forest::geo::Conic;
    use crate::pakoo_forest::geo::conic::conic_index::ConicParamKind;
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
        let t = unit.tangent_or_not(&p, 0.0).unwrap();
        assert!(t.approx_eq(&l, 0.0));
        // 不在曲线上的点无切线
        assert_eq!(
            unit.tangent_or_not(&Dot::from_f64s(2.0, 0.0, 1.0), 0.0),
            None
        );
        // 圆心的极线是无穷远直线
        let center = Dot::ZERO; // (0:0:1)
        assert!(unit.polar(&center).approx_eq(&Line::INF, 0.0));
        // 抛物线 y² − 2xz 与无穷远直线相切于 (1:0:0)，该处切线即 L∞
        let par = Conic::from_f64s(0.0, 0.0, 1.0, -1.0, 0.0, 0.0);
        let inf_point = Dot::from_f64s(1.0, 0.0, 0.0);
        assert_eq!(par.evaluate(&inf_point), Complex::ZERO);
        let t_inf = par.tangent_or_not(&inf_point, 0.0).unwrap();
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
        assert_ne!(q.evaluate(&Dot::J), Complex::ZERO);
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
            "Conic:(A: 1, B: 0, C: 1, D: 0, E: 0, F: -1)"
        );
        assert_eq!(
            format!("{:?}", Conic::ZERO),
            "Conic:(A: 0, B: 0, C: 0, D: 0, E: 0, F: 0)"
        );
    }

    // ---- 连续参数化 / 数值索引 ----

    #[test]
    fn param_nonsingular_real_conics() {
        let params = [
            -100.0,
            -2.5,
            -1.0,
            -0.3,
            0.0,
            0.4,
            1.0,
            3.7,
            1e8,
            f64::INFINITY,
        ];

        // 1. 单位圆
        let circle = Conic::UNIT_CIRCLE;
        let cp = circle.parametrization(EPS);
        assert_eq!(cp.kind(), ConicParamKind::NonSingular);
        for &t in &params {
            let p = cp.point_at_f64(t);
            assert!(p.is_real(), "实圆在 t={t} 应给出实点: {p}");
            assert!(circle.contains(&p, EPS), "t={t} 不在圆上: {p}");
            let (s, u) = cp.uv_of(&p).unwrap();
            let p_back = cp.point_at_uv(s, u);
            assert!(p.approx_eq(&p_back, EPS), "t={t} 往返失败: {p} vs {p_back}");
        }

        // 2. 对角线全为零的双曲线 2xy - z² = 0（检验 45° 主元旋转分支）
        let xy_hyp = Conic::from_f64s(0.0, 1.0, 0.0, 0.0, 0.0, -1.0);
        let hp = xy_hyp.parametrization(EPS);
        assert_eq!(hp.kind(), ConicParamKind::NonSingular);
        for &t in &params {
            let p = hp.point_at_f64(t);
            assert!(p.is_real());
            assert!(xy_hyp.contains(&p, EPS));
            let (s, u) = hp.uv_of(&p).unwrap();
            assert!(p.approx_eq(&hp.point_at_uv(s, u), EPS));
        }

        // 3. 抛物线 y² - 2xz = 0 与一般斜椭圆
        let parabola = Conic::from_f64s(0.0, 0.0, 1.0, -1.0, 0.0, 0.0);
        let ellipse = Conic::from_f64s(5.0, 2.0, 3.0, -1.0, 1.5, -4.0);
        for q in [parabola, ellipse] {
            let qp = q.parametrization(EPS);
            assert_eq!(qp.kind(), ConicParamKind::NonSingular);
            for &t in &params {
                let p = qp.point_at_f64(t);
                assert!(p.is_real());
                assert!(q.contains(&p, EPS));
                let (s, u) = qp.uv_of(&p).unwrap();
                assert!(p.approx_eq(&qp.point_at_uv(s, u), EPS));
            }
        }

        // 4. 虚圆 x² + y² + z² = 0
        let imag_circle = Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        let ip = imag_circle.parametrization(EPS);
        assert_eq!(ip.kind(), ConicParamKind::NonSingular);
        for &t in &params {
            let p = ip.point_at_f64(t);
            assert!(imag_circle.contains(&p, EPS));
        }

        // 5. 整体乘复标量的实圆仍给出实参数化
        let scaled_circle = circle.scaled(c(3.0, 4.0));
        let sp = scaled_circle.parametrization(EPS);
        assert!(sp.point_at_f64(0.5).is_real());
        assert!(scaled_circle.contains(&sp.point_at_f64(0.5), EPS));
    }

    #[test]
    fn param_nonsingular_complex_conic() {
        // 对角线全为零的复非退化二次曲线：2xy + 2iyz + 2xz = 0
        let q = Conic::new(
            Complex::ZERO,
            Complex::ONE,
            Complex::ZERO,
            Complex::ONE,
            Complex::I,
            Complex::ZERO,
        );
        let qp = q.parametrization(EPS);
        assert_eq!(qp.kind(), ConicParamKind::NonSingular);
        for t in [c(0.5, -1.3), c(-2.0, 0.7), Complex::I, Complex::ZERO] {
            let p = qp.point_at(t);
            assert!(q.contains(&p, EPS));
            let t_back = qp.param_of(&p).unwrap();
            assert!(p.approx_eq(&qp.point_at(t_back), EPS));
        }
    }

    #[test]
    fn param_degenerate_two_lines() {
        // 1. X 形相交直线：(x - 2z)² - 4(y + z)² = 0，交点 (2 : -1 : 1)
        let l_a = Line::from_f64s(1.0, -2.0, -4.0);
        let l_b = Line::from_f64s(1.0, 2.0, 0.0);
        let mat = l_a.to_cov3c().transpose() * l_b.to_cov3c();
        let x_conic = Conic::from_matrix(mat);
        let xp = x_conic.parametrization(EPS);
        assert_eq!(xp.kind(), ConicParamKind::TwoLines);
        let (l1, l2) = xp.lines().unwrap();

        let inter = Dot::from_f64s(2.0, -1.0, 1.0);
        assert!(xp.point_at_f64(-1.0).approx_eq(&inter, EPS));
        assert!(xp.point_at_f64(1.0).approx_eq(&inter, EPS));
        assert!(xp.point_at_f64(0.0).is_at_infinity());
        assert!(xp.point_at_f64(f64::INFINITY).is_at_infinity());

        // 四段各自对应四条完整欧氏射线，开区间内不穿过无穷远
        for t in [-0.7, -0.2, 0.2, 0.7] {
            let p = xp.point_at_f64(t);
            assert!(!p.is_at_infinity());
            assert!(l1.contains(&p, EPS));
            assert!(x_conic.contains(&p, EPS));
            let t_back = xp.param_of(&p).unwrap();
            assert!((t_back.re - t).abs() < EPS && t_back.im.abs() < EPS);
        }
        for t in [-5.0, -1.5, 1.5, 5.0] {
            let p = xp.point_at_f64(t);
            assert!(!p.is_at_infinity());
            assert!(l2.contains(&p, EPS));
            assert!(x_conic.contains(&p, EPS));
            let t_back = xp.param_of(&p).unwrap();
            assert!((t_back.re - t).abs() < EPS && t_back.im.abs() < EPS);
        }

        // 2. 平行直线：(x + 2y - z)(x + 2y + 3z) = 0
        let pa = Line::from_f64s(1.0, 2.0, -1.0);
        let pb = Line::from_f64s(1.0, 2.0, 3.0);
        let par_conic = Conic::from_matrix(pa.to_cov3c().transpose() * pb.to_cov3c());
        let pp = par_conic.parametrization(EPS);
        assert_eq!(pp.kind(), ConicParamKind::TwoLines);
        assert!(pp.point_at_f64(-1.0).is_at_infinity());
        assert!(pp.point_at_f64(1.0).is_at_infinity());
        for t in [-3.0, -0.5, 0.0, 0.5, 3.0, f64::INFINITY] {
            let p = pp.point_at_f64(t);
            assert!(!p.is_at_infinity(), "平行直线在 t={t} 应为有限点");
            assert!(par_conic.contains(&p, EPS));
            let (s, u) = pp.uv_of(&p).unwrap();
            assert!(p.approx_eq(&pp.point_at_uv(s, u), EPS));
        }

        // 3. 点圆：x² + y² = 0
        let pt_circle = Conic::from_f64s(1.0, 0.0, 1.0, 0.0, 0.0, 0.0);
        let pcp = pt_circle.parametrization(EPS);
        assert_eq!(pcp.kind(), ConicParamKind::TwoLines);
        assert!(pcp.point_at_f64(1.0).approx_eq(&Dot::ZERO, EPS));
        assert!(pt_circle.contains(&pcp.point_at_f64(0.3), EPS));
        assert!(pt_circle.contains(&pcp.point_at_f64(2.5), EPS));
    }

    #[test]
    fn param_degenerate_double_line_and_invalid() {
        // 1. 普通二重直线 (2x - 3y + 5z)² = 0
        let l = Line::from_f64s(2.0, -3.0, 5.0);
        let q = Conic::from_matrix(l.to_cov3c().transpose() * l.to_cov3c());
        let qp = q.parametrization(EPS);
        assert_eq!(qp.kind(), ConicParamKind::DoubleLine);
        assert!(qp.double_line().unwrap().approx_eq(&l, EPS));
        for t in [-10.0, -0.5, 0.0, 0.5, 10.0] {
            let p = qp.point_at_f64(t);
            assert!(l.contains(&p, EPS));
            assert!(q.contains(&p, EPS));
            let t_back = qp.param_of(&p).unwrap();
            assert!((t_back.re - t).abs() < EPS);
        }
        assert!(qp.point_at_f64(f64::INFINITY).is_at_infinity());
        assert!(q.contains(&qp.point_at_f64(f64::INFINITY), EPS));

        // 2. 无穷远二重直线 z² = 0
        let inf2 = Conic::from_f64s(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let ip = inf2.parametrization(EPS);
        assert_eq!(ip.kind(), ConicParamKind::DoubleLine);
        assert!(ip.point_at_f64(0.3).is_at_infinity());
        assert!(inf2.contains(&ip.point_at_f64(0.3), EPS));

        // 3. 零型与极端缩放稳定性
        assert_eq!(
            Conic::ZERO.parametrization(EPS).kind(),
            ConicParamKind::Invalid
        );
        assert!(Conic::ZERO.point_at_f64(0.5, EPS).is_zero());

        let huge = Conic::UNIT_CIRCLE.scaled(c(1e150, 0.0));
        let tiny = Conic::UNIT_CIRCLE.scaled(c(1e-150, 0.0));
        assert!(huge.contains(&huge.point_at_f64(0.5, EPS), EPS));
        assert!(tiny.contains(&tiny.point_at_f64(0.5, EPS), EPS));
    }
}
