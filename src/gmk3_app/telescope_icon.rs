//
use crate::pakoo_forest::geo::{Conic, Dot, Line};
use eframe::{egui, egui_wgpu};
use egui::{
    Align2, Button, Color32, CornerRadius, FontId, Image, Pos2, Response, Sense,
    Shape::{self, Path},
    Stroke, StrokeKind, Ui, pos2, vec2,
};
use std::f32::consts::TAU;

//
pub fn telescope_line_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: Color32,
    line: Line,
    r: f32,
) {
    let stroke = Stroke::new(1.6, color);

    let is_at_infinity: bool = line.is_at_infinity();
    let is_real: bool = line.is_real();
    if is_at_infinity {
        // 无穷远直线
        // 绘制虚线圆
        let segments: i32 = 12;
        let radius: f32 = r * 0.9;
        let circle_points: Vec<Pos2> = (0..=segments)
            .map(|i: i32| {
                let angle: f32 = (i as f32 / segments as f32) * TAU;
                center + vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        let shapes: Vec<Shape> = Shape::dashed_line(&circle_points, stroke, 2.0, 2.0);
        painter.extend(shapes);
        painter.text(
            center,
            Align2::CENTER_CENTER,
            "∞",
            FontId::proportional(r * 1.5),
            Color32::WHITE,
        );
    } else {
        if is_real {
            // 有穷实直线
            // 绘制示意方向
            let [a, b] = line.to_2f64(); // 齐次线坐标
            let dir: egui::Vec2 = vec2(a as f32, -b as f32).rot90().normalized();
            let dir: egui::Vec2 = if dir.length_sq() > 0.0 {
                dir
            } else {
                vec2(1.0, -1.0).normalized()
            };
            let offset: egui::Vec2 = dir * r;
            painter.line_segment([center - offset, center + offset], stroke);
        } else {
            // 有穷虚直线
            // 绘制虚斜45直线
            let shapes = Shape::dashed_line(
                &vec![
                    center + vec2(-r * 0.9, r * 0.9),
                    center - vec2(-r * 0.9, r * 0.9),
                ],
                stroke,
                1.2,
                1.8,
            );
            painter.extend(shapes);
            painter.text(
                center,
                Align2::LEFT_TOP,
                "img",
                FontId::proportional(6.0),
                Color32::WHITE,
            );
        }
    }
}
