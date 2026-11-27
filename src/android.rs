// src/android.rs
//! Android（NativeActivity）入口。
//!
//! android-activity 的 native-activity glue 会在 `ANativeActivity_onCreate`
//! 中启动一条专用线程并调用本文件的 `android_main`；
//! Java 侧只需一个 `android.app.NativeActivity`（`android.app.lib_name = "gmk3"`）。

#![cfg(target_os = "android")]

use android_activity::AndroidApp;
use crate::app::Gmk3App;
use crate::monkey::Monkey;
use eframe::egui;
use std::ffi::{c_void, CStr};
use std::time::Duration;

/// 在 libandroid.so / libnativewindow.so 中查找符号。
///
/// `ANativeWindow_setFrameRate`（API 30+）在不同 Android 版本上位于不同库：
/// API < 35 时在 libandroid.so，API ≥ 35 时 libnativewindow.so 恢复为独立库。
/// 又因为 NDK r30 的 API<35 sysroot 已移除 libnativewindow.so（无法静态链接），
/// 所以统一在运行时用 dlsym 解析。
unsafe fn dlsym_any(name: &CStr) -> Option<*mut c_void> {
    for lib in [c"libandroid.so", c"libnativewindow.so"] {
        let handle = unsafe { libc::dlopen(lib.as_ptr(), libc::RTLD_LAZY) };
        if handle.is_null() {
            continue;
        }
        let sym = unsafe { libc::dlsym(handle, name.as_ptr()) };
        if !sym.is_null() {
            return Some(sym);
        }
    }
    None
}

/// 申请 120fps 高刷。
///
/// 背景：Android 上非白名单应用的 NativeActivity 窗口默认被系统限在 60fps
/// （各厂商的“高刷白名单”，浏览器等通常被放行，所以同样内容在浏览器里反而流畅）。
/// 应用自己主动调用 `ANativeWindow_setFrameRate(120, FixedSource)` 即可解除限制。
///
/// 窗口要等 eframe 收到 Resume 事件后才创建（native_window() 在
/// InitWindow/TerminateWindow 之间才返回 Some），所以起一个线程轮询等待。
/// ANativeWindow 的 API 线程安全，跨线程设置没有问题。
fn request_high_refresh(app: &AndroidApp) {
    let app = app.clone();
    std::thread::spawn(move || {
        for _ in 0..1000 {
            let Some(window) = app.native_window() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };

            // ANativeWindow_setFrameRate(ANativeWindow*, float frameRate, int8_t compatibility)
            // compatibility: 1 = ANATIVEWINDOW_FRAME_RATE_COMPATIBILITY_FIXED_SOURCE
            type SetFrameRateFn = unsafe extern "C" fn(*mut c_void, f32, i8) -> i32;
            let Some(sym) = (unsafe { dlsym_any(c"ANativeWindow_setFrameRate") }) else {
                log::warn!("系统库中找不到 ANativeWindow_setFrameRate（需要 Android 11+）");
                return;
            };
            let set_frame_rate: SetFrameRateFn = unsafe { std::mem::transmute(sym) };
            let ret = unsafe { set_frame_rate(window.ptr().as_ptr().cast(), 120.0, 1) };
            if ret == 0 {
                log::info!("已申请 120fps 高刷（ANativeWindow_setFrameRate, FixedSource）");
            } else {
                log::warn!("ANativeWindow_setFrameRate 返回错误码 {ret}");
            }
            return;
        }
        log::warn!("10 秒内未等到 ANativeWindow，未设置帧率");
    });
}

/// Android 主入口：Java 侧加载 `libgmk3.so` 后调用。
///
/// 签名约定见 `android-activity` 文档：必须是导出、不混名的
/// `fn android_main(app: AndroidApp)`（默认 Rust ABI）。
#[unsafe(no_mangle)]
pub fn android_main(app: AndroidApp) {
    // 把 Rust 侧日志转发到 logcat（标签 gmk3）
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("gmk3"),
    );
    log::info!("gmk3 android_main 启动");

    // 高刷屏上把窗口帧率从默认的 60fps 提到 120fps（见函数注释）
    request_high_refresh(&app);

    // 导入内嵌样例 DSL（与桌面端一致）
    let monkey = match Monkey::from_dsl(include_str!("../examples/sample.gmk3")) {
        Ok(m) => m,
        Err(diags) => {
            for d in &diags {
                log::error!("样例 DSL 第 {} 行第 {} 列: {}", d.line, d.col, d.message);
            }
            panic!("样例 DSL 解析失败");
        }
    };

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("gmk3")
            .with_inner_size([1100.0, 760.0]),
        // Android 必需：把 Activity 交给 winit 事件循环
        android_app: Some(app),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "gmk3",
        options,
        Box::new(move |cc| Ok(Box::new(Gmk3App::new(cc, monkey)))),
    ) {
        log::error!("eframe 启动失败: {e:?}");
    }
}
