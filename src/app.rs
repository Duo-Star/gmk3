// src/app.rs

use crate::canvas::camera::Mat3F64;
use crate::canvas::labels::{CanvasLabel, LabelAnchor};
use crate::canvas::plotter::Plotter;
use crate::canvas::renderer::{CanvasCallback, CanvasGpuResources};
use eframe::{egui, egui_wgpu};
use egui::{Button, CornerRadius, Image, Sense, Stroke, Ui, pos2, vec2};

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MobileTab {
    Objects,
    Act,
    Properties,
    Tools,
}

impl MobileTab {
    const ALL: [(MobileTab, &'static str); 4] = [
        (MobileTab::Objects, "对象列表"),
        (MobileTab::Act, "操作栏"),
        (MobileTab::Properties, "属性栏"),
        (MobileTab::Tools, "工具栏"),
    ];
}

pub struct Gmk3App {
    pub plotter: Plotter,
    update_fn: Box<dyn FnMut(&mut Plotter)>,
    start_time: Instant,
    paused: bool,
    paused_t: f64,
    canvas_labels: Vec<CanvasLabel>,
    was_multi_touch: bool,
    mobile_active_tab: Option<MobileTab>,
    mobile_last_tab: Option<MobileTab>,
    mobile_sheet_ratio: f32,
    mobile_switch_anim: f32,
}

fn label_anchor_to_egui(anchor: LabelAnchor) -> egui::Align2 {
    match anchor {
        LabelAnchor::TopLeft => egui::Align2::LEFT_TOP,
        LabelAnchor::TopCenter => egui::Align2::CENTER_TOP,
        LabelAnchor::TopRight => egui::Align2::RIGHT_TOP,
        LabelAnchor::CenterLeft => egui::Align2::LEFT_CENTER,
        LabelAnchor::CenterCenter => egui::Align2::CENTER_CENTER,
        LabelAnchor::CenterRight => egui::Align2::RIGHT_CENTER,
        LabelAnchor::BottomLeft => egui::Align2::LEFT_BOTTOM,
        LabelAnchor::BottomCenter => egui::Align2::CENTER_BOTTOM,
        LabelAnchor::BottomRight => egui::Align2::RIGHT_BOTTOM,
    }
}

fn draw_canvas_labels(painter: &egui::Painter, rect: egui::Rect, labels: &[CanvasLabel]) {
    let w = rect.width();
    let h = rect.height();
    for label in labels {
        if label.alpha <= 0.001 {
            continue;
        }
        let px = rect.left() + (label.pos_ndc[0] * 0.5 + 0.5) * w + label.offset_pt[0];
        let py = rect.top() + (0.5 - label.pos_ndc[1] * 0.5) * h + label.offset_pt[1];

        let base = egui::Color32::from_rgba_unmultiplied(
            (label.color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (label.color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (label.color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (label.color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        );
        let color = base.gamma_multiply(label.alpha);

        painter.text(
            pos2(px, py),
            label_anchor_to_egui(label.anchor),
            &label.text,
            egui::FontId::proportional(label.font_size_pt),
            color,
        );
    }
}

#[allow(dead_code)]
fn rounded_square_icon_button(ui: &mut Ui, image: Image<'_>) -> egui::Response {
    let size = vec2(40.0, 40.0);
    let img = image.fit_to_exact_size(vec2(24.0, 24.0));
    let button = Button::image(img).corner_radius(CornerRadius::same(8));
    ui.add_sized(size, button)
}

fn custom_rounded_square_button(ui: &mut Ui) -> egui::Response {
    let size = vec2(40.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let corner_radius = CornerRadius::same(8);

        ui.painter().rect(
            rect,
            corner_radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let center = rect.center();
        let half_len = 8.0;
        let icon_stroke = Stroke::new(2.0, visuals.fg_stroke.color);

        ui.painter().line_segment(
            [center - vec2(half_len, 0.0), center + vec2(half_len, 0.0)],
            icon_stroke,
        );
        ui.painter().line_segment(
            [center - vec2(0.0, half_len), center + vec2(0.0, half_len)],
            icon_stroke,
        );
    }

    response
}

impl Gmk3App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        update_fn: impl FnMut(&mut Plotter) + 'static,
    ) -> Self {
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let gpu_res = CanvasGpuResources::new(&render_state.device, render_state.target_format);
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(gpu_res);
        }

        Self {
            plotter: Plotter::new(),
            update_fn: Box::new(update_fn),
            start_time: Instant::now(),
            paused: false,
            paused_t: 0.0,
            canvas_labels: Vec::with_capacity(48),
            was_multi_touch: false,
            mobile_active_tab: None,
            mobile_last_tab: None,
            mobile_sheet_ratio: 0.3,
            mobile_switch_anim: 1.0,
        }
    }

    fn current_t(&self) -> f64 {
        if self.paused {
            self.paused_t
        } else {
            self.start_time.elapsed().as_secs_f64()
        }
    }

    // ------------------------------------------------------------------
    // 各面板：单一职责；`ui()` 只做组合
    // ------------------------------------------------------------------

    /// 1. 上方栏
    fn draw_top_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("gmk3 2026.11.17 duo");
            if ui.button("切换主题").clicked() {
                let is_dark = ui.visuals().dark_mode;
                if is_dark {
                    ui.ctx().set_visuals(egui::Visuals::light());
                } else {
                    ui.ctx().set_visuals(egui::Visuals::dark());
                }
            }
        });
    }

    fn draw_act_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("操作栏");
        });
    }

    /// 2. 右侧最外层：工具切换栏
    fn draw_tool_switch(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label("工具栏");
            custom_rounded_square_button(ui)
        });
    }

    /// 3. 右侧内层：属性栏
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("属性栏");
        ui.separator();

        ui.label("显示");
        ui.checkbox(&mut self.plotter.show_grid, "显示网格");
        ui.checkbox(&mut self.plotter.show_axes, "显示坐标轴");
        ui.separator();

        let camera = self.plotter.camera_mut();
        let mut changed = false;

        ui.label("视角矩阵");

        egui::Grid::new("proj_matrix_grid")
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                for row in 0..3 {
                    for col in 0..3 {
                        let response = ui.add(
                            egui::DragValue::new(&mut camera.proj_matrix.m[row][col])
                                .speed(0.005)
                                .range(-100.0..=100.0),
                        );
                        changed |= response.changed();
                    }
                    ui.end_row();
                }
            });

        if ui.button("重置视角矩阵").clicked() {
            camera.proj_matrix = Mat3F64::IDENTITY;
            changed = true;
        }

        ui.separator();
        ui.label("信息");
        ui.label(format!(
            "center: ({:.3}, {:.3})",
            camera.center[0], camera.center[1]
        ));

        ui.label(format!("half_height: {:.3}", camera.half_height));
        ui.add(egui::DragValue::new(&mut camera.half_height).speed(0.05));
        ui.label(format!(
            "proj 可逆: {}",
            camera.proj_matrix.inverse().is_some()
        ));

        if changed && self.paused {
            ui.ctx().request_repaint();
        }
    }

    /// 4. 左侧：对象列表栏
    fn draw_object_list(&self, ui: &mut egui::Ui) {
        ui.heading("对象列表000000");
    }

    // ------------------------------------------------------------------
    // 竖屏专属：下方窄条选择栏 + 向上展开的动画覆盖面板
    // ------------------------------------------------------------------

    fn draw_mobile_bottom_bar(&mut self, ui: &mut egui::Ui) {
        let accent = if ui.visuals().dark_mode {
            egui::Color32::from_rgb(170, 94, 122)
        } else {
            egui::Color32::from_rgb(170, 94, 122)
        };

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let tab_w = (ui.available_width() / 4.0).max(1.0);
            let tab_h = 36.0;

            for (tab, label) in MobileTab::ALL {
                let (rect, resp) = ui.allocate_exact_size(vec2(tab_w, tab_h), Sense::click());
                let is_active = self.mobile_active_tab == Some(tab);

                if ui.is_rect_visible(rect) {
                    if is_active {
                        ui.painter().rect_filled(
                            rect.shrink(3.0),
                            CornerRadius::same(8),
                            accent.gamma_multiply(0.16),
                        );
                        // let indicator = egui::Rect::from_min_size(
                        //     pos2(rect.center().x - 14.0, rect.top() + 1.0),
                        //     vec2(28.0, 2.5),
                        // );
                        // ui.painter()
                        //     .rect_filled(indicator, CornerRadius::same(2), accent);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(
                            rect.shrink(3.0),
                            CornerRadius::same(8),
                            ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.5),
                        );
                    }

                    let text_color = if is_active {
                        accent
                    } else if resp.hovered() {
                        ui.visuals().strong_text_color()
                    } else {
                        ui.visuals().text_color()
                    };

                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(14.0),
                        text_color,
                    );
                }

                if resp.clicked() {
                    if self.mobile_active_tab == Some(tab) {
                        self.mobile_active_tab = None;
                    } else {
                        if self.mobile_active_tab.is_some() {
                            self.mobile_switch_anim = 0.0;
                        }
                        self.mobile_active_tab = Some(tab);
                        self.mobile_last_tab = Some(tab);
                    }
                }
            }
        });
    }

    fn compute_mobile_sheet_geometry(
        &mut self,
        ctx: &egui::Context,
        screen_rect: egui::Rect,
        top_bar_bottom: f32,
        bottom_bar_top: f32,
    ) -> Option<(egui::Rect, f32, f32, f32, f32)> {
        let open_t = ctx.animate_bool_with_time(
            egui::Id::new("mobile_sheet_open"),
            self.mobile_active_tab.is_some(),
            0.20,
        );

        if self.mobile_switch_anim < 1.0 {
            let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.05);
            self.mobile_switch_anim = (self.mobile_switch_anim + dt / 0.14).min(1.0);
            ctx.request_repaint();
        }

        if open_t <= 0.001 && self.mobile_active_tab.is_none() {
            self.mobile_last_tab = None;
            return None;
        }

        let ease = open_t * open_t * (3.0 - 2.0 * open_t);
        let max_h = (bottom_bar_top - top_bar_bottom - 12.0).max(96.0);
        let min_h = 96.0_f32.min(max_h);
        let target_h = (self.mobile_sheet_ratio * screen_rect.height()).clamp(min_h, max_h);
        let cur_h = (target_h * ease).max(1.0);

        let rect = egui::Rect::from_min_size(
            pos2(screen_rect.left(), bottom_bar_top - cur_h),
            vec2(screen_rect.width(), cur_h),
        );
        Some((rect, ease, target_h, min_h, max_h))
    }

    fn draw_mobile_sheet(
        &mut self,
        ctx: &egui::Context,
        screen_rect: egui::Rect,
        geom: (egui::Rect, f32, f32, f32, f32),
    ) {
        let Some(tab) = self.mobile_last_tab else {
            return;
        };
        let (rect, ease, target_h, min_h, max_h) = geom;

        egui::Area::new(egui::Id::new("mobile_bottom_sheet"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(rect);
                ui.set_opacity(ease);

                let border_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
                let frame = egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .corner_radius(CornerRadius {
                        nw: 14,
                        ne: 14,
                        sw: 0,
                        se: 0,
                    })
                    .stroke(Stroke::new(1.0, border_color))
                    .inner_margin(egui::Margin::symmetric(12, 6));

                frame.show(ui, |ui| {
                    let inner_w = (rect.width() - 24.0).max(0.0);
                    let inner_h = (rect.height() - 12.0).max(0.0);
                    ui.set_min_size(vec2(inner_w, inner_h));
                    ui.set_max_size(vec2(inner_w, inner_h));

                    // 上边缘拖拽调节高度把手
                    let (handle_rect, handle_resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::drag());

                    if handle_resp.hovered() || handle_resp.dragged() {
                        ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if handle_resp.dragged() {
                        let new_h = (target_h - handle_resp.drag_delta().y).clamp(min_h, max_h);
                        self.mobile_sheet_ratio =
                            (new_h / screen_rect.height().max(1.0)).clamp(0.10, 0.90);
                    }

                    if ui.is_rect_visible(handle_rect) {
                        let pill_color = if handle_resp.dragged() || handle_resp.hovered() {
                            ui.visuals().strong_text_color().gamma_multiply(0.75)
                        } else {
                            ui.visuals().text_color().gamma_multiply(0.35)
                        };
                        let pill =
                            egui::Rect::from_center_size(handle_rect.center(), vec2(38.0, 4.0));
                        ui.painter()
                            .rect_filled(pill, CornerRadius::same(2), pill_color);
                    }

                    ui.separator();

                    // 切换栏目时的轻微淡入上浮过渡
                    let sw = self.mobile_switch_anim;
                    let sw_ease = 1.0 - (1.0 - sw).powi(3);
                    ui.set_opacity(ease * sw_ease);
                    let y_shift = (1.0 - sw_ease) * 8.0;
                    if y_shift > 0.01 {
                        ui.add_space(y_shift);
                    }

                    let avail_h = ui.available_height().max(0.0);
                    egui::ScrollArea::vertical()
                        .max_height(avail_h)
                        .auto_shrink([false, false])
                        .show(ui, |ui| match tab {
                            MobileTab::Objects => self.draw_object_list(ui),
                            MobileTab::Act => self.draw_act_bar(ui),
                            MobileTab::Properties => self.draw_properties_panel(ui),
                            MobileTab::Tools => {
                                self.draw_tool_switch(ui);
                            }
                        });
                });
            });
    }

    /// 5. 中间画布区域（桌面端保留 16px 边距；竖屏无空隙贴合边缘）
    fn draw_canvas(
        &mut self,
        ui: &mut egui::Ui,
        t: f64,
        is_portrait: bool,
        blocking_rect: Option<egui::Rect>,
    ) {
        let canvas_rect = if is_portrait {
            ui.max_rect()
        } else {
            ui.max_rect().shrink(16.0)
        };
        if canvas_rect.width() <= 2.0 || canvas_rect.height() <= 2.0 {
            return;
        }

        let response = ui.allocate_rect(canvas_rect, egui::Sense::click_and_drag());

        let aspect = (canvas_rect.width() / canvas_rect.height()) as f64;
        let pos_to_ndc = |pos: egui::Pos2| -> [f64; 2] {
            let fx = ((pos.x - canvas_rect.center().x) / (0.5 * canvas_rect.width())) as f64;
            let fy = (-(pos.y - canvas_rect.center().y) / (0.5 * canvas_rect.height())) as f64;
            [fx, fy]
        };

        let pointer_blocked = blocking_rect
            .is_some_and(|r| ui.input(|i| i.pointer.latest_pos().is_some_and(|p| r.contains(p))));

        // 1. 移动端多指手势（双指捏合缩放 + 双指拖动平移）
        let multi_touch = ui.input(|i| i.multi_touch());
        let multi_touch_active = match multi_touch {
            Some(mt)
                if mt.num_touches >= 2
                    && !blocking_rect
                        .is_some_and(|r| r.contains(mt.start_pos) || r.contains(mt.center_pos))
                    && (canvas_rect.contains(mt.start_pos)
                        || canvas_rect.contains(mt.center_pos)
                        || response.dragged()) =>
            {
                let dx_ndc = (mt.translation_delta.x / (0.5 * canvas_rect.width())) as f64;
                let dy_ndc = (-mt.translation_delta.y / (0.5 * canvas_rect.height())) as f64;
                if dx_ndc.abs() > 0.0 || dy_ndc.abs() > 0.0 {
                    self.plotter
                        .camera_mut()
                        .pan_by_ndc_delta(dx_ndc, dy_ndc, aspect);
                }

                let zoom = mt.zoom_delta as f64;
                if zoom.is_finite() && zoom > 0.0 && (zoom - 1.0).abs() > 1e-5 {
                    let focus_ndc = pos_to_ndc(mt.center_pos);
                    self.plotter
                        .camera_mut()
                        .zoom_by_at_ndc(zoom, focus_ndc, aspect);
                }
                true
            }
            _ => false,
        };

        // 2. 桌面端鼠标右键：转动射影视角（直接更新 proj_matrix）
        if !multi_touch_active
            && !pointer_blocked
            && response.dragged_by(egui::PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            let dx_ndc = (2.6 * delta.x / (0.5 * canvas_rect.width())) as f64;
            let dy_ndc = (2.6 * (-delta.y) / (0.5 * canvas_rect.height())) as f64;
            self.plotter
                .camera_mut()
                .tilt_by_ndc_delta(dx_ndc, dy_ndc, aspect);
        }
        // 3. 左键/单指/中键拖动：平移画布（屏蔽双指期间及刚抬起一指的过渡帧）
        else if !multi_touch_active
            && !self.was_multi_touch
            && !pointer_blocked
            && (response.dragged_by(egui::PointerButton::Primary)
                || response.dragged_by(egui::PointerButton::Middle))
        {
            let delta = response.drag_delta();
            let dx_ndc = (delta.x / (0.5 * canvas_rect.width())) as f64;
            let dy_ndc = (-delta.y / (0.5 * canvas_rect.height())) as f64;
            self.plotter
                .camera_mut()
                .pan_by_ndc_delta(dx_ndc, dy_ndc, aspect);
        }

        self.was_multi_touch = multi_touch_active;

        // 4. 桌面端滚轮 / 笔记本触控板缩放（以光标为中心）
        if !multi_touch_active && !pointer_blocked && response.hovered() {
            let (scroll_y, pinch_zoom, hover_pos) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.pointer.hover_pos(),
                )
            });
            let focus_ndc = hover_pos.map(pos_to_ndc).unwrap_or([0.0, 0.0]);

            if scroll_y.abs() > 0.0 {
                self.plotter.camera_mut().zoom_by_at_ndc(
                    (scroll_y as f64 * 0.002).exp(),
                    focus_ndc,
                    aspect,
                );
            } else if pinch_zoom > 0.0 && (pinch_zoom - 1.0).abs() > 1e-5 {
                self.plotter
                    .camera_mut()
                    .zoom_by_at_ndc(pinch_zoom as f64, focus_ndc, aspect);
            }
        }

        // 每帧重算：清空缓冲 -> 设置 t -> 执行命令式几何构造与提交
        let update_fn = &mut self.update_fn;
        self.plotter.update(t, |p| update_fn(p));

        // 黑底 + wgpu 绘制回调 + 坐标文字 + 细框线
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::BLACK);

        let ppp = ui.ctx().pixels_per_point();
        let viewport_px = [canvas_rect.width() * ppp, canvas_rect.height() * ppp];
        let draw_data = self.plotter.prepare_draw_data(viewport_px, ppp);
        self.plotter
            .prepare_labels_into(viewport_px, ppp, &mut self.canvas_labels);

        painter.add(egui_wgpu::Callback::new_paint_callback(
            canvas_rect,
            CanvasCallback { data: draw_data },
        ));

        draw_canvas_labels(&painter, canvas_rect, &self.canvas_labels);

        if !is_portrait {
            painter.rect_stroke(
                canvas_rect,
                0.0,
                Stroke::new(1.0, egui::Color32::from_gray(50)),
                egui::StrokeKind::Inside,
            );
        }
    }
}

impl eframe::App for Gmk3App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t = self.current_t();
        let screen_rect = ui.max_rect();
        let is_portrait = screen_rect.height() > screen_rect.width();

        if is_portrait {
            // 1. 顶部栏（与桌面端一致）
            let top_resp = egui::Panel::top("top_bar").show(ui, |ui| self.draw_top_bar(ui));

            // 2. 底部窄条选择栏（对象列表 / 操作栏 / 属性栏 / 工具栏）
            let bottom_resp = egui::Panel::bottom("mobile_bottom_bar")
                .resizable(false)
                .show(ui, |ui| self.draw_mobile_bottom_bar(ui));

            let sheet_geom = self.compute_mobile_sheet_geometry(
                ui.ctx(),
                screen_rect,
                top_resp.response.rect.bottom(),
                bottom_resp.response.rect.top(),
            );
            let blocking_rect = sheet_geom.map(|(r, ..)| r);

            // 3. 中间最下层绘图区（无空隙）
            egui::CentralPanel::no_frame()
                .show(ui, |ui| self.draw_canvas(ui, t, true, blocking_rect));

            // 4. 上浮覆盖层（从底栏上边缘向上展开）
            if let Some(geom) = sheet_geom {
                self.draw_mobile_sheet(ui.ctx(), screen_rect, geom);
            }
        } else {
            // 桌面端布局
            egui::Panel::top("top_bar").show(ui, |ui| self.draw_top_bar(ui));
            egui::Panel::top("act_bar").show(ui, |ui| self.draw_act_bar(ui));

            egui::Panel::right("tool_switch")
                .resizable(false)
                .max_size(54.0)
                .show(ui, |ui| self.draw_tool_switch(ui));

            egui::Panel::right("properties_panel").show(ui, |ui| self.draw_properties_panel(ui));

            egui::Panel::left("object_list").show(ui, |ui| self.draw_object_list(ui));

            egui::CentralPanel::default().show(ui, |ui| self.draw_canvas(ui, t, false, None));
        }

        if !self.paused {
            ui.ctx().request_repaint();
        }
    }
}
