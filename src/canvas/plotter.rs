use crate::canvas::camera::{
    CanvasUniforms, ConicInstance, LineVertex, Mat3F64, PointInstance, PreparedDrawData,
    ProjectiveCamera, clip_line_to_ndc, extrude_line_quad,
};
use crate::canvas::labels::{CanvasLabel, collect_canvas_labels};
use crate::canvas::styles::{CanvasColors, ConicStyle, DdotStyle, DotStyle, LineStyle};
use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::{conic::Conic, dot::Dot, line::Line};

// ---------------------------------------------------------------------------
// 网格与坐标轴：显示参数
// ---------------------------------------------------------------------------

/// 次网格线的目标像素间距（喂给 nice numbers 算法）
const MINOR_TARGET_PX: f64 = 24.0;
/// 主网格与次网格步长比（主网格线每隔 5 条次网格线一条）
// 可见性：
pub(crate) const MAJOR_EVERY: i64 = 5;

/// 次/主网格线像素线宽与坐标轴线宽（颜色见 `CanvasColors`）
const GRID_MINOR_LINE_PX: f32 = 1.0;
const GRID_MAJOR_LINE_PX: f32 = 1.15;
const AXIS_WIDTH_PX: f32 = 1.3;

/// 灭线：核心细线半宽与光晕高斯宽度（px）
const HORIZON_LINE_PX: f32 = 0.8;
const HORIZON_GLOW_PX: f32 = 16.0;

/// 网格显示模式。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GridMode {
    /// 不显示网格
    Off,
    /// 仅主要网格
    Major,
    /// 主要 + 次要网格（默认）
    #[default]
    Full,
}

impl GridMode {
    /// 全部模式与 UI 文案。
    pub const ALL: [(GridMode, &'static str); 3] = [
        (GridMode::Off, "无网格"),
        (GridMode::Major, "主要网格"),
        (GridMode::Full, "主要和次要网格"),
    ];

    /// UI 文案。
    pub fn label(self) -> &'static str {
        match self {
            GridMode::Off => "无网格",
            GridMode::Major => "主要网格",
            GridMode::Full => "主要和次要网格",
        }
    }

    /// 着色器编号（0/1/2）。
    #[inline]
    pub fn as_u32(self) -> u32 {
        match self {
            GridMode::Off => 0,
            GridMode::Major => 1,
            GridMode::Full => 2,
        }
    }
}

/// 美观步长（nice numbers）算法 —— Heckbert, Graphics Gems I (1990)。
/// 把任意正数舍入到 1-2-5 序列：`{1, 2, 5} × 10ᵏ`。
pub fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f64.powf(exp);
    let frac = raw / base;
    let nice_frac = if frac < 1.5 {
        1.0
    } else if frac < 3.0 {
        2.0
    } else if frac < 7.0 {
        5.0
    } else {
        10.0
    };
    nice_frac * base
}

/// 网格次步长：**只由仿射视图（`half_height` / `aspect` / 视口像素）决定，
/// 与 `proj_matrix` 无关**。
pub fn grid_minor_step(camera: &ProjectiveCamera, aspect: f64, viewport_px: [f32; 2]) -> f64 {
    let w_px = f64::from(viewport_px[0].max(1.0));
    let h_px = f64::from(viewport_px[1].max(1.0));
    let world_per_px =
        (2.0 * camera.half_height * aspect / w_px).max(2.0 * camera.half_height / h_px);
    nice_step(world_per_px * MINOR_TARGET_PX)
}

/// 在 `f64` 下构造 `NDC -> 次网格单位坐标` 的 3x3 逆变换矩阵：
/// 预先把原点对齐到当前视口附近的主网格整数倍，消除高倍缩放/大平移下的 `f32` 抖动，
/// 同时在射影视角下严格保持所有主/次网格线的射影不变性。
fn build_grid_inv_matrix(camera: &ProjectiveCamera, h_inv: &Mat3F64, minor_step: f64) -> Mat3F64 {
    let major_step = minor_step * (MAJOR_EVERY as f64);

    let center_w = h_inv.mul_vec([0.0, 0.0, 1.0]);
    let (ref_x, ref_y) = if center_w[2].abs() > 1e-4 {
        let wx = center_w[0] / center_w[2];
        let wy = center_w[1] / center_w[2];
        if wx.abs() < 1e9 * major_step && wy.abs() < 1e9 * major_step {
            (wx, wy)
        } else {
            (camera.center[0], camera.center[1])
        }
    } else {
        (camera.center[0], camera.center[1])
    };

    let snap_x = (ref_x / major_step).round() * major_step;
    let snap_y = (ref_y / major_step).round() * major_step;

    let inv_s = 1.0 / minor_step;
    let to_grid = Mat3F64 {
        m: [
            [inv_s, 0.0, -snap_x * inv_s],
            [0.0, inv_s, -snap_y * inv_s],
            [0.0, 0.0, 1.0],
        ],
    };
    to_grid.mul_mat(h_inv)
}

/// 把世界坐标系直线（系数 `[a, b, c]`：`a·x + b·y + c = 0`）经
/// `h_invᵀ` 变换到 NDC、截断并挤出为三角形顶点。
#[inline]
fn push_world_line(
    l_world: [f64; 3],
    h_inv: &Mat3F64,
    w_px: f32,
    h_px: f32,
    half_width_px: f32,
    color: [f32; 4],
    out: &mut Vec<LineVertex>,
) {
    let l_ndc = h_inv.mul_cov(l_world);
    if let Some((p0, p1)) = clip_line_to_ndc(l_ndc) {
        extrude_line_quad(p0, p1, w_px, h_px, half_width_px, color, out);
    }
}

#[inline]
fn push_projected_dot(
    dot: &Dot,
    h: &Mat3F64,
    radius_px: f32,
    color: [f32; 4],
    shape: u32,
    selected: u32,
    out: &mut Vec<PointInstance>,
) {
    let dot_re = dot.to_3f64();
    let p = h.mul_vec(dot_re);
    if p[2].abs() > 1e-12 {
        let x = (p[0] / p[2]) as f32;
        let y = (p[1] / p[2]) as f32;
        if x.abs() <= 1.2 && y.abs() <= 1.2 {
            out.push(PointInstance {
                center_ndc: [x, y],
                radius_px,
                color,
                shape,
                selected,
            });
        }
    }
}

/// 视角矩阵插值动画：逐元素线性插值 + smoothstep 缓动，按 [`Plotter::time_sec`] 推进。
#[derive(Clone, Copy, Debug)]
struct ProjAnim {
    start: Mat3F64,
    target: Mat3F64,
    t0: f64,
    dur: f64,
}

/// 视角矩阵动画时长（秒）。
const PROJ_ANIM_DURATION: f64 = 0.24;

/// 命令式几何绘图器：每帧重置缓冲、执行构造更新、打包提交给 wgpu。
pub struct Plotter {
    dots: Vec<Dot>,
    lines: Vec<Line>,
    conics: Vec<Conic>,
    d_dots: Vec<Ddot>,

    dot_styles: Vec<DotStyle>,
    line_styles: Vec<LineStyle>,
    conic_styles: Vec<ConicStyle>,
    d_dot_styles: Vec<DdotStyle>,

    pub camera: ProjectiveCamera,
    /// 网格显示模式（无 / 仅主网格 / 主+次网格，默认主+次）
    pub grid_mode: GridMode,
    /// 是否显示世界坐标系坐标轴（默认开启）
    pub show_axes: bool,
    /// 是否显示坐标刻度文字（独立于网格，默认开启）
    pub show_tick_labels: bool,
    /// 是否显示灭线（无穷远直线处的细线 + 光晕，默认关闭）
    pub show_horizon: bool,
    /// 底图配色（背景 / 网格 / 坐标轴 / 标签 / 边框 / 灭线），随主题切换
    pub colors: CanvasColors,
    time_sec: f64,
    /// 视角矩阵插值动画（进行中为 `Some`）
    proj_anim: Option<ProjAnim>,
}

impl Default for Plotter {
    fn default() -> Self {
        Self::new()
    }
}

impl Plotter {
    pub fn new() -> Self {
        Self {
            dots: Vec::new(),
            lines: Vec::new(),
            conics: Vec::new(),
            d_dots: Vec::new(),
            dot_styles: Vec::new(),
            line_styles: Vec::new(),
            conic_styles: Vec::new(),
            d_dot_styles: Vec::new(),
            camera: ProjectiveCamera::default(),
            grid_mode: GridMode::Full,
            show_axes: true,
            show_tick_labels: true,
            show_horizon: false,
            colors: CanvasColors::DARK,
            time_sec: 0.0,
            proj_anim: None,
        }
    }

    /// 按主题切换底图配色（`dark = true` 夜间，`false` 日间）。
    #[inline]
    pub fn set_theme(&mut self, dark: bool) {
        self.colors = CanvasColors::for_theme(dark);
    }

    /// 启动视角矩阵插值动画：从当前矩阵平滑过渡到 `target`（时长约 0.24s）。
    ///
    /// 再次调用会以当前插值结果为新的起点，天然平滑衔接。
    pub fn animate_proj_matrix_to(&mut self, target: Mat3F64) {
        self.proj_anim = Some(ProjAnim {
            start: self.camera.proj_matrix,
            target,
            t0: self.time_sec,
            dur: PROJ_ANIM_DURATION,
        });
    }

    /// 取消进行中的视角矩阵动画（用户直接操作相机时调用，避免互相覆盖）。
    pub fn cancel_proj_anim(&mut self) {
        self.proj_anim = None;
    }

    /// 是否有视角矩阵动画正在进行。
    pub fn proj_anim_active(&self) -> bool {
        self.proj_anim.is_some()
    }

    /// 按 `time_sec` 推进视角矩阵动画（`update` 内调用）。
    fn tick_proj_anim(&mut self) {
        let Some(anim) = self.proj_anim else {
            return;
        };
        let s = ((self.time_sec - anim.t0) / anim.dur).clamp(0.0, 1.0);
        // smoothstep 缓动：起止都柔
        let e = s * s * (3.0 - 2.0 * s);

        let mut m = [[0.0_f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = (1.0 - e) * anim.start.m[i][j] + e * anim.target.m[i][j];
            }
        }
        // 齐次矩阵整体缩放不改变射影意义：仅保持数值量级健康
        let mut max_abs = 0.0_f64;
        for row in m {
            for v in row {
                max_abs = max_abs.max(v.abs());
            }
        }
        if max_abs > 20.0 || (max_abs > 1e-18 && max_abs < 0.05) {
            let sc = 1.0 / max_abs;
            for row in &mut m {
                for v in row {
                    *v *= sc;
                }
            }
        }

        self.camera.proj_matrix = Mat3F64 { m };
        if s >= 1.0 {
            self.proj_anim = None;
        }
    }

    #[inline]
    pub fn t(&self) -> f64 {
        self.time_sec
    }

    #[inline]
    pub fn camera(&self) -> &ProjectiveCamera {
        &self.camera
    }

    #[inline]
    pub fn camera_mut(&mut self) -> &mut ProjectiveCamera {
        &mut self.camera
    }

    /// 清空本帧缓冲（保留已分配内存容量，避免每帧堆分配）
    pub fn clear(&mut self) {
        self.dots.clear();
        self.lines.clear();
        self.conics.clear();
        self.d_dots.clear();
        self.dot_styles.clear();
        self.line_styles.clear();
        self.conic_styles.clear();
        self.d_dot_styles.clear();
    }

    /// 绘图器帧更新入口：更新时间 t -> 推进视角动画 -> 清缓冲 -> 执行用户的命令式构造闭包
    pub fn update(&mut self, t: f64, f: impl FnOnce(&mut Self)) {
        self.time_sec = t;
        self.tick_proj_anim();
        self.clear();
        f(self);
    }

    // ------------------------------------------------------------------
    // 命令式提交接口
    // ------------------------------------------------------------------

    #[inline]
    pub fn draw_dot(&mut self, dot: Dot) -> usize {
        self.draw_dot_with(dot, DotStyle::default())
    }

    #[inline]
    pub fn draw_dot_with(&mut self, dot: Dot, style: DotStyle) -> usize {
        let idx = self.dots.len();
        self.dots.push(dot);
        self.dot_styles.push(style);
        idx
    }

    #[inline]
    pub fn draw_d_dot(&mut self, d_dot: Ddot) -> usize {
        self.draw_d_dot_with(d_dot, DdotStyle::default())
    }

    #[inline]
    pub fn draw_d_dot_with(&mut self, d_dot: Ddot, style: DdotStyle) -> usize {
        let idx = self.d_dots.len();
        self.d_dots.push(d_dot);
        self.d_dot_styles.push(style);
        idx
    }

    #[inline]
    pub fn draw_line(&mut self, line: Line) -> usize {
        self.draw_line_with(line, LineStyle::default())
    }

    #[inline]
    pub fn draw_line_with(&mut self, line: Line, style: LineStyle) -> usize {
        let idx = self.lines.len();
        self.lines.push(line);
        self.line_styles.push(style);
        idx
    }

    #[inline]
    pub fn draw_conic(&mut self, conic: Conic) -> usize {
        self.draw_conic_with(conic, ConicStyle::default())
    }

    #[inline]
    pub fn draw_conic_with(&mut self, conic: Conic, style: ConicStyle) -> usize {
        let idx = self.conics.len();
        self.conics.push(conic);
        self.conic_styles.push(style);
        idx
    }

    // ------------------------------------------------------------------
    // 只读访问平行缓冲区
    // ------------------------------------------------------------------

    pub fn dots(&self) -> (&[Dot], &[DotStyle]) {
        (&self.dots, &self.dot_styles)
    }

    pub fn d_dots(&self) -> (&[Ddot], &[DdotStyle]) {
        (&self.d_dots, &self.d_dot_styles)
    }

    pub fn lines(&self) -> (&[Line], &[LineStyle]) {
        (&self.lines, &self.line_styles)
    }

    pub fn conics(&self) -> (&[Conic], &[ConicStyle]) {
        (&self.conics, &self.conic_styles)
    }

    // ------------------------------------------------------------------
    // 生成供 wgpu 消费的帧数据（f64 射影变换在此完成）
    // ------------------------------------------------------------------

    pub fn prepare_draw_data(
        &self,
        viewport_px: [f32; 2],
        pixels_per_point: f32,
    ) -> PreparedDrawData {
        let w_px = viewport_px[0].max(1.0);
        let h_px = viewport_px[1].max(1.0);
        let aspect = (w_px as f64) / (h_px as f64);

        let (h, h_inv) = self.camera.ndc_transforms(aspect);
        let h_inv_t = h_inv.transpose();

        // 0. 无限射影网格矩阵（由 GPU 片元着色器在全画布求值）
        let minor_step = grid_minor_step(&self.camera, aspect, viewport_px);
        let grid_inv = build_grid_inv_matrix(&self.camera, &h_inv, minor_step);

        // 1. 世界坐标系坐标轴（仅 2 条线，排在用户直线之前）
        let mut line_vertices = Vec::with_capacity(self.lines.len() * 6 + 12);
        if self.show_axes {
            for l_world in [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0]] {
                push_world_line(
                    l_world,
                    &h_inv,
                    w_px,
                    h_px,
                    0.5 * AXIS_WIDTH_PX * pixels_per_point,
                    self.colors.axis,
                    &mut line_vertices,
                );
            }
        }
        let axis_vertex_count = line_vertices.len();

        // 2. 用户直线 + 细节信息 -> 截断挤出
        for (line, style) in self.lines.iter().zip(&self.line_styles) {
            if !style.visible {
                continue;
            }
            let line_re = line.to_3f64();
            push_world_line(
                line_re,
                &h_inv,
                w_px,
                h_px,
                0.5 * style.width_px * pixels_per_point,
                style.color,
                &mut line_vertices,
            );
        }

        // 3. 圆锥曲线 + 细节信息 -> 隐式二次型系数
        let mut conic_instances = Vec::with_capacity(self.conics.len());
        for (conic, style) in self.conics.iter().zip(&self.conic_styles) {
            if !style.visible {
                continue;
            }
            let q_re = conic.to_3x3f64();
            let q_ndc = h_inv_t.mul_mat(&Mat3F64 { m: q_re }).mul_mat(&h_inv);

            let a = q_ndc.m[0][0];
            let b = 0.5 * (q_ndc.m[0][1] + q_ndc.m[1][0]);
            let c = q_ndc.m[1][1];
            let d = 0.5 * (q_ndc.m[0][2] + q_ndc.m[2][0]);
            let e = 0.5 * (q_ndc.m[1][2] + q_ndc.m[2][1]);
            let f = q_ndc.m[2][2];

            let max_abs = [a, b, c, d, e, f]
                .into_iter()
                .map(f64::abs)
                .fold(0.0_f64, f64::max);
            if max_abs > 1e-15 {
                let s = 1.0 / max_abs;
                conic_instances.push(ConicInstance {
                    quad_abc: [(a * s) as f32, (b * s) as f32, (c * s) as f32],
                    quad_def: [(d * s) as f32, (e * s) as f32, (f * s) as f32],
                    half_width_px: 0.5 * style.width_px * pixels_per_point,
                    color: style.color,
                });
            }
        }

        // 4. 点与骈点 + 细节信息 -> 透视除法得到 NDC
        let mut point_instances = Vec::with_capacity(self.dots.len() + self.d_dots.len() * 2);
        for (dot, style) in self.dots.iter().zip(&self.dot_styles) {
            if !style.visible {
                continue;
            }
            push_projected_dot(
                dot,
                &h,
                style.radius_px * pixels_per_point,
                style.color,
                style.shape.shader_code(),
                u32::from(style.selected),
                &mut point_instances,
            );
        }
        for (d_dot, style) in self.d_dots.iter().zip(&self.d_dot_styles) {
            if !style.visible {
                continue;
            }
            let r = style.radius_px * pixels_per_point;
            // 骈点绘制本轮不变：实心圆、无选中渲染
            push_projected_dot(&d_dot.p(), &h, r, style.color, 0, 0, &mut point_instances);
            push_projected_dot(&d_dot.q(), &h, r, style.color, 0, 0, &mut point_instances);
        }

        // 4. 灭线（无穷远直线）在 NDC 的系数（归一化，|(a,b)| = 1）
        let l_inf_ndc = h_inv.mul_cov([0.0, 0.0, 1.0]);
        let n_inf = (l_inf_ndc[0] * l_inf_ndc[0] + l_inf_ndc[1] * l_inf_ndc[1]).sqrt();
        let horizon_abc = if n_inf > 1e-12 {
            [
                (l_inf_ndc[0] / n_inf) as f32,
                (l_inf_ndc[1] / n_inf) as f32,
                (l_inf_ndc[2] / n_inf) as f32,
                0.0,
            ]
        } else {
            // 灭线在 NDC 的无穷远处（仿射视角）：a=b=0，着色器直接跳过
            [0.0, 0.0, 1.0, 0.0]
        };

        let gm = &grid_inv.m;
        PreparedDrawData {
            uniforms: CanvasUniforms {
                viewport_px: [w_px, h_px],
                grid_mode: self.grid_mode.as_u32(),
                minor_line_px: GRID_MINOR_LINE_PX * pixels_per_point,
                grid_inv_col0: [gm[0][0] as f32, gm[1][0] as f32, gm[2][0] as f32, 0.0],
                grid_inv_col1: [gm[0][1] as f32, gm[1][1] as f32, gm[2][1] as f32, 0.0],
                grid_inv_col2: [gm[0][2] as f32, gm[1][2] as f32, gm[2][2] as f32, 0.0],
                grid_minor_color: self.colors.grid_minor,
                grid_major_color: self.colors.grid_major,
                major_line_px: GRID_MAJOR_LINE_PX * pixels_per_point,
                major_every: MAJOR_EVERY as f32,
                show_horizon: u32::from(self.show_horizon),
                horizon_line_px: HORIZON_LINE_PX * pixels_per_point,
                horizon_glow_px: HORIZON_GLOW_PX * pixels_per_point,
                _pad0: [0.0, 0.0, 0.0],
                horizon_abc,
                horizon_color: self.colors.horizon,
            },
            axis_vertex_count,
            line_vertices,
            conic_instances,
            point_instances,
        }
    }

    pub fn prepare_labels_into(
        &self,
        viewport_px: [f32; 2],
        pixels_per_point: f32,
        out: &mut Vec<CanvasLabel>,
    ) {
        collect_canvas_labels(
            &self.camera,
            self.show_axes,
            self.show_tick_labels,
            viewport_px,
            pixels_per_point,
            &self.colors,
            out,
        );
    }

    pub fn prepare_labels(&self, viewport_px: [f32; 2], pixels_per_point: f32) -> Vec<CanvasLabel> {
        let mut out = Vec::with_capacity(48);
        self.prepare_labels_into(viewport_px, pixels_per_point, &mut out);
        out
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::camera::{Mat3F64, ProjectiveCamera};
    use crate::pakoo_forest::geo::Line;

    #[test]
    fn nice_step_1_2_5() {
        assert_eq!(nice_step(0.00013), 1e-4);
        assert_eq!(nice_step(0.23), 0.2);
        assert_eq!(nice_step(0.47), 0.5);
        assert_eq!(nice_step(0.9), 1.0);
        assert_eq!(nice_step(1.0), 1.0);
        assert_eq!(nice_step(2.0), 2.0);
        assert_eq!(nice_step(2.9), 2.0);
        assert_eq!(nice_step(3.1), 5.0);
        assert_eq!(nice_step(5.0), 5.0);
        assert_eq!(nice_step(6.0), 5.0);
        assert_eq!(nice_step(8.0), 10.0);
        assert_eq!(nice_step(23.0), 20.0);
        assert_eq!(nice_step(50.0), 50.0);
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(nice_step(-3.0), 1.0);
        assert_eq!(nice_step(f64::NAN), 1.0);
        assert_eq!(nice_step(f64::INFINITY), 1.0);
    }

    #[test]
    fn nice_step_density() {
        for world_per_px in [1e-4, 0.0013, 0.013333, 0.27, 3.7, 123.0] {
            let s = nice_step(world_per_px * MINOR_TARGET_PX);
            let px = s / world_per_px;
            assert!(
                (12.0..=48.0).contains(&px),
                "world_per_px={world_per_px}, step={s}, px={px}"
            );
        }
    }

    #[test]
    fn grid_step_independent_of_proj_matrix() {
        let c1 = ProjectiveCamera::default();
        let mut c2 = ProjectiveCamera::default();
        c2.proj_matrix = Mat3F64 {
            m: [[1.0, 0.2, 0.0], [0.1, 1.0, 0.3], [0.0, 0.5, 1.0]],
        };
        let s1 = grid_minor_step(&c1, 4.0 / 3.0, [800.0, 600.0]);
        let s2 = grid_minor_step(&c2, 4.0 / 3.0, [800.0, 600.0]);
        assert_eq!(s1, s2);

        let mut c3 = ProjectiveCamera::default();
        c3.half_height = 40.0;
        assert!(grid_minor_step(&c3, 4.0 / 3.0, [800.0, 600.0]) > s1);

        let mut c4 = ProjectiveCamera::default();
        c4.center = [100.0, -50.0];
        assert_eq!(grid_minor_step(&c4, 4.0 / 3.0, [800.0, 600.0]), s1);
    }

    #[test]
    fn grid_inv_matrix_maps_projective_world_points_to_grid_units() {
        let mut cam = ProjectiveCamera::default();
        cam.center = [10000.3, -5000.7];
        cam.proj_matrix = Mat3F64 {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.015, 0.015, 1.0]],
        };
        let aspect = 4.0 / 3.0;
        let minor = grid_minor_step(&cam, aspect, [800.0, 600.0]);
        let (h, h_inv) = cam.ndc_transforms(aspect);
        let m_grid = build_grid_inv_matrix(&cam, &h_inv, minor);

        // 验证世界坐标系里相距 1 个主步长与 1 个次步长的点，在变换后精确差 (1, 5)
        let p0 = [10000.0 * minor, -5000.0 * minor, 1.0];
        let p1 = [10001.0 * minor, -4995.0 * minor, 1.0];
        let ndc0 = h.mul_vec(p0);
        let ndc1 = h.mul_vec(p1);

        let g0 = m_grid.mul_vec([ndc0[0] / ndc0[2], ndc0[1] / ndc0[2], 1.0]);
        let g1 = m_grid.mul_vec([ndc1[0] / ndc1[2], ndc1[1] / ndc1[2], 1.0]);

        let u0 = g0[0] / g0[2];
        let v0 = g0[1] / g0[2];
        let u1 = g1[0] / g1[2];
        let v1 = g1[1] / g1[2];

        assert!((u1 - u0 - 1.0).abs() < 1e-6);
        assert!((v1 - v0 - 5.0).abs() < 1e-6);
    }

    #[test]
    fn prepare_draws_grid_uniform_and_axes() {
        let p = Plotter::new();
        assert_eq!(p.grid_mode, GridMode::Full);
        assert!(p.show_axes && p.show_tick_labels && !p.show_horizon);
        let data = p.prepare_draw_data([800.0, 600.0], 1.0);
        assert_eq!(data.uniforms.grid_mode, 2);
        assert_eq!(data.uniforms.show_horizon, 0);
        // 仅 2 条坐标轴在直线缓冲里（网格由 GPU 全屏着色器绘制）
        assert_eq!(data.axis_vertex_count, 12);
        assert_eq!(data.line_vertices.len(), 12);
        assert!(
            data.line_vertices
                .iter()
                .all(|v| v.color == CanvasColors::DARK.axis)
        );

        let mut p2 = Plotter::new();
        p2.grid_mode = GridMode::Off;
        p2.show_axes = false;
        let data2 = p2.prepare_draw_data([800.0, 600.0], 1.0);
        assert_eq!(data2.uniforms.grid_mode, 0);
        assert!(data2.line_vertices.is_empty());
        assert_eq!(data2.axis_vertex_count, 0);
    }

    #[test]
    fn grid_modes_map_to_uniform() {
        let mut p = Plotter::new();
        for (mode, code) in [
            (GridMode::Off, 0),
            (GridMode::Major, 1),
            (GridMode::Full, 2),
        ] {
            p.grid_mode = mode;
            let data = p.prepare_draw_data([800.0, 600.0], 1.0);
            assert_eq!(data.uniforms.grid_mode, code, "{mode:?}");
        }
    }

    #[test]
    fn horizon_line_is_normalized_ndc_coefficients() {
        let mut p = Plotter::new();
        p.show_horizon = true;
        // 前视视角：灭线进入画面
        p.camera.proj_matrix = Mat3F64::FRONT_VIEW;
        let data = p.prepare_draw_data([800.0, 600.0], 1.0);
        assert_eq!(data.uniforms.show_horizon, 1);
        let n =
            (data.uniforms.horizon_abc[0].powi(2) + data.uniforms.horizon_abc[1].powi(2)).sqrt();
        assert!((n - 1.0).abs() < 1e-6, "灭线系数 (a,b) 应归一化");
        // 前视：灭线在 NDC 为 y = const 的横线（a ≈ 0）
        assert!(data.uniforms.horizon_abc[0].abs() < 1e-6);
    }

    #[test]
    fn axis_layer_precedes_user_geometry() {
        let mut p = Plotter::new();
        p.draw_line(Line::from_f64s(1.0, 0.0, 0.0));
        let data = p.prepare_draw_data([800.0, 600.0], 1.0);
        assert_eq!(data.axis_vertex_count, 12);
        assert!(data.line_vertices.len() > data.axis_vertex_count);
    }

    #[test]
    fn proj_anim_interpolates_and_completes() {
        let mut p = Plotter::new();
        p.camera.proj_matrix = Mat3F64::SIDE_VIEW;
        p.update(0.0, |_| {});
        p.animate_proj_matrix_to(Mat3F64::IDENTITY);
        assert!(p.proj_anim_active());

        // t = 0.12（s = 0.5，smoothstep e = 0.5）：底行恰为起点与目标的中点
        p.update(0.12, |_| {});
        assert!(p.proj_anim_active());
        let m = p.camera.proj_matrix.m;
        assert!((m[2][0] - 0.175).abs() < 1e-12);
        assert!(m[2][1].abs() < 1e-12);
        assert!((m[2][2] - 1.0).abs() < 1e-12);
        assert!((m[0][0] - 1.0).abs() < 1e-12);

        // t = 0.25：动画结束，矩阵精确等于目标
        p.update(0.25, |_| {});
        assert!(!p.proj_anim_active());
        assert_eq!(p.camera.proj_matrix.m, Mat3F64::IDENTITY.m);
    }

    #[test]
    fn proj_anim_cancel_stops_and_keeps_matrix() {
        let mut p = Plotter::new();
        p.update(0.0, |_| {});
        p.animate_proj_matrix_to(Mat3F64::FRONT_VIEW);
        p.update(0.1, |_| {}); // 动画中途
        assert!(p.proj_anim_active());
        p.cancel_proj_anim();
        let mid = p.camera.proj_matrix.m;
        p.update(1.0, |_| {}); // 取消后不再推进
        assert!(!p.proj_anim_active());
        assert_eq!(p.camera.proj_matrix.m, mid);
    }

    #[test]
    fn proj_anim_restart_uses_current_as_start() {
        let mut p = Plotter::new();
        p.camera.proj_matrix = Mat3F64::SIDE_VIEW;
        p.update(0.0, |_| {});
        p.animate_proj_matrix_to(Mat3F64::IDENTITY);
        p.update(0.15, |_| {}); // 中途
        let mid = p.camera.proj_matrix.m;
        // 重新开启动画到前视：新起点应为当前插值
        p.animate_proj_matrix_to(Mat3F64::FRONT_VIEW);
        assert_eq!(p.camera.proj_matrix.m, mid);
        p.update(0.151, |_| {}); // 前进一小步
        let step = p.camera.proj_matrix.m;
        assert!((step[2][0] - mid[2][0]).abs() > 0.0);
        assert!((step[2][1] - mid[2][1]).abs() > 0.0); // 向 [0, 0.35] 前进
    }
}
