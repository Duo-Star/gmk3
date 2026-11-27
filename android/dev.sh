#!/usr/bin/env bash
# ============================================================
# gmk3 Android 一键开发脚本
#
# 用法（在仓库任意位置执行）：
#   android/dev.sh          编译 .so → 打包 APK → 安装到手机 → 启动 → 跟随日志
#   android/dev.sh build    只编译 .so + 打包 APK（debug 包，日常开发用）
#   android/dev.sh coooool  发布版装手机：无调试信息 + 极致优化 + 最小体积
#   android/dev.sh install  只安装 debug APK（需先 build）
#   android/dev.sh run      只启动应用
#   android/dev.sh logcat   只跟随应用日志（标签 gmk3）
#   android/dev.sh crash    抓取最近的崩溃栈并符号化到源码行
#
# 依赖：rustup 的 android 目标、NDK、cargo-ndk、JDK、Gradle、adb
# ============================================================
set -euo pipefail

# 脚本所在目录（android/）与仓库根目录
ANDROID_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$ANDROID_DIR/.." && pwd)"

PACKAGE="com.duo.gmk3"
ACTIVITY=".Gmk3Activity"
JNI_DIR="$ANDROID_DIR/app/src/main/jniLibs"
APK_DEBUG="$ANDROID_DIR/app/build/outputs/apk/debug/app-debug.apk"
APK_RELEASE="$ANDROID_DIR/app/build/outputs/apk/release/app-release.apk"

# NDK 路径：优先 ANDROID_NDK_HOME，其次常见位置
NDK="${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/30.0.16248370}"
ADDR2LINE="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-addr2line"
LLVM_NM="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm"

say()  { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m错误:\033[0m %s\n' "$*" >&2; exit 1; }

# ------------------------------------------------------------------
# 探测已连接设备的 CPU 架构，只编译对应的 ABI（省一半编译时间）；
# 没有设备连上时两个架构都编（保证 APK 通用）。
# ------------------------------------------------------------------
detect_abi() {
    if command -v adb >/dev/null 2>&1 && adb devices | grep -qw "device"; then
        local abi
        abi="$(adb shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r')"
        case "$abi" in
            arm64-v8a|x86_64) echo "$abi"; return ;;
        esac
    fi
    echo "arm64-v8a x86_64"
}

# ------------------------------------------------------------------
# 1) Rust → libgmk3.so
#    cargo-ndk 自动设置交叉编译器/链接器；-P 24 对应 minSdk 24。
#    两个 profile（见 Cargo.toml）：
#      release  日常开发：带行号表，崩溃可用 addr2line 还原源码位置（.so 较大）
#      dist     发布演示：无调试信息 + strip + thin LTO，体积最小性能最佳
# ------------------------------------------------------------------
build_so() {
    local profile="${1:-release}"
    local targets=($(detect_abi))
    say "编译 Rust → .so（profile=$profile，目标: ${targets[*]}）"
    # 清掉旧架构产物：保证 APK 里只有本次编译的 ABI（体积最小）
    rm -rf "$JNI_DIR"/arm64-v8a "$JNI_DIR"/x86_64
    local ndk_args=()
    for t in "${targets[@]}"; do ndk_args+=(-t "$t"); done
    if [ "$profile" = "release" ]; then
        (cd "$ROOT_DIR" && cargo ndk "${ndk_args[@]}" -P 24 -o "$JNI_DIR" build --release --lib)
    else
        (cd "$ROOT_DIR" && cargo ndk "${ndk_args[@]}" -P 24 -o "$JNI_DIR" build --profile "$profile" --lib)
    fi
}

# ------------------------------------------------------------------
# 2) Gradle 打包 APK（aapt2 打包 manifest+资源+jniLibs → apksigner 签名 → zipalign）
#    Debug 包：debug 签名，零配置；Release 包也暂用 debug 签名（见 build.gradle.kts）
# ------------------------------------------------------------------
build_apk() {
    local variant="${1:-Debug}"   # Debug | Release
    say "Gradle 打包 APK（$variant）"
    (cd "$ANDROID_DIR" && gradle "assemble${variant}")
}

install_apk() {
    say "安装到设备"
    adb install -r "$1"
}

run_app() {
    say "启动应用"
    adb shell am start -n "$PACKAGE/$ACTIVITY"
}

# ------------------------------------------------------------------
# 跟随日志：-s 按标签过滤，只显示 gmk3 与 AndroidRuntime（崩溃）
# ------------------------------------------------------------------
follow_log() {
    say "跟随日志（Ctrl+C 退出）"
    adb logcat -c 2>/dev/null || true
    adb logcat -s "gmk3:*" "AndroidRuntime:*" "*:S"
}

# ------------------------------------------------------------------
# 抓崩溃栈：debuggerd 输出的 pc 值是 .so 内偏移，直接喂给
# llvm-addr2line（配合 release profile 的行号表）即可还原出
# 「函数名 + 文件:行号」。
# 注意：coooool 装的 dist 版 .so 已剥离符号，无法还原——先用
# android/dev.sh build 装回带行号表的版本再抓崩溃。
# ------------------------------------------------------------------
crash_report() {
    [ -f "$ADDR2LINE" ] || die "找不到 llvm-addr2line（NDK: $NDK）"
    local abi
    abi="$(adb shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r')"
    local so="$JNI_DIR/$abi/libgmk3.so"
    [ -f "$so" ] || die "找不到 $so，请先 build"
    if ! "$LLVM_NM" "$so" 2>/dev/null | grep -q " android_main"; then
        die "$so 已剥离符号（coooool 产物）。先跑 android/dev.sh build 装回带调试信息的版本"
    fi
    say "最近的崩溃栈（已符号化）"
    adb logcat -d \
        | grep -oE "pc [0-9a-f]+ .*libgmk3\.so" \
        | grep -oE "pc [0-9a-f]+" \
        | awk '{print "0x"$2}' \
        | sort -u \
        | xargs -r "$ADDR2LINE" -f -C -e "$so" \
        | paste - - \
        | sort -u
}

# ------------------------------------------------------------------
# 主流程
# ------------------------------------------------------------------
CMD="${1:-all}"
case "$CMD" in
    all)
        build_so release && build_apk Debug && install_apk "$APK_DEBUG" && run_app && follow_log
        ;;
    build)
        build_so release && build_apk Debug
        ;;
    so)
        build_so release
        ;;
    apk)
        build_apk Debug
        ;;
    install)
        install_apk "$APK_DEBUG"
        ;;
    run)
        run_app
        ;;
    logcat)
        follow_log
        ;;
    # 发布版装手机：dist profile（无调试/极致优化/最小体积）→ release APK → 安装 → 启动
    coooool)
        build_so dist && build_apk Release && install_apk "$APK_RELEASE" && run_app
        ;;
    crash)
        crash_report
        ;;
    *)
        die "未知子命令: $CMD（可用: all build so apk install run logcat crash coooool）"
        ;;
esac
