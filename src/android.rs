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
use jni::objects::{Global, JClass};
use jni::JValue;
use jni::JavaVM;
use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

/// JavaVM（android_main 里初始化一次），主题切换时用来附加当前线程
static VM: OnceLock<JavaVM> = OnceLock::new();

/// Gmk3Activity 的 JClass 全局引用：在 nativeUpdateInsets 回调里由系统
/// 免费提供并缓存（原生线程上 FindClass 找不到应用类，这是标准做法）
static ACTIVITY_CLASS: OnceLock<Global<JClass<'static>>> = OnceLock::new();

/// 记录 JavaVM，供 set_system_bars 使用
pub fn init_vm(vm: JavaVM) {
    let _ = VM.set(vm);
}

/// 安全区边衬（像素）：[上, 下, 左, 右]。
/// 由 Java 侧 Gmk3Activity 的 OnApplyWindowInsetsListener 通过 JNI 实时更新
/// （状态栏/刘海/手势导航条变化、旋转、导航模式切换都会触发回调）。
static INSETS: [AtomicI32; 4] = [AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];

/// Gmk3Activity.nativeUpdateInsets 的 JNI 实现（由 libgmk3.so 导出，
/// 系统解析 `com.duo.gmk3.Gmk3Activity` 的 native 方法时自动找到）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_duo_gmk3_Gmk3Activity_nativeUpdateInsets(
    mut env: jni::JNIEnv,
    class: JClass,
    top: jni::sys::jint,
    bottom: jni::sys::jint,
    left: jni::sys::jint,
    right: jni::sys::jint,
) {
    // 首次回调时缓存 JClass 全局引用（供反向调用 setSystemBarsStyle 使用）
    if ACTIVITY_CLASS.get().is_none() {
        match env.with_env(|env| env.new_global_ref(&class)).into_outcome() {
            jni::Outcome::Ok(g) => {
                let _ = ACTIVITY_CLASS.set(g);
            }
            jni::Outcome::Err(e) => {
                log::error!("缓存 Gmk3Activity 的 JClass 全局引用失败: {e:?}");
            }
            jni::Outcome::Panic(p) => {
                log::error!("缓存 Gmk3Activity 的 JClass 全局引用时发生 panic: {p:?}");
            }
        }
    }

    let old: Vec<i32> = INSETS.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    if [top, bottom, left, right] != [old[0], old[1], old[2], old[3]] {
        log::info!("gmk3-insets: top={top} bottom={bottom} left={left} right={right}");
    }
    INSETS[0].store(top, Ordering::Relaxed);
    INSETS[1].store(bottom, Ordering::Relaxed);
    INSETS[2].store(left, Ordering::Relaxed);
    INSETS[3].store(right, Ordering::Relaxed);
}

/// 当前安全区边衬（points，另加 4pt 呼吸空间），供 egui 把整个界面收进安全区。
/// 注意 egui::Margin 字段是 i8，截断前先夹取到合理范围。
pub fn safe_margin(pixels_per_point: f32) -> egui::Margin {
    let side = |px: i32| -> i8 {
        let pts = px.max(0) as f32 / pixels_per_point + 4.0;
        pts.round().clamp(0.0, 100.0) as i8
    };
    egui::Margin {
        top: side(INSETS[0].load(Ordering::Relaxed)),
        bottom: side(INSETS[1].load(Ordering::Relaxed)),
        left: side(INSETS[2].load(Ordering::Relaxed)),
        right: side(INSETS[3].load(Ordering::Relaxed)),
    }
}

/// 把系统栏配色同步为 egui 当前主题（面板底色 + 深浅图标）。
///
/// 只在主题切换瞬间调用一次（JNI 调用 + UI 线程消息，微秒级）；
/// 不切换主题时完全不参与，零每帧开销。
pub fn sync_system_bars(ctx: &egui::Context) {
    let visuals = &ctx.global_style().visuals;
    let c = visuals.window_fill;
    let argb = (u32::from(c.a()) << 24)
        | (u32::from(c.r()) << 16)
        | (u32::from(c.g()) << 8)
        | u32::from(c.b());
    set_system_bars(visuals.dark_mode, argb);
}

fn set_system_bars(dark: bool, argb: u32) {
    let Some(vm) = VM.get() else { return };
    let Some(class) = ACTIVITY_CLASS.get() else { return };
    let name = jni::strings::JNIString::from("setSystemBarsStyle");
    let _ = vm.attach_current_thread(|env| -> std::result::Result<(), jni::errors::Error> {
        env.call_static_method(
            class,
            &name,
            jni::jni_sig!("(IZ)V"),
            &[JValue::Int(argb as i32), JValue::Bool(dark)],
        )?;
        Ok(())
    });
    log::info!("gmk3-bars: 已同步系统栏 dark={dark} argb=#{argb:08X}");
}

/// 中文字体兜底：按 Android 官方规范解析各分区的 fonts.xml，
/// 找到 lang="zh-Hans" 的字体文件，以最低优先级装进 egui。
///
/// 背景：eframe 的 `system_fonts`（fontique）只扫描 `/system/fonts` 一个目录，
/// 而部分机型的中文字体位于 `/vendor/fonts`、`/data/fonts`（可更新字体）等位置，
/// 导致中文全部渲染成方框。本函数按 Android 框架 FontManager 的查找顺序
/// 解析 6 个分区的 fonts.xml，取 lang="zh-Hans"（回退 zh-CN / zh）的
/// `<font>` 条目（连同 TTC 的 index 属性），追加到 egui 的 Proportional
/// 字体族队尾：拉丁字符仍走内置字体（行为不变），中文缺字形时才落到这里。
/// 零体积代价（字体来自系统），只在启动时执行一次。
pub fn install_cjk_fallback(ctx: &egui::Context) {
    // (fonts.xml 路径, 字体文件相对目录) —— 顺序与 Android FontManager 一致
    const CONFIGS: &[(&str, &str)] = &[
        ("/system/etc/fonts.xml", "/system/fonts"),
        ("/system_ext/etc/fonts.xml", "/system_ext/fonts"),
        ("/product/etc/fonts.xml", "/product/fonts"),
        ("/vendor/etc/fonts.xml", "/vendor/fonts"),
        ("/odm/etc/fonts.xml", "/odm/fonts"),
        ("/data/fonts/fonts.xml", "/data/fonts/files"),
    ];
    // 语言偏好：精确 zh-Hans 优先，zh-CN / zh 兜底（lang 属性可能是逗号分隔列表）
    const LANG_PREFS: &[&str] = &["zh-Hans", "zh-CN", "zh"];

    struct Cand {
        path: std::path::PathBuf,
        index: u32,
        // 评分越小越好：语言优先级 * 1000 + |weight - 400|
        score: i64,
    }

    let mut best: Option<Cand> = None;

    for (cfg_path, font_dir) in CONFIGS {
        let Ok(xml) = std::fs::read_to_string(cfg_path) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&xml) else {
            continue;
        };
        for family in doc.descendants().filter(|n| n.tag_name().name() == "family") {
            let lang = family.attribute("lang").unwrap_or("");
            let Some(lang_rank) = LANG_PREFS
                .iter()
                .position(|p| lang.split(',').any(|l| l.trim().eq_ignore_ascii_case(p)))
            else {
                continue;
            };
            for font in family.children().filter(|n| n.tag_name().name() == "font") {
                let Some(name) = font.text() else { continue };
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let weight = font
                    .attribute("weight")
                    .and_then(|w| w.parse::<i64>().ok())
                    .unwrap_or(400);
                let index = font
                    .attribute("index")
                    .and_then(|i| i.parse::<u32>().ok())
                    .unwrap_or(0);
                let path = std::path::Path::new(font_dir).join(name);
                if !path.is_file() {
                    continue;
                }
                let score = lang_rank as i64 * 1000 + (weight - 400).abs();
                if best.as_ref().is_none_or(|b| score < b.score) {
                    best = Some(Cand { path, index, score });
                }
            }
        }
    }

    let Some(cand) = best else {
        log::warn!("gmk3-cjk: 未在任何 fonts.xml 中找到 zh-Hans 字体（中文可能显示为方框）");
        return;
    };

    match std::fs::read(&cand.path) {
        Ok(bytes) => {
            ctx.add_font(egui::FontInsert {
                name: "gmk3-cjk".to_owned(),
                data: egui::FontData::from_blob(std::sync::Arc::new(bytes), cand.index),
                families: vec![egui::InsertFontFamily {
                    family: egui::FontFamily::Proportional,
                    priority: egui::FontPriority::Lowest,
                }],
            });
            log::info!(
                "gmk3-cjk: 已安装兜底中文字体 {}（TTC index={}）",
                cand.path.display(),
                cand.index
            );
        }
        Err(e) => log::warn!("gmk3-cjk: 读取 {} 失败: {e}", cand.path.display()),
    }
}

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

    // 记录 JavaVM（主题切换时反向调用 Java 用）
    init_vm(unsafe {
        JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM)
    });

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
