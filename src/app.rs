use crate::canvas::plotter::Plotter;
use crate::canvas::renderer::{CanvasCallback, CanvasGpuResources};
use eframe::{egui, egui_wgpu};
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

        // 1. 上方应用栏（极简）
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("gmk3");
            });
        });

        // 2. 中间画布（上下左右保留 16px 边距，不贴边缘）
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