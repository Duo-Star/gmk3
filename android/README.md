# gmk3 Android 工程

Rust 核心（仓库根目录）编译为 `libgmk3.so`，由本目录的最小 Gradle 工程
打包成 APK。纯命令行即可完成，Android Studio 只是可选工具。

## 目录结构

```
gmk3/                       # 仓库根：Rust 源码（cargo 工程）
├── src/android.rs          # android_main 入口（NativeActivity 加载）
└── android/                # 本目录：Android 壳工程
    ├── dev.sh              # 一键开发脚本（编译→打包→安装→运行→日志）
    ├── settings.gradle.kts # Gradle 工程设置（用 AS 打开本目录即可）
    ├── gradlew             # Gradle Wrapper（锁定 9.9.0，本地与 CI 一致）
    └── app/
        ├── build.gradle.kts# minSdk 24 / compileSdk 36 / debug 签名
        └── src/main/
            ├── AndroidManifest.xml       # NativeActivity + lib_name=gmk3
            └── jniLibs/<abi>/libgmk3.so  # 构建产物（已 gitignore）
```

## 日常开发（真机 adb 调试）

```bash
android/dev.sh              # 一键：编译 so → 打包 → 安装 → 启动 → 跟随日志
android/dev.sh build        # 只编译 + 打包
android/dev.sh install      # 只安装
android/dev.sh run          # 只启动
android/dev.sh logcat       # 只跟随日志
android/dev.sh crash        # 抓最近崩溃栈并符号化到源码行
android/dev.sh coooool      # 发布版装手机：无调试信息 + 极致优化 + 最小体积
```

脚本会自动探测手机架构（`adb shell getprop ro.product.cpu.abi`），
只编译对应 ABI；没连设备时才同时编 arm64-v8a 与 x86_64。

### 两个编译 profile

| profile | 用途 | 特点 |
|---------|------|------|
| `release`（默认） | 日常开发调试 | 带行号表，崩溃可用 `dev.sh crash` 还原到源码行；.so 较大（~95MB） |
| `dist`（coooool） | 发布 / 演示 | 无调试信息 + 剥离符号 + thin LTO + codegen-units=1 + panic=abort；体积最小性能最佳（.so ~15MB 级） |

注意：`coooool` 装的是剥离符号的精简版，之后 `dev.sh crash` 会拒绝工作
并提示先用 `dev.sh build` 装回调试版。两个 profile 的编译缓存相互独立，
首次 `coooool` 需要完整编译一遍（几分钟），之后增量很快。

## 命令行打包（不开 Android Studio）

```bash
# 1) Rust → .so（NDK 交叉编译）
cargo ndk -t arm64-v8a -t x86_64 -P 24 \
    -o android/app/src/main/jniLibs build --release --lib

# 2) Gradle 打包 APK
cd android && ./gradlew assembleDebug
# 产物：android/app/build/outputs/apk/debug/app-debug.apk
```

`assembleRelease` 也是 debug 签名（见 build.gradle.kts），个人自用足够；
以后上架 Play 再换正式 keystore。

## 用 Android Studio 打开（可选）

直接用 AS **打开 `android/` 目录**即可（Gradle 工程根）。
Rust 侧改动后 AS 里重新点 Run 不会自动重编 .so——先在命令行跑
`android/dev.sh build`，再在 AS 里 Run，或直接用 dev.sh。

## 调试

- 日志：`adb logcat -s gmk3:*`（Rust 侧 log 已转发 logcat，标签 gmk3）
- 性能：logcat 里每 3 秒一条 `gmk3-perf: FPS ... 平均帧时 ... solve+draw ...`
  （Android 专用探针，见 app.rs；平均帧时只有零点几毫秒却掉帧，说明瓶颈
  不在计算而在系统帧率策略）
- 崩溃：`android/dev.sh crash`（依赖 Cargo.toml 里 release 的
  `line-tables-only` 调试信息 + NDK 的 llvm-addr2line）
- 帧时序：`adb shell dumpsys SurfaceFlinger --latency <图层名>`（图层名用
  `dumpsys SurfaceFlinger --list | grep gmk3` 找）
- 截图：`adb exec-out screencap -p > screen.png`

## 常见问题

- **`adb devices` 显示 unauthorized**：手机上点「允许 USB 调试」
- **安装报 INSTALL_FAILED_USER_RESTRICTED**：vivo/小米等需在开发者
  选项里打开「USB 安装」
- **应用黑屏**：看 logcat；渲染后端为 wgpu（优先 Vulkan，自动回退 GLES）
- **刘海/状态栏/手势条遮挡界面**：已按现代标准处理——Edge-to-Edge
  （targetSdk 36 系统强制，低版本在 Gmk3Activity.java 里兼容）+ 
  WindowInsets 动态避让。Java 侧监听 `systemBars + displayCutout` 边衬，
  JNI 回调给 Rust，egui 把整个界面收进安全区；旋转/切换导航模式自动跟随。
  诊断：logcat 里 `gmk3-insets: top=.. bottom=.. left=.. right=..`
- **高刷屏上掉帧/只有 60fps**：Android 厂商把非白名单应用的窗口默认限在
  60fps（浏览器在白名单里所以反而流畅）。本工程已在 `src/android.rs` 里
  通过 `ANativeWindow_setFrameRate(120, FixedSource)` 主动申请 120fps 解决。
  若个别机型仍被限制，可在系统「屏幕刷新率 → 应用高刷名单」里手动加白。
  注意：NDK r30 的 API<35 sysroot 移除了 libnativewindow.so，所以这个调用
  是运行时 dlsym 解析的（详见 android.rs 注释）
- **部分机型中文显示为方框**：eframe 的 system_fonts（fontique）只扫描
  `/system/fonts`，而不少机型的中文字体在 `/vendor/fonts`、`/data/fonts`
  （可更新字体）等位置。本工程在 `src/android.rs` 的 `install_cjk_fallback`
  里按 Android 官方规范解析 6 个分区的 fonts.xml，取 `lang="zh-Hans"` 的
  字体（含 TTC index）以最低优先级装进 egui 兜底：零体积、零运行时开销。
  诊断：logcat 里 `gmk3-cjk: 已安装兜底中文字体 <路径>` 与
  `gmk3-cjk: 中文字形覆盖检查: true/false` 两条日志

## GitHub Actions 自动发布 APK

每次推送 `master` 都会自动构建发布版 APK（`.github/workflows/android.yml`，
与 Vercel 的 web 部署 workflow 并行互不影响）：

- 编译：Rust 交叉编译双架构 .so（dist 发布配置 = 无调试 + thin LTO 极致优化）
- 打包：`assembleRelease`（debug 签名，免密钥管理），ABI 拆分一次性产出三个 APK：
  - `gmk3-android-arm64-v8a.apk`：仅真机（最小，推荐）
  - `gmk3-android-x86_64.apk`：仅模拟器
  - `gmk3-android-universal.apk`：双架构合一
- 发布：滚动 **latest** Release（每次覆盖旧的），另存一份 workflow artifact
- 产物：仓库 Releases 页面 → 最新一条，下载 `gmk3-android.apk` 即可分发
- 手动触发：仓库 Actions → Build Android APK → Run workflow
- 首次构建约 20–40 分钟（要下载 NDK + 全量编译），之后有缓存约 5–10 分钟

## 环境依赖（本机已装）

rustup android 目标、NDK r30、cargo-ndk、JDK 17+、Gradle 9.9、
Android SDK（build-tools / platform-tools / platforms）。
