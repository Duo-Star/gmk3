// src/monkey/solve.rs
//! 解算与绘制：`Monkey::solve` 单遍执行程序并把结果缓存在 `Monkey` 身上；
//! `Monkey::draw_into` 用缓存数据喂给绘图器；`Monkey::collect_labels`
//! 生成对象名标签（本轮仅点）。
//!
//! 解算状态三态：
//! - [`EvalStatus::Ok`]：正常，绘制；
//! - [`EvalStatus::Degenerate`]：退化（重合直线的交点等全零对象），不绘制；
//! - [`EvalStatus::Error`]：出错（含上游错误级联），不绘制。

use crate::canvas::labels::{CanvasLabel, LabelAnchor, LabelKind};
use crate::canvas::plotter::Plotter;
use crate::canvas::styles::{
    ConicStyle, DdotStyle, DotShape, DotStyle, LineShape, LineStyle,
};
use crate::monkey::kinds::{CONSTANTS, GeoObject};
use crate::monkey::method::ArgValue;
use crate::monkey::stmt::{Arg, FIRST_USER_ID};
use crate::monkey::style::{DOT_BASE_RADIUS_PX, LINE_BASE_WIDTH_PX, Shape, StyleSpec};
use crate::monkey::Monkey;

/// 单条语句的解算状态。
#[derive(Clone, Debug, PartialEq)]
pub enum EvalStatus {
    Ok,
    /// 退化（如重合直线的交点、重合点的连线），不绘制。
    Degenerate,
    /// 出错（含上游错误级联），不绘制。
    Error(String),
}

/// 单条语句的解算结果（`Monkey::solve` 后缓存，按下标与语句一一对应）。
#[derive(Clone, Debug)]
pub struct Solved {
    pub status: EvalStatus,
    pub object: Option<GeoObject>,
}

/// 单遍解算：按构造顺序执行全部语句，写入 `monkey.solved`。
///
/// 解析器保证实参只引用先前语句（无前向引用），因此单遍执行即收敛。
pub(crate) fn solve(monkey: &mut Monkey) {
    let stmts = &monkey.stmts;
    let mut results: Vec<Solved> = Vec::with_capacity(stmts.len());

    for stmt in stmts.iter() {
        let def = stmt.method.def();

        // 1. 解析实参：常量 / 先前语句的解算结果
        let mut arg_values: Vec<ArgValue<'_>> = Vec::with_capacity(stmt.args.len());
        let mut upstream_error: Option<String> = None;

        for arg in &stmt.args {
            match arg {
                Arg::Lit(x) => arg_values.push(ArgValue::Lit(*x)),
                Arg::Id(id) => {
                    let obj: &GeoObject = if (id.0 as usize) < CONSTANTS.len() {
                        &CONSTANTS[id.0 as usize].1
                    } else {
                        let prev = (id.0 - FIRST_USER_ID) as usize;
                        match results[prev].object {
                            Some(ref o) => o,
                            None => {
                                upstream_error =
                                    Some(format!("依赖「{}」解算失败", stmts[prev].name));
                                break;
                            }
                        }
                    };
                    arg_values.push(ArgValue::Ref(obj));
                }
            }
        }

        // 2. 执行方法并判定三态
        let result = if let Some(msg) = upstream_error {
            Solved {
                status: EvalStatus::Error(msg),
                object: None,
            }
        } else {
            match (def.eval)(&arg_values) {
                Ok(obj) if obj.is_zero() => Solved {
                    status: EvalStatus::Degenerate,
                    object: Some(obj),
                },
                Ok(obj) => Solved {
                    status: EvalStatus::Ok,
                    object: Some(obj),
                },
                Err(msg) => Solved {
                    status: EvalStatus::Error(msg.to_string()),
                    object: None,
                },
            }
        };
        results.push(result);
    }

    monkey.solved = results;
}

// ---------------------------------------------------------------------------
// 桥接绘制层：StyleSpec → canvas 样式
// ---------------------------------------------------------------------------

/// DSL 点形状 → 画布点形状。
fn dot_shape(s: Shape) -> DotShape {
    match s {
        Shape::Solid => DotShape::Solid,
        Shape::Hollow => DotShape::Hollow,
        Shape::Dashed => DotShape::Cross,
        Shape::Dotted => DotShape::Dotted,
    }
}

/// DSL 线形状 → 画布线形状（本轮仅 `Solid` 参与渲染）。
fn line_shape(s: Shape) -> LineShape {
    match s {
        Shape::Solid => LineShape::Solid,
        Shape::Dashed => LineShape::Dashed,
        Shape::Dotted => LineShape::Dotted,
        Shape::Hollow => LineShape::Hollow,
    }
}

/// 点类样式。
fn dot_style(s: &StyleSpec, dark: bool, label: String, selected: bool) -> DotStyle {
    DotStyle {
        color: s.color.resolve(dark),
        radius_px: DOT_BASE_RADIUS_PX * s.width,
        shape: dot_shape(s.shape),
        label,
        label_visible: s.label_visible,
        selected,
        visible: s.visible,
    }
}

/// 骈点样式（本轮骈点绘制不变：仍为实心圆，形状/选中暂不生效）。
fn ddot_style(s: &StyleSpec, dark: bool, label: String, selected: bool) -> DdotStyle {
    DdotStyle {
        color: s.color.resolve(dark),
        radius_px: DOT_BASE_RADIUS_PX * s.width,
        shape: DotShape::Solid,
        label,
        label_visible: s.label_visible,
        selected,
        visible: s.visible,
    }
}

/// 直线样式（dashed/dotted/hollow 渲染下一轮落地，数据先行）。
fn line_style(s: &StyleSpec, dark: bool, label: String, selected: bool) -> LineStyle {
    LineStyle {
        color: s.color.resolve(dark),
        width_px: LINE_BASE_WIDTH_PX * s.width,
        shape: line_shape(s.shape),
        label,
        label_visible: s.label_visible,
        selected,
        visible: s.visible,
    }
}

/// 圆锥曲线样式（同直线）。
fn conic_style(s: &StyleSpec, dark: bool, label: String, selected: bool) -> ConicStyle {
    ConicStyle {
        color: s.color.resolve(dark),
        width_px: LINE_BASE_WIDTH_PX * s.width,
        shape: line_shape(s.shape),
        label,
        label_visible: s.label_visible,
        selected,
        visible: s.visible,
    }
}

/// 把解算结果提交到 `Plotter`（`Ok` 状态且可见的对象才绘制；不清空缓冲）。
///
/// Qdot / XLine / DXLine 暂不绘制（注册表也不产出）。
pub(crate) fn draw_into(monkey: &Monkey, plotter: &mut Plotter, dark: bool) {
    if monkey.solved.len() != monkey.stmts.len() {
        // 尚未 solve（或缓存失效）：不提交
        return;
    }
    for (i, stmt) in monkey.stmts.iter().enumerate() {
        let solved = &monkey.solved[i];
        if solved.status != EvalStatus::Ok {
            continue;
        }
        if !stmt.style.visible {
            continue;
        }
        let label = stmt.name.clone();
        let selected = stmt.selected;
        let style = &stmt.style;
        match solved.object {
            Some(GeoObject::Dot(d)) => {
                plotter.draw_dot_with(d, dot_style(style, dark, label, selected));
            }
            Some(GeoObject::Ddot(dd)) => {
                plotter.draw_d_dot_with(dd, ddot_style(style, dark, label, selected));
            }
            Some(GeoObject::Line(l)) => {
                plotter.draw_line_with(l, line_style(style, dark, label, selected));
            }
            Some(GeoObject::Conic(c)) => {
                plotter.draw_conic_with(c, conic_style(style, dark, label, selected));
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// 对象名标签（本轮仅点）：参考坐标轴数字标签的 CanvasLabel 管线
// ---------------------------------------------------------------------------

/// 对象名标签的屏幕偏移（pt）：右上方向。
const LABEL_OFFSET_PT: [f32; 2] = [9.0, -9.0];
const LABEL_FONT_SIZE_PT: f32 = 12.0;

/// 生成对象名标签。仅处理点：位置取点的 NDC，右上偏移，锚点左下；
/// 无穷远点 / 画布外 / 不可见 / 标签关闭的对象跳过；**不在灭线处渐隐**。
pub(crate) fn collect_labels(
    monkey: &Monkey,
    plotter: &Plotter,
    viewport_px: [f32; 2],
    dark: bool,
    out: &mut Vec<CanvasLabel>,
) {
    if monkey.solved.len() != monkey.stmts.len() {
        return;
    }
    if viewport_px[0] <= 0.0 || viewport_px[1] <= 0.0 {
        return;
    }
    let aspect = f64::from(viewport_px[0]) / f64::from(viewport_px[1]);
    let (h, _) = plotter.camera().ndc_transforms(aspect);

    for (i, stmt) in monkey.stmts.iter().enumerate() {
        let solved = &monkey.solved[i];
        if solved.status != EvalStatus::Ok || !stmt.style.visible || !stmt.style.label_visible {
            continue;
        }
        let Some(GeoObject::Dot(dot)) = solved.object else {
            continue; // 本轮仅点的名字标签
        };
        let p = h.mul_vec(dot.to_3f64());
        if p[2].abs() <= 1e-12 {
            continue; // 无穷远点
        }
        let x = p[0] / p[2];
        let y = p[1] / p[2];
        if !x.is_finite() || !y.is_finite() || x.abs() > 1.05 || y.abs() > 1.05 {
            continue;
        }
        out.push(CanvasLabel {
            pos_ndc: [x as f32, y as f32],
            offset_pt: LABEL_OFFSET_PT,
            anchor: LabelAnchor::BottomLeft,
            text: stmt.name.clone(),
            color: stmt.style.color.resolve(dark),
            alpha: 1.0, // 不做灭线渐隐
            font_size_pt: LABEL_FONT_SIZE_PT,
            kind: LabelKind::ObjectName,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monkey::stmt::ObjectId;
    use crate::pakoo_forest::geo::{Dot, Line};

    fn sample() -> Monkey {
        let mut monkey =
            Monkey::from_dsl(include_str!("../../examples/sample.gmk3")).expect("样例 DSL 应解析成功");
        monkey.solve();
        monkey
    }

    #[test]
    fn sample_all_ok() {
        let monkey = sample();
        assert_eq!(monkey.solved.len(), 7);
        assert!(
            monkey.solved.iter().all(|s| s.status == EvalStatus::Ok),
            "{:?}",
            monkey.solved
        );
    }

    #[test]
    fn sample_geometry_matches_kernel() {
        let monkey = sample();
        let get = |i: usize| monkey.value_of(ObjectId(5 + i as u64)).expect("对象存在");

        // 与直接调用内核的期望值比对
        let d1 = Dot::from_affine_f64(1.0, 1.0);
        let d2 = Dot::from_affine_f64(1.0, 0.0);
        let c = crate::pakoo_forest::geo::Conic::circle_from_center_point(d1, d2);
        let p = c.point_at_f64(-3.0, 1e-12);
        let l = p.join(d1);
        let x = l.meet(Line::X_AXIS);
        let y = l.meet(Line::Y_AXIS);

        match get(2) {
            GeoObject::Conic(got) => assert!(got.approx_eq(&c, 1e-12)),
            o => panic!("语句 2 应为圆锥曲线，得到 {o:?}"),
        }
        match get(3) {
            GeoObject::Dot(got) => assert!(got.approx_eq(&p, 1e-12)),
            o => panic!("语句 3 应为点，得到 {o:?}"),
        }
        match get(4) {
            GeoObject::Line(got) => assert!(got.approx_eq(&l, 1e-12)),
            o => panic!("语句 4 应为直线，得到 {o:?}"),
        }
        match get(5) {
            GeoObject::Dot(got) => assert!(got.approx_eq(&x, 1e-12)),
            o => panic!("语句 5 应为点，得到 {o:?}"),
        }
        match get(6) {
            GeoObject::Dot(got) => assert!(got.approx_eq(&y, 1e-12)),
            o => panic!("语句 6 应为点，得到 {o:?}"),
        }
    }

    #[test]
    fn queries_by_id_and_name() {
        let monkey = sample();
        // 名字 → ID → 值 / 状态 / 样式
        let id = monkey.id_of("d2").expect("d2 存在");
        assert_eq!(id, ObjectId(6));
        assert_eq!(monkey.name_of(id), "d2");
        assert!(monkey.value_of(id).is_some());
        assert_eq!(monkey.status_of(id), Some(&EvalStatus::Ok));
        let style = monkey.style_of(id).expect("样式存在");
        assert_eq!(style.width, 1.5);
        assert_eq!(style.draggable, Some(false));
        // 内置常量：值总是有效，语句 / 状态 / 样式为 None
        let xa = monkey.id_of("XA").expect("常量存在");
        assert!(matches!(monkey.value_of(xa), Some(GeoObject::Line(_))));
        assert!(monkey.stmt_of(xa).is_none());
        assert!(monkey.status_of(xa).is_none());
        assert!(monkey.style_of(xa).is_none());
    }

    #[test]
    fn degenerate_join_of_coincident_points() {
        let mut monkey =
            Monkey::from_dsl("a d_xy_ff 0 0\nb d_xy_ff 0 0\nl l_join_dd a b\n").expect("解析成功");
        monkey.solve();
        assert_eq!(monkey.solved[0].status, EvalStatus::Ok);
        assert_eq!(monkey.solved[2].status, EvalStatus::Degenerate);
    }

    #[test]
    fn degenerate_meet_of_coincident_lines() {
        let mut monkey = Monkey::from_dsl(
            "a d_xy_ff 0 0\nb d_xy_ff 1 1\nl1 l_join_dd a b\nl2 l_join_dd a b\nx d_meet_ll l1 l2\n",
        )
        .expect("解析成功");
        monkey.solve();
        assert_eq!(monkey.solved[4].status, EvalStatus::Degenerate);
    }

    #[test]
    fn error_cascades_to_dependents() {
        // 圆心不在圆上：切线失败 → Error；依赖它的语句级联 Error
        let mut monkey = Monkey::from_dsl(
            "o d_xy_ff 0 0\np d_xy_ff 3 0\nc c_circle_cp_dd o p\nt l_tangent_dc o c\nx d_meet_ll t XA\n",
        )
        .expect("解析成功");
        monkey.solve();
        assert_eq!(monkey.solved[2].status, EvalStatus::Ok);
        assert!(matches!(monkey.solved[3].status, EvalStatus::Error(_)));
        assert!(matches!(monkey.solved[4].status, EvalStatus::Error(_)));
        assert_eq!(monkey.solved[4].object, None);
    }

    #[test]
    fn drawing_sample_submits_all_objects() {
        let monkey = sample();
        let mut plotter = Plotter::new();
        monkey.draw(&mut plotter, false);
        assert_eq!(plotter.dots().0.len(), 5); // d1 d2 p x y
        assert_eq!(plotter.lines().0.len(), 1); // l
        assert_eq!(plotter.conics().0.len(), 1); // c
    }

    #[test]
    fn hidden_objects_not_drawn() {
        let mut monkey = Monkey::from_dsl("a d_xy_ff 1 2 # hidden\nb d_xy_ff 3 4\n").expect("解析成功");
        monkey.solve();
        let mut plotter = Plotter::new();
        monkey.draw(&mut plotter, false);
        assert_eq!(plotter.dots().0.len(), 1);
    }

    #[test]
    fn dot_labels_collected_for_visible_dots_only() {
        let mut monkey = Monkey::from_dsl(
            "a d_xy_ff 1 1\nb d_xy_ff 3 4 # label_hidden\nc d_xy_ff 0 0 # hidden\n",
        )
        .expect("解析成功");
        monkey.solve();
        let plotter = Plotter::new();
        let mut labels = Vec::new();
        monkey.collect_labels(&plotter, [800.0, 600.0], false, &mut labels);
        // 仅 a 有标签（b 标签关闭，c 对象隐藏）
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "a");
        assert_eq!(labels[0].kind, LabelKind::ObjectName);
        assert_eq!(labels[0].alpha, 1.0);
        assert_eq!(labels[0].offset_pt, LABEL_OFFSET_PT);
    }

    #[test]
    fn selected_stored_on_stmt_not_in_dsl() {
        let mut monkey = Monkey::from_dsl("a d_xy_ff 1 1\n").expect("解析成功");
        assert!(!monkey.stmts()[0].selected, "导入后选中态应复位");
        monkey.set_selected(monkey.id_of("a").unwrap(), true);
        assert!(monkey.stmts()[0].selected);
        let text = monkey.export_dsl();
        assert!(!text.contains("selected"), "selected 不进 DSL：{text}");
        // 再次导入：选中态复位
        let mut again = Monkey::from_dsl(&text).expect("往返解析成功");
        assert!(!again.stmts()[0].selected);
    }

    #[test]
    fn atom_resolves_against_theme() {
        let light = crate::monkey::style::Palette::Atom.resolve(false);
        let dark = crate::monkey::style::Palette::Atom.resolve(true);
        assert!(light[0] < dark[0]); // 亮色主题下 atom 为深色
        assert_eq!(light[3], 1.0);
    }
}
