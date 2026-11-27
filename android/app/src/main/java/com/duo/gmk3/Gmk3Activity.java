package com.duo.gmk3;

import android.app.NativeActivity;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;

/**
 * gmk3 的 Activity：在 NativeActivity 基础上补齐现代 Edge-to-Edge +
 * WindowInsets 动态避让。
 *
 * 作用：让窗口内容铺满全屏（边到边），同时监听系统栏/刘海的边衬变化，
 * 通过 JNI 实时把安全区边衬（上/下/左/右，像素）传给 Rust 侧
 * （libgmk3.so 的 nativeUpdateInsets），egui 再据此把整个界面收进安全区。
 * 旋转屏幕、切换手势/三键导航时系统会自动重新回调，无需额外处理。
 */
public class Gmk3Activity extends NativeActivity {

    /** 由 libgmk3.so 提供：安全区边衬变化时回调（单位：像素） */
    private static native void nativeUpdateInsets(int top, int bottom, int left, int right);

    /** 当前 Activity 实例（onCreate 时赋值），供静态方法切 UI 线程用 */
    private static Gmk3Activity sInstance;

    static {
        // 提前加载原生库：WindowInsets 回调可能在 NativeActivity 自己的
        // loadLibrary 之前就触发，否则会 UnsatisfiedLinkError。
        // 重复加载是幂等的，NativeActivity 稍后再加载一次没有影响。
        System.loadLibrary("gmk3");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        sInstance = this;

        // ---- 1. Edge-to-Edge（边到边）----
        // targetSdk 35+ 系统强制边到边，无需处理；以下为低版本兼容
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
        } else {
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
        }
        if (Build.VERSION.SDK_INT >= 21) {
            getWindow().setStatusBarColor(Color.TRANSPARENT);
            getWindow().setNavigationBarColor(Color.TRANSPARENT);
        }
        // 允许内容延伸进刘海区域（API 28+），实际避让靠下面的 insets 监听
        if (Build.VERSION.SDK_INT >= 28) {
            getWindow().getAttributes().layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        // 应用底色是深色，用浅色（白色）系统栏图标
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().getInsetsController().setSystemBarsAppearance(
                    0,
                    WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                            | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS);
        }

        // ---- 2. WindowInsets 动态避让 ----
        getWindow().getDecorView().setOnApplyWindowInsetsListener((v, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets bars = insets.getInsets(
                        WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                nativeUpdateInsets(bars.top, bars.bottom, bars.left, bars.right);
            } else {
                nativeUpdateInsets(
                        insets.getSystemWindowInsetTop(),
                        insets.getSystemWindowInsetBottom(),
                        insets.getSystemWindowInsetLeft(),
                        insets.getSystemWindowInsetRight());
            }
            return WindowInsets.CONSUMED;
        });
    }

    /**
     * 由 libgmk3.so 调用（可能在非 UI 线程）：把系统栏配色同步为应用主题色。
     * 只在应用主题切换时被调用一次，平时零开销。
     *
     * @param argb 状态栏/导航栏背景色（与 egui 面板底色一致）
     * @param dark 应用是否深色主题（决定系统栏图标用白还是黑）
     */
    public static void setSystemBarsStyle(final int argb, final boolean dark) {
        final Gmk3Activity a = sInstance;
        if (a == null) return;
        // 窗口属性只能在 UI 线程设置：从 Rust 帧线程切过去
        a.runOnUiThread(() -> {
            // 边到边模式下系统栏本身透明，真正露出的是窗口背景；
            // 应用表面清屏是不透明的黑，主方案在 egui 侧全窗铺主题色，
            // 这里设置窗口背景作为表面未绘制间隙（首帧前/转屏瞬间）的兜底
            a.getWindow().setBackgroundDrawable(
                    new android.graphics.drawable.ColorDrawable(argb));
            if (Build.VERSION.SDK_INT >= 21) {
                // 低版本（非边到边模式）下这俩仍然有效
                a.getWindow().setStatusBarColor(argb);
                a.getWindow().setNavigationBarColor(argb);
            }
            if (Build.VERSION.SDK_INT >= 29) {
                // 关闭系统自动对比度调整，保证颜色精确
                a.getWindow().setNavigationBarContrastEnforced(false);
            }
            if (Build.VERSION.SDK_INT >= 30) {
                WindowInsetsController c = a.getWindow().getInsetsController();
                if (c != null) {
                    c.setSystemBarsAppearance(
                            dark ? 0
                                 : (WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                                            | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS),
                            WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                                    | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS);
                }
            } else {
                // API 24-29：用旧 flag 控制状态栏图标明暗
                int flags = a.getWindow().getDecorView().getSystemUiVisibility();
                if (dark) {
                    flags &= ~View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
                } else {
                    flags |= View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
                }
                a.getWindow().getDecorView().setSystemUiVisibility(flags);
            }
        });
    }
}
