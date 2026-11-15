use eframe::egui;
use gmk3::app::Gmk3App; // 若 Cargo.toml 中 package.name 不是 gmk3，请改为对应包名

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

    eframe::run_native(
        "gmk3",
        options,
        Box::new(|cc| Ok(Box::new(Gmk3App::new(cc)))),
    )
}

// 网页端 (WebAssembly) 入口
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // 将 Rust 的 panic 信息输出到浏览器的 F12 控制台，方便排查网页端报错
    // console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("无法获取 window 对象")
            .document()
            .expect("无法获取 document 对象");

        let canvas = document
            .get_element_by_id("pakoo_canvas")
            .expect("找不到 id 为 the_canvas_id 的 canvas 元素")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id 不是有效的 HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(Gmk3App::new(cc)))),
            )
            .await;

        if let Err(e) = start_result {
            web_sys::console::error_1(&format!("启动 eframe 失败: {:?}", e).into());
        }
    });
}
