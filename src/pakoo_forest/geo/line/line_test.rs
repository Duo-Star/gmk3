// src/pakoo_forest/geo/line/line_test.rs

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::pakoo_forest::calc::{Complex, Cov3C};
    use crate::pakoo_forest::geo::{Dot, Line};

    const EPS: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    // ---- 构造与访问 ----

    #[test]
    fn construction_access() {
        let l = Line::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0));
        assert_eq!(l.x(), c(1.0, 2.0));
        assert_eq!(l.y(), c(3.0, -4.0));
        assert_eq!(l.z(), c(0.0, 5.0));
        assert_eq!(
            Line::from_f64s(1.0, 2.0, 3.0),
            Line::new(c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0))
        );
        assert_eq!(
            Line::from_cov3c(Cov3C::from_f64s(1.0, 2.0, 3.0)),
            Line::from_f64s(1.0, 2.0, 3.0)
        );
        assert_eq!(Line::INF, Line::from_f64s(0.0, 0.0, 1.0));
        assert_eq!(l.to_array(), [c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0)]);
        assert_eq!(
            l.to_cov3c(),
            Cov3C::new(c(1.0, 2.0), c(3.0, -4.0), c(0.0, 5.0))
        );
        // 过两点的直线
        let p = Dot::from_f64s(0.0, 0.0, 1.0);
        let q = Dot::from_f64s(1.0, 0.0, 1.0);
        assert_eq!(Line::from_points(&p, &q), p.join(q));
    }

    // ---- 平行 ----

    #[test]
    fn parallel() {
        let l = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let m = Line::from_f64s(0.0, 2.0, 3.0); // 2y + 3z = 0，平行
        assert!(l.is_pall(m, 0.0));
        assert!(m.is_pall(l, 0.0));
        // 尺度无关
        assert!(l.is_pall(l.scaled(c(2.0, 0.0)), 0.0));
        // 无穷远直线与一切直线平行
        assert!(Line::INF.is_pall(l, 0.0));
        assert!(l.is_pall(Line::INF, 0.0));
        // 不平行的例子
        assert!(!l.is_pall(Line::from_f64s(1.0, 0.0, 0.0), EPS));
        // 平行直线的交点在无穷远直线上
        let meet = l.meet(m);
        assert!(meet.is_at_infinity());
    }

    // ---- 垂直 ----

    #[test]
    fn perpendicular() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        assert!(x_axis.is_vert(y_axis, 0.0));
        assert!(y_axis.is_vert(x_axis, 0.0));
        let l = Line::from_f64s(1.0, 1.0, 0.0);
        let m = Line::from_f64s(1.0, -1.0, 0.0);
        assert!(l.is_vert(m, 0.0));
        assert!(!l.is_vert(Line::from_f64s(1.0, 0.0, 0.0), EPS));
        // 尺度无关
        assert!(x_axis.scaled(c(3.0, 0.0)).is_vert(y_axis, 0.0));
        // 无穷远直线与一切直线垂直（由绝对定义的约定）
        assert!(Line::INF.is_vert(x_axis, 0.0));
        // 垂线与自身的无穷远点：调和四点由虚圆点定义（I、J 与 l⊥ 的无穷远点共轭配对）
        assert_eq!(Dot::I.conj(), Dot::J);
    }

    // ---- 两线交点 ----

    #[test]
    fn meet() {
        let x_axis = Line::from_f64s(0.0, 1.0, 0.0); // y = 0
        let y_axis = Line::from_f64s(1.0, 0.0, 0.0); // x = 0
        // 坐标轴交于齐次原点（射影相等）
        let origin = x_axis.meet(y_axis);
        assert!(origin.approx_eq(&Dot::ZERO, 0.0));
        assert!(origin.is_on_line(x_axis, 0.0));
        assert!(origin.is_on_line(y_axis, 0.0));
        // 平行直线相交于无穷远点，且该点同时在两直线上
        let l = Line::from_f64s(0.0, 1.0, 3.0);
        let m = Line::from_f64s(0.0, 1.0, -2.0);
        let meet = l.meet(m);
        assert!(meet.is_at_infinity());
        assert!(meet.is_on_line(l, 0.0));
        assert!(meet.is_on_line(m, 0.0));
        // 相同直线 → 无效点
        assert!(l.meet(l).is_zero());
        // 与无穷远直线的交点 == 方向无穷远点
        let g = Line::from_f64s(2.0, 3.0, 1.0);
        assert!(Line::INF.meet(g).approx_eq(&g.inf_point(), 0.0));
        assert!(Line::INF.meet(g).approx_eq(&Dot::inf(g), 0.0));
    }

    // ---- 关联判定 / 配对 ----

    #[test]
    fn contains_evaluate() {
        let p0 = Dot::from_f64s(0.0, 0.0, 1.0);
        let p1 = Dot::from_f64s(1.0, 2.0, 1.0);
        let l = p0.join(p1);
        // 精确判定
        assert!(l.contains(&p0, 0.0));
        assert!(l.contains(&p1, 0.0));
        assert!(l.contains(&p0.scaled(c(1.0, 2.0)), 0.0));
        assert_eq!(l * p0, Complex::ZERO); // 运算符 == evaluate
        assert_eq!(l.evaluate(&p1), Complex::ZERO);
        // 不包含
        assert!(!l.contains(&Dot::from_f64s(0.0, 1.0, 1.0), EPS));
        // 容差版：l: −2x + y = 0；点 (1e-10, 3e-10, 1) 偏离 1e-10
        let near = Dot::from_f64s(1e-10, 3e-10, 1.0);
        assert!(l.contains(&near, 1e-9));
        assert!(!l.contains(&near, 1e-12));
    }

    // ---- 方向无穷远点 ----

    #[test]
    fn inf_point() {
        let l = Line::from_f64s(2.0, 3.0, 1.0);
        let inf = l.inf_point();
        assert_eq!(inf, Dot::from_f64s(3.0, -2.0, 0.0));
        assert!(inf.is_at_infinity());
        assert!(inf.is_on_line(l, 0.0));
        assert!(inf.is_on_line(Line::INF, 0.0));
    }

    // ---- 射影相等 / 代表元运算 ----

    #[test]
    fn projective_ops() {
        let l = Line::new(c(3.0, 1.0), c(0.0, -2.0), c(1.0, 0.5));
        // 与整体缩放无关（含复数缩放；非精确表示的比例因子需极小容差）
        assert!(l.approx_eq(&l.scaled(c(-2.0, 0.0)), 0.0));
        assert!(l.approx_eq(&l.scaled(c(0.3, -0.7)), EPS));
        let n = l.normalize().unwrap();
        assert!((n.norm() - 1.0).abs() < EPS);
        assert!(n.approx_eq(&l, EPS));
        assert_eq!(Line::from_f64s(0.0, 0.0, 0.0).normalize(), None);
        // 共轭 / 缩放
        assert_eq!(l.conj(), Line::new(c(3.0, -1.0), c(0.0, 2.0), c(1.0, -0.5)));
        assert_eq!(
            l.scaled(c(2.0, 0.0)),
            Line::from_cov3c(l.to_cov3c().mul_complex(c(2.0, 0.0)))
        );
    }

    // ---- 点到直线距离 ----

    #[test]
    fn distance_to_dot() {
        // 直线 x = 0 到点 (3, 4, 1) 的距离为 3，到直线上点的距离为 0
        let y_axis = Line::Y_AXIS; // x = 0
        let p = Dot::from_f64s(3.0, 4.0, 1.0);
        let on_line = Dot::from_f64s(0.0, 5.0, 1.0);

        assert!((y_axis.distance_f64(p) - 3.0).abs() < EPS);
        assert!((y_axis.distance_complex(p) - c(3.0, 0.0)).abs() < EPS);
        assert!((y_axis.distance_sqr_complex(p) - c(9.0, 0.0)).abs() < EPS);
        assert!(y_axis.distance_complex(on_line).abs() < EPS);
        assert!(y_axis.distance_sqr_complex(on_line).abs() < EPS);
        assert!(y_axis.distance_f64(on_line).abs() < EPS);

        // 经典斜线：3x + 4y - 10 = 0 到原点 (0, 0, 1) 的距离为 |-10| / 5 = 2
        let l = Line::from_f64s(3.0, 4.0, -10.0);
        assert!((l.distance_f64(Dot::ZERO) - 2.0).abs() < EPS);
        assert!((l.distance_sqr_complex(Dot::ZERO) - c(4.0, 0.0)).abs() < EPS);

        // 齐次尺度不变性：缩放直线或点不影响距离模长与复平方距离
        let l_scaled = l.scaled(c(2.0, -1.0));
        let p_scaled = p.scaled(c(0.5, 1.5));
        assert!((l_scaled.distance_f64(p_scaled) - l.distance_f64(p)).abs() < EPS);
        assert!((l_scaled.distance_sqr_complex(p_scaled) - l.distance_sqr_complex(p)).abs() < EPS);

        // 复数点到实直线：x = 0 到点 (1 + 2i, 0, 1)
        let cp = Dot::from_affine(c(1.0, 2.0), Complex::ZERO);
        assert!((y_axis.distance_complex(cp) - c(1.0, 2.0)).abs() < EPS);
        assert!((y_axis.distance_sqr_complex(cp) - c(-3.0, 4.0)).abs() < EPS);
        assert!((y_axis.distance_f64(cp) - 5.0_f64.sqrt()).abs() < EPS);

        // 无穷远点 / 无穷远直线 / 迷向退化直线 (a^2 + b^2 = 0) 均返回非有限值
        assert!(!l.distance_f64(Dot::I).is_finite());
        assert!(!l.distance_complex(Dot::I).is_finite());
        assert!(!l.distance_sqr_complex(Dot::I).is_finite());
        assert!(!Line::INF.distance_f64(p).is_finite());
        assert!(!Line::INF.distance_complex(p).is_finite());
        assert!(!Line::INF.distance_sqr_complex(p).is_finite());

        let isotropic = Line::new(Complex::ONE, Complex::I, Complex::ZERO); // x + iy = 0, a^2 + b^2 = 0
        assert!(!isotropic.distance_f64(p).is_finite());
        assert!(!isotropic.distance_complex(p).is_finite());
        assert!(!isotropic.distance_sqr_complex(p).is_finite());
    }

    // ---- 谓词 / 输出 ----
    #[test]
    fn predicates_display() {
        assert!(Line::INF.is_at_infinity());
        assert!(!Line::from_f64s(1.0, 0.0, 0.0).is_at_infinity());
        assert!(Line::from_f64s(0.0, 0.0, 0.0).is_zero());
        assert!(!Line::INF.is_zero());
        assert!(Line::from_f64s(1.0, 2.0, 3.0).is_real());
        assert!(!Line::new(Complex::I, Complex::ZERO, Complex::ZERO).is_real());
        assert!(Line::from_f64s(1.0, 2.0, 3.0).is_finite());
        assert!(!Line::new(c(f64::INFINITY, 0.0), Complex::ZERO, Complex::ZERO).is_finite());
        // 输出
        assert_eq!(
            Line::from_f64s(1.0, 2.0, 3.0).to_string(),
            "Line:[1 : 2 : 3]"
        );
        assert_eq!(Line::INF.to_string(), "Line:[0 : 0 : 1]");
    }
}
