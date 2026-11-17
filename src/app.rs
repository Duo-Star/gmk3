use crate::canvas::plotter::Plotter;
use crate::canvas::renderer::{CanvasCallback, CanvasGpuResources};
use eframe::{egui, egui_wgpu};


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
}
impl eframe::App for Gmk3App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t = self.current_t();
        // 1. 上方操作栏（最外层顶部）
        egui::Panel::top("top_bar")
            .show(ui, |ui| {
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
        });

        // 2. 右侧最外层：工具切换栏
        egui::Panel::right("tool_switch").show(ui, |ui| {
            ui.vertical_centered(|ui| {
               // ui.add_space(8.0);
               // ui.label("工\n具\n切\n换");
                ui.button("0")
            });
        });

        // 3. 右侧内层：属性栏（位于工具切换左侧）
        egui::Panel::right("properties_panel").show(ui, |ui| {
            ui.heading("属性栏");
            // 假设这是你结构体中的一个 3x3 矩阵数据，例如 [[f32; 3]; 3]
            // matrix: [[f32; 3]; 3]

            let mut value_changed = false;

            ui.label("视角变换矩阵");

            // 使用 egui::Grid 排列 3x3 的输入框
            egui::Grid::new("matrix_3x3_grid")
                .spacing([8.0, 8.0]) // 设置网格间距
                .show(ui, |ui| {
                    for row in 0..3 {
                        for col in 0..3 {
                            // 每个格子使用 DragValue 或 Slider
                            // 假设你的矩阵字段叫 self.matrix
                            let response = ui.add(
                                egui::DragValue::new(&mut self.matrix[row][col])
                                    .speed(0.01) // 调整拖动灵敏度
                                    .range(-100.0..=100.0) // 可选：限制范围
                            );

                            // 检测当前格子数值是否发生改变
                            if response.changed() {
                                value_changed = true;
                            }
                        }
                        ui.end_row(); // 换行
                    }
                });

            // 当矩阵中任意数值改变时
            if value_changed {

            }
        });

        // 4. 左侧：对象列表栏
        egui::Panel::left("object_list").show(ui, |ui| {
            ui.heading("对象列表");
        });

        // 5. 中间画布区域（上下左右保留 16px 边距，不贴边缘）
        egui::CentralPanel::default().show(ui, |ui| {
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
                self.plotter.camera_mut().pan_by_ndc_delta(dx_ndc, dy_ndc, aspect);
            }
            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y.abs() > 0.0 {
                    self.plotter.camera_mut().zoom_by((scroll_y as f64 * 0.002).exp());
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
        });

        if !self.paused {
            ui.ctx().request_repaint();
        }
    }
}