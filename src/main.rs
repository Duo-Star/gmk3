// src/main.rs
use eframe::{AppCreator, egui};
use gmk3::app::Gmk3App;
use gmk3::monkey::Monkey;

/// 启动时导入内嵌样例 DSL（文本 ⇄ 中间表示 ⇄ 画布 的演示）。
fn se() -> AppCreator<'static> {
    Box::new(|cc| {
        let monkey = match Monkey::from_dsl(include_str!("../examples/sample.gmk3")) {
            Ok(m) => m,
            Err(diags) => {
                for d in &diags {
                    eprintln!("样例 DSL 第 {} 行第 {} 列: {}", d.line, d.col, d.message);
                }
                panic!("样例 DSL 解析失败");
            }
        };
        Ok(Box::new(Gmk3App::new(cc, monkey)))
    })
}

// 桌面端原生入口
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
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
