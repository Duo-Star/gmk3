// src/main.rs
use eframe::{AppCreator, egui};
use gmk3::app::Gmk3App;
use gmk3::canvas::plotter::Plotter;
use gmk3::pakoo_forest::geo::{Conic, Dot};


fn se() -> AppCreator<'static> {
    Box::new(|cc| {
        Ok(Box::new(Gmk3App::new(cc, |p: &mut Plotter| {
            let t = p.t();

            let c = Conic::from_f64s(1.0, 0.0, 0.0, 0.0, -1.0, -1.0);
            let d = Dot::from_f64s(1.0+(0.2*t).sin(),0.0+(0.2*t).cos(),1.0);
            let l = c.polar(&d);
            let dp = c.intersect_line(&l);
            // let cen = dp.center();
            let pp = c.point_at_f64(t.tan(),1e-12);

            p.draw_conic(c);
            p.draw_line(l);
            p.draw_dot(d);
            p.draw_d_dot(dp);
            // p.draw_dot(cen);
            p.draw_dot(pp);


            // p.camera_mut().proj_matrix.m[2][0] = 2.0 * t.sin();
            // p.camera_mut().proj_matrix.m[2][1] = 2.0 * (0.5*t).cos();
            let _ = t;
        })))
    })
}

// 桌面端原生入口
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // println!(
    //     "{}",
    //     Conic::UNIT_CIRCLE.intersect_line(&Line::from_f64s(1.0, 0.0, -2.0))
    // );
    println!("{}", Conic::UNIT_CIRCLE);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("gmk3")
            .with_inner_size([1100.0, 760.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native("gmk3", options, se())
}
// 网页端 (WebAssembly) 入口
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // 把 Rust 的 log / panic 转发到浏览器控制台
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    // 网页端没有 NativeOptions，使用 WebOptions 默认会在可用时优先使用 wgpu，否则自动回退到 WebGL
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("没有 window")
            .document()
            .expect("没有 document");

        let canvas = document
            .get_element_by_id("pakoo_canvas")
            .expect("找不到 id 为 the_canvas_id 的 canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id 不是一个 HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(canvas, web_options, se())
            .await;

        let loading_text = document.get_element_by_id("loading_text");
        match start_result {
            Ok(_) => {
                if let Some(el) = loading_text {
                    el.remove();
                }
            }
            Err(e) => {
                if let Some(el) = loading_text {
                    el.set_inner_html("<p>应用崩溃了，请查看浏览器控制台。</p>");
                }
                panic!("启动 eframe 应用失败: {e:?}");
            }
        }
    });
}
