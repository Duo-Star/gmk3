use crate::canvas::camera::Mat3F64;
use crate::canvas::plotter::Plotter;
use crate::canvas::renderer::{CanvasCallback, CanvasGpuResources};
use eframe::{egui, egui_wgpu};
use egui::{vec2, Button, CornerRadius, Image, Ui, Stroke, Sense};

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub struct Gmk3App {
    pub plotter: Plotter,
    update_fn: Box<dyn FnMut(&mut Plotter)>,
    start_time: Instant,
    paused: bool,
    paused_t: f64,
}

fn rounded_square_icon_button(ui: &mut Ui, image: Image<'_>) -> egui::Response {
    let size = vec2(40.0, 40.0); // 1. 设置相等的宽高以形成正方形

    // 2. 调整图片大小，留出内边距
    let img = image.fit_to_exact_size(vec2(24.0, 24.0));

    let button = Button::image(img)
        // 3. 设置圆角半径 (老版本 egui 请使用 .rounding(8.0))
        .corner_radius(CornerRadius::same(8));

    // 4. 使用 add_sized 强制按钮尺寸为正方形
    ui.add_sized(size, button)
}

fn custom_rounded_square_button(ui: &mut Ui) -> egui::Response {
    let size = vec2(40.0, 40.0);
    // 1. 分配正方形区域并注册点击事件
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        // 2. 获取当前交互状态（悬停、按下、默认）对应的主题样式
        let visuals = ui.style().interact(&response);
        let corner_radius = CornerRadius::same(8);

        // 3. 绘制圆角正方形背景和边框
        ui.painter().rect(
            rect,
            corner_radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        // 4. 在中心绘制一个“+”号图标
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

    /// 1. 上方操作栏
    fn draw_top_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("操作栏\n111\n111");
            ui.separator();
            ui.label("gmk3 2026.11.17 duo");
            if ui.button("切换主题").clicked() {
                // 检查当前的 dark_mode 状态
                let is_dark = ui.visuals().dark_mode;

                if is_dark {
                    // 切换为亮色模式
                    ui.ctx().set_visuals(egui::Visuals::light());
                } else {
                    // 切换为暗色模式
                    ui.ctx().set_visuals(egui::Visuals::dark());
                }
            }
        });
    }

    /// 2. 右侧最外层：工具切换栏
    fn draw_tool_switch(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| custom_rounded_square_button(ui));
    }

    /// 3. 右侧内层：属性栏。
    ///
    /// 架构约定：属性值**直接就地绑定**到应用状态（单一数据源），
    /// 不做拷贝——"视角变换矩阵"的 9 个输入框绑定的是
    /// `ProjectiveCamera::proj_matrix` 本体；任何编辑立即作用于相机，
    /// 画布在随后的同一帧内以新矩阵重绘。
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("视图属性");
        ui.separator();

        // 显示开关：直接绑定 Plotter 属性（单一数据源）
        ui.label("显示");
        ui.checkbox(&mut self.plotter.show_grid, "显示网格");
        ui.checkbox(&mut self.plotter.show_axes, "显示坐标轴");
        if self.plotter.show_grid {
            ui.add(
                egui::Slider::new(&mut self.plotter.max_grid_lines, 1024..=65535).text("网格线上限"),
            );
            ui.add(
                egui::Slider::new(&mut self.plotter.grid_extent_margin, 1.0..=65.0)
                    .text("网格范围边距"),
            );
        }
        ui.separator(); // 分割线 -----------------------------------------

        // 直接绑定相机状态（借用自己的字段，非拷贝）
        let camera = self.plotter.camera_mut();
        let mut changed = false;

        ui.label("视角矩阵");

        // 3×3 网格：每个格子绑定 proj_matrix 的一个元素
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

        ui.separator(); // 分割 ---------------------------------------
        ui.label("信息");
        // 只读相机信息（随编辑/拖拽实时刷新）
        ui.label(format!(
            "center: ({:.3}, {:.3})",
            camera.center[0], camera.center[1]
        ));

        ui.label(format!("half_height: {:.3}", camera.half_height));
        ui.add(
            egui::DragValue::new(&mut camera.half_height)
                .speed(0.05)
        );
        ui.label(format!(
            "proj 可逆: {}",
            camera.proj_matrix.inverse().is_some()
        ));

        // 暂停态下编辑矩阵后主动请求一帧重绘，让画布立即反映修改
        if changed && self.paused {
            // ui.ctx().request_repaint();
        }
    }

    /// 4. 左侧：对象列表栏
    fn draw_object_list(&self, ui: &mut egui::Ui) {
        ui.heading("对象列表");
    }

    /// 5. 中间画布区域（上下左右保留 16px 边距，不贴边缘）
    fn draw_canvas(&mut self, ui: &mut egui::Ui, t: f64) {
        let canvas_rect = ui.max_rect().shrink(16.0);
        if canvas_rect.width() <= 2.0 || canvas_rect.height() <= 2.0 {
            return;
        }

        let response = ui.allocate_rect(canvas_rect, egui::Sense::click_and_drag());

        let aspect = (canvas_rect.width() / canvas_rect.height()) as f64;
        if response.dragged() {
            let delta = response.drag_delta();
            let dx_ndc = (delta.x / (0.5 * canvas_rect.width())) as f64;
            let dy_ndc = (-delta.y / (0.5 * canvas_rect.height())) as f64;
            self.plotter
                .camera_mut()
                .pan_by_ndc_delta(dx_ndc, dy_ndc, aspect);
        }
        if response.hovered() {
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.0 {
                self.plotter
                    .camera_mut()
                    .zoom_by((scroll_y as f64 * 0.002).exp());
            }
        }

        // 每帧重算：清空缓冲 -> 设置 t -> 执行命令式几何构造与提交
        let update_fn = &mut self.update_fn;
        self.plotter.update(t, |p| update_fn(p));

        // 黑底 + wgpu 绘制回调 + 细框线
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::BLACK);

        let ppp = ui.ctx().pixels_per_point();
        let viewport_px = [canvas_rect.width() * ppp, canvas_rect.height() * ppp];
        let draw_data = self.plotter.prepare_draw_data(viewport_px, ppp);

        painter.add(egui_wgpu::Callback::new_paint_callback(
            canvas_rect,
            CanvasCallback { data: draw_data },
        ));

        painter.rect_stroke(
            canvas_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(50)),
            egui::StrokeKind::Inside,
        );
    }
}

impl eframe::App for Gmk3App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t = self.current_t();

        // 1. 上方操作栏（最外层顶部）
        egui::Panel::top("top_bar").show(ui, |ui| self.draw_top_bar(ui));

        // 2. 右侧最外层：工具切换栏
        egui::Panel::right("tool_switch").resizable(false)
            .resizable(false)
            .max_size(54.0)
            .show(ui, |ui| self.draw_tool_switch(ui));

        // 3. 右侧内层：属性栏（位于工具切换左侧）
        egui::Panel::right("properties_panel").show(ui, |ui| self.draw_properties_panel(ui));

        // 4. 左侧：对象列表栏
        egui::Panel::left("object_list").show(ui, |ui| self.draw_object_list(ui));

        // 5. 中间画布区域
        egui::CentralPanel::default().show(ui, |ui| self.draw_canvas(ui, t));

        if !self.paused {
            ui.ctx().request_repaint();
        }
    }
}
