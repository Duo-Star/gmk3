// src/canvas/styles.rs
//! 画布层绘图样式（由 monkey 桥接自 DSL 绘图属性，勿与 DSL 层混淆）。

/// 点形状（与 DSL 点形状语义一一对应）：
/// `solid` 实心圆；`hollow` 空心圆环；`cross`（DSL `dashed`）十字；
/// `dotted` 缩小实心点。着色器编号见变体顺序（0 起）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DotShape {
    #[default]
    Solid,
    Hollow,
    Cross,
    Dotted,
}

impl DotShape {
    /// 供点着色器使用的形状编号（与 WGSL 分支一致）。
    #[inline]
    pub fn shader_code(self) -> u32 {
        match self {
            DotShape::Solid => 0,
            DotShape::Hollow => 1,
            DotShape::Cross => 2,
            DotShape::Dotted => 3,
        }
    }
}

/// 线 / 圆锥曲线形状。本轮渲染仅实现 `Solid`（dashed/dotted/hollow
/// 需要弧长虚线分段，下一轮落地），字段先行对齐 DSL。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineShape {
    #[default]
    Solid,
    Dashed,
    Dotted,
    Hollow,
}

/// 点样式。每个对象必然拥有名称（`label`），`label_visible` 控制标签是否显示。
#[derive(Clone, Debug)]
pub struct DotStyle {
    pub color: [f32; 4],
    pub radius_px: f32,
    pub shape: DotShape,
    pub label: String,
    pub label_visible: bool,
    pub selected: bool,
    pub visible: bool,
}

impl Default for DotStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            radius_px: 3.8,
            shape: DotShape::Solid,
            label: String::new(),
            label_visible: true,
            selected: false,
            visible: true,
        }
    }
}

/// 骈点样式（同点样式；本轮骈点绘制不变）。
#[derive(Clone, Debug)]
pub struct DdotStyle {
    pub color: [f32; 4],
    pub radius_px: f32,
    pub shape: DotShape,
    pub label: String,
    pub label_visible: bool,
    pub selected: bool,
    pub visible: bool,
}

impl Default for DdotStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            radius_px: 3.8,
            shape: DotShape::Solid,
            label: String::new(),
            label_visible: true,
            selected: false,
            visible: true,
        }
    }
}

/// 直线样式。
#[derive(Clone, Debug)]
pub struct LineStyle {
    pub color: [f32; 4],
    pub width_px: f32,
    pub shape: LineShape,
    pub label: String,
    pub label_visible: bool,
    pub selected: bool,
    pub visible: bool,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            width_px: 1.6,
            shape: LineShape::Solid,
            label: String::new(),
            label_visible: true,
            selected: false,
            visible: true,
        }
    }
}

/// 圆锥曲线样式（同直线）。
#[derive(Clone, Debug)]
pub struct ConicStyle {
    pub color: [f32; 4],
    pub width_px: f32,
    pub shape: LineShape,
    pub label: String,
    pub label_visible: bool,
    pub selected: bool,
    pub visible: bool,
}

impl Default for ConicStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            width_px: 1.6,
            shape: LineShape::Solid,
            label: String::new(),
            label_visible: true,
            selected: false,
            visible: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 画布底图配色（背景 / 网格 / 坐标轴 / 标签 / 边框）
// ---------------------------------------------------------------------------

/// 画布底图配色，随日间 / 夜间主题切换（`CanvasColors::for_theme`）。
///
/// 注意：对象（点 / 线 / 曲线）的颜色由 monkey 层的色板按主题解析，
/// 不在此处。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasColors {
    /// 画布背景（sRGB，不透明）。
    pub background: [f32; 3],
    /// 次网格线颜色。
    pub grid_minor: [f32; 4],
    /// 主网格线颜色。
    pub grid_major: [f32; 4],
    /// 坐标轴颜色。
    pub axis: [f32; 4],
    /// 坐标轴刻度数字颜色。
    pub tick_label: [f32; 4],
    /// 坐标轴名称（X / Y）颜色。
    pub axis_name: [f32; 4],
    /// 画布边框灰阶（0..=255）。
    pub border_gray: u8,
    /// 灭线（无穷远直线）颜色：细核心线 + 光晕同色。
    pub horizon: [f32; 4],
}

impl CanvasColors {
    /// 夜间主题（默认，与旧版硬编码一致）。
    pub const DARK: Self = Self {
        background: [0.0, 0.0, 0.0],
        grid_minor: [0.16, 0.16, 0.17, 1.0],
        grid_major: [0.30, 0.30, 0.32, 1.0],
        axis: [0.7, 0.7, 0.7, 1.0],
        tick_label: [0.78, 0.78, 0.80, 0.92],
        axis_name: [0.92, 0.92, 0.94, 0.96],
        border_gray: 50,
        horizon: [1.0, 1.0, 1.0, 1.0],
    };

    /// 日间主题。
    pub const LIGHT: Self = Self {
        background: [0.97, 0.97, 0.98],
        grid_minor: [0.84, 0.84, 0.86, 1.0],
        grid_major: [0.66, 0.66, 0.69, 1.0],
        axis: [0.36, 0.36, 0.40, 1.0],
        tick_label: [0.33, 0.33, 0.36, 0.92],
        axis_name: [0.15, 0.15, 0.18, 0.96],
        border_gray: 200,
        horizon: [1.0, 0.82, 0.45, 1.0],
    };

    /// 按主题取配色。
    pub fn for_theme(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }
}
