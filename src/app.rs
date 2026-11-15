use crate::canvas::camera::ProjectiveCamera;
use crate::canvas::renderer::{CanvasCallback, CanvasGpuResources};
use crate::canvas::scene::CanvasScene;
use eframe::egui;
use eframe::egui_wgpu;

pub struct Gmk3App {
    pub camera: ProjectiveCamera,
    pub scene: CanvasScene,
}

impl Gmk3App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let gpu_res = CanvasGpuResources::new(&render_state.device, render_state.target_format);
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(gpu_res);
        }

        Self {
            camera: ProjectiveCamera::default(),
            scene: CanvasScene::demo(),
        }
    }
}

impl eframe::App for Gmk3App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 更新秒制时间 t
        let time = std::time::Instant::now();



        // 1. 上方应用栏
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

            // 鼠标拖拽平移 / 滚轮缩放
            let aspect = (canvas_rect.width() / canvas_rect.height()) as f64;
            if response.dragged() {
                let delta = response.drag_delta();
                let dx_ndc = (delta.x / (0.5 * canvas_rect.width())) as f64;
                let dy_ndc = (-delta.y / (0.5 * canvas_rect.height())) as f64;
                self.camera.pan_by_ndc_delta(dx_ndc, dy_ndc, aspect);
            }
            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y.abs() > 0.0 {
                    self.camera.zoom_by((scroll_y as f64 * 0.002).exp());
                }
            }


            // 黑底 + 细框线
            let painter = ui.painter_at(canvas_rect);
            painter.rect_filled(canvas_rect, 0.0, egui::Color32::BLACK);

            // wgpu 自定义绘制回调
            let ppp = ui.ctx().pixels_per_point();
            let viewport_px = [canvas_rect.width() * ppp, canvas_rect.height() * ppp];
            let draw_data = self.camera.prepare_draw_data(&self.scene, viewport_px, ppp);

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
    }
}