// src/pakoo_forest/geo/dot/dot_test.rs

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::pakoo_forest::calc::Complex;
    use crate::pakoo_forest::geo::Dot;
    use crate::pakoo_forest::geo::Line;

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let p = Dot::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0));
        assert_eq!(p.x(), c(1.0, 2.0));
        assert_eq!(p.y(), c(3.0, -4.0));
        assert_eq!(p.z(), c(0.0, 5.0));
        assert_eq!(
            Dot::from_f64s(1.0, 2.0, 3.0),
            Dot::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(Dot::from_vec3c(p.p), p);
        assert_eq!(
            Dot::from_affine(c(1.0, 2.0), c(3.0, 4.0)),
            Dot::new(c(1.0, 2.0), c(3.0, 4.0), Complex::ONE)
        );
        assert_eq!(
            Dot::ZERO,
            Dot::new(Complex::ZERO, Complex::ZERO, Complex::ONE)
        );
        assert_eq!(p.to_array(), [c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0)]);
        assert_eq!(Dot::default(), Dot::ZERO);
        // 仿射坐标往返
        assert_eq!(
            Dot::from_f64s(2.0, 4.0, 2.0).to_affine(),
            Some((c(1.0, 0.0), c(2.0, 0.0)))
        );
        assert_eq!(
            Dot::from_affine(c(1.0, 0.0), c(2.0, 0.0)).to_affine(),
            Some((c(1.0, 0.0), c(2.0, 0.0)))
        );
        // 无穷远点无仿射坐标
        assert_eq!(Dot::I.to_affine(), None);
    }

    // ---- 虚圆点 ----

    #[test]
    fn circular_points() {
        assert_eq!(Dot::I, Dot::new(Complex::ONE, Complex::I, Complex::ZERO));
        assert_eq!(
            Dot::J,
            Dot::new(Complex::ONE, Complex::NEG_I, Complex::ZERO)
        );
        // 共轭互换两个虚圆点
        assert_eq!(Dot::I.conj(), Dot::J);
        assert_eq!(Dot::J.conj(), Dot::I);
        // 虚圆点在无穷远直线上
        assert!(Dot::I.is_at_infinity());
        assert!(Dot::J.is_at_infinity());
        assert_eq!(Line::INF * Dot::I, Complex::ZERO);
        assert_eq!(Line::INF * Dot::J, Complex::ZERO);
        // 两个虚圆点的连线是无穷远直线（差一个标量因子）
        assert!(Dot::I.join(Dot::J).is_pall(Line::INF, 0.0));
    }

    // ---- 两点连线 ----

    #[test]
    fn join() {
        // (0:0:1) 与 (1:0:1) 决定直线 y = 0
        let l = Dot::from_f64s(0.0, 0.0, 1.0).join(Dot::from_f64s(1.0, 0.0, 1.0));
        assert_eq!(
            (l.x(), l.y(), l.z()),
            (Complex::ZERO, Complex::ONE, Complex::ZERO)
        );
        // 生成点精确地在结果线上
        assert!(l.contains(&Dot::from_f64s(0.0, 0.0, 1.0), 0.0));
        assert!(l.contains(&Dot::from_f64s(1.0, 0.0, 1.0), 0.0));
        assert!(l.contains(&Dot::from_f64s(-3.0, 0.0, 1.0), 0.0));
        assert!(!l.contains(&Dot::from_f64s(0.0, 1.0, 1.0), 0.0));
        // 交换律：join(p, q) 与 join(q, p) 是同一直线（反号）
        let p = Dot::new(c(1.0, 0.0), c(2.0, -1.0), c(3.0, 0.5));
        let q = Dot::from_f64s(4.0, -2.0, 1.0);
        assert!(p.join(q).approx_eq(&q.join(p), 0.0));
    }

    // ---- 关联判定与尺度无关性 ----

    #[test]
    fn incidence_scale_invariance() {
        let p = Dot::new(c(1.0, 1.0), c(2.0, -1.0), c(3.0, 0.5));
        let q = Dot::from_f64s(4.0, -2.0, 1.0);
        let l = p.join(q);
        assert!(p.is_on_line(l, 0.0));
        assert!(q.is_on_line(l, 0.0));
        // 点/线整体缩放不影响关联性
        assert!(
            p.scaled(c(2.0, 3.0))
                .is_on_line(l.scaled(c(0.5, -0.25)), 0.0)
        );
        // 配对运算符 == evaluate
        assert_eq!(l * p, l.evaluate(&p));
        assert_eq!(l * p, Complex::ZERO);
        // 不关联的点
        let r = Dot::from_f64s(0.0, 0.0, 1.0);
        assert!(!r.is_on_line(l, EPS));
    }

    // ---- 两线交点 ----

    #[test]
    fn meet() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        // 坐标轴交于齐次原点（射影相等，分量可能反号）
        let origin = x_axis.meet(y_axis);
        assert!(origin.approx_eq(&Dot::ZERO, 0.0));
        assert!(origin.is_on_line(x_axis, 0.0));
        assert!(origin.is_on_line(y_axis, 0.0));
        // 与无穷远直线相交 → 该直线的无穷远点
        let l = Line::from_f64s(1.0, 2.0, 3.0);
        let inf_meet = Line::INF.meet(l);
        assert!(inf_meet.is_at_infinity());
        assert!(inf_meet.approx_eq(&Dot::inf(l), 0.0));
        // 过同一点的两直线交于该点
        let a = Dot::from_f64s(1.0, 2.0, 1.0);
        let b = Dot::from_f64s(-1.0, 3.0, 2.0);
        let c3 = Dot::from_f64s(5.0, 0.5, 4.0);
        let l1 = a.join(b);
        let l2 = a.join(c3);
        assert!(l1.meet(l2).approx_eq(&a, EPS));
    }

    // ---- 三点共线 ----

    #[test]
    fn collinear() {
        let p = Dot::from_f64s(0.0, 0.0, 1.0);
        let p0 = Dot::from_f64s(1.0, 0.0, 1.0);
        let p1 = Dot::from_f64s(2.0, 0.0, 1.0);
        assert!(p.is_collinear(p0, p1, 0.0));
        assert!(p.is_collinear(p1, p0, 0.0)); // 顺序无关
        assert!(p.scaled(c(2.0, 0.0)).is_collinear(p0, p1, 0.0)); // 尺度无关
        let q = Dot::from_f64s(0.0, 1.0, 1.0);
        assert!(!p.is_collinear(p0, q, EPS));
        // 重合点视为共线
        assert!(p.is_collinear(p, q, 0.0));
    }

    // ---- 无穷远方向点 ----

    #[test]
    fn inf_direction() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        assert_eq!(Dot::inf(x_axis), Dot::from_f64s(1.0, 0.0, 0.0));
        assert_eq!(Dot::inf(y_axis), Dot::from_f64s(0.0, -1.0, 0.0));
        assert_eq!(x_axis.inf_point(), Dot::inf(x_axis));
        // 方向点在直线上、且在无穷远直线上
        let inf = Dot::inf(x_axis);
        assert!(inf.is_at_infinity());
        assert!(inf.is_on_line(x_axis, 0.0));
        assert!(inf.is_on_line(Line::INF, 0.0));
    }

    // ---- 射影相等 / 代表元运算 ----

    #[test]
    fn projective_equality() {
        let p = Dot::new(c(1.0, 0.5), c(-2.0, 1.0), c(3.0, 0.0));
        // 与整体缩放无关（含复数缩放与反号）
        assert!(p.approx_eq(&p.scaled(c(2.0, -1.0)), 0.0));
        assert!(p.approx_eq(&p.scaled(c(-1.0, 0.0)), 0.0));
        assert!(p.approx_eq(&p.scaled(Complex::I), 0.0));
        let q = Dot::new(c(1.1, 0.5), c(-2.0, 1.0), c(3.0, 0.0));
        assert!(p.approx_eq(&q, 1e-1));
        assert!(!p.approx_eq(&q, 1e-6));
    }

    #[test]
    fn normalize_scaled_conj() {
        let p = Dot::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        let n = p.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert!(n.approx_eq(&p, EPS)); // 归一化不改变射影点
        // 无效点无法归一化
        assert_eq!(Dot::from_f64s(0.0, 0.0, 0.0).normalize(), None);
        // 共轭 / 缩放
        assert_eq!(p.conj(), Dot::new(c(3.0, -1.0), c(0.0, 2.0), c(1.0, -0.5)));
        assert_eq!(
            p.scaled(c(2.0, 0.0)),
            Dot::from_vec3c(p.p.mul_complex(c(2.0, 0.0)))
        );
    }

    // ---- 两点距离 ----
    #[test]
    fn distance_to_dot() {
        let p = Dot::from_f64s(1.0, 2.0, 1.0);
        let q = Dot::from_f64s(4.0, 6.0, 1.0);

        // 实点基准：sqrt(3^2 + 4^2) = 5，复平方距离为 25 + 0i，复距离主值为 5 + 0i
        assert!((p.distance_f64(&q) - 5.0).abs() < EPS);
        assert!((p.distance_sqr_complex(&q) - c(25.0, 0.0)).abs() < EPS);
        assert!((p.distance_complex(&q) - c(5.0, 0.0)).abs() < EPS);

        // 同点距离归零
        assert!(p.distance_f64(&p).abs() < EPS);
        assert!(p.distance_sqr_complex(&p).abs() < EPS);
        assert!(p.distance_complex(&p).abs() < EPS);

        // 齐次坐标非单位 z 分量与复数缩放不变性
        let p_h = Dot::from_f64s(2.0, 4.0, 2.0).scaled(c(1.0, -1.0));
        let q_h = Dot::from_f64s(12.0, 18.0, 3.0).scaled(c(-0.5, 2.0));
        assert!((p_h.distance_f64(&q_h) - 5.0).abs() < EPS);
        assert!((p_h.distance_sqr_complex(&q_h) - c(25.0, 0.0)).abs() < EPS);

        // 一般复数仿射坐标：dx = 1 + i, dy = 0
        // |dx| = sqrt(2)，但 dx^2 = (1 + i)^2 = 2i
        let o = Dot::from_affine(Complex::ZERO, Complex::ZERO);
        let w = Dot::from_affine(c(1.0, 1.0), Complex::ZERO);
        assert!((o.distance_f64(&w) - 2.0_f64.sqrt()).abs() < EPS);
        assert!((o.distance_sqr_complex(&w) - c(0.0, 2.0)).abs() < EPS);
        assert!((o.distance_complex(&w) - c(0.0, 2.0).sqrt()).abs() < EPS);

        // 迷向差向量（dx = 1, dy = i）：dx^2 + dy^2 = 1 + (-1) = 0，但实模长为 sqrt(2)
        let iso_target = Dot::from_affine(c(1.0, 0.0), c(0.0, 1.0));
        assert!(o.distance_sqr_complex(&iso_target).abs() < EPS);
        assert!(o.distance_complex(&iso_target).abs() < EPS);
        assert!((o.distance_f64(&iso_target) - 2.0_f64.sqrt()).abs() < EPS);

        // 无穷远点退化为非有限值
        assert!(!p.distance_f64(&Dot::I).is_finite());
        assert!(!p.distance_sqr_complex(&Dot::I).is_finite());
        assert!(!p.distance_complex(&Dot::I).is_finite());
        assert!(!Dot::I.distance_f64(&Dot::J).is_finite());
        assert!(!Dot::I.distance_sqr_complex(&Dot::J).is_finite());
    }

    // ---- 中点 ----

    #[test]
    fn midpoint() {
        let a = Dot::from_f64s(1.0, 2.0, 1.0);
        let b = Dot::from_f64s(5.0, 6.0, 1.0);
        assert!(a.mid(&b).approx_eq(&Dot::from_f64s(3.0, 4.0, 1.0), EPS));

        // 齐次缩放不变性
        let a_s = a.scaled(c(2.0, -1.0));
        let b_s = b.scaled(c(-1.5, 0.5));
        assert!(a_s.mid(&b_s).approx_eq(&Dot::from_f64s(3.0, 4.0, 1.0), EPS));
    }

    // ---- 谓词 / 输出 ----

    #[test]
    fn predicates_display() {
        assert!(Dot::from_f64s(0.0, 0.0, 0.0).is_zero());
        assert!(!Dot::ZERO.is_zero()); // ZERO 是齐次原点 (0:0:1)，不是无效点
        assert!(Dot::I.is_at_infinity());
        assert!(!Dot::from_f64s(1.0, 2.0, 1.0).is_at_infinity());
        assert!(Dot::from_f64s(1.0, 2.0, 1.0).is_real());
        assert!(!Dot::I.is_real());
        assert!(Dot::from_f64s(1.0, 2.0, 1.0).is_finite());
        assert!(!Dot::new(c(f64::NAN, 0.0), Complex::ZERO, Complex::ZERO).is_finite());
        // 输出
        assert_eq!(Dot::from_f64s(1.0, 2.0, 3.0).to_string(), "Dot:(1 : 2 : 3)");
        assert_eq!(Dot::ZERO.to_string(), "Dot:(0 : 0 : 1)");
        assert_eq!(Dot::I.to_string(), "Dot:(1 : 1i : 0)");
    }
}
