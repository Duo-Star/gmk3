use crate::canvas::camera::{
    CanvasUniforms, ConicInstance, LineVertex, Mat3F64, PointInstance, PreparedDrawData,
    ProjectiveCamera, clip_line_to_ndc, extrude_line_quad,
};

use crate::pakoo_forest::geo::d_dot::Ddot;
use crate::pakoo_forest::geo::{conic::Conic, dot::Dot, line::Line};

use crate::canvas::styles::{ConicStyle, DdotStyle, DotStyle, LineStyle};

// ---------------------------------------------------------------------------
// 网格与坐标轴：显示参数（世界坐标系，绘制在用户几何底层）
// ---------------------------------------------------------------------------

/// 次网格线的目标像素间距（喂给 nice numbers 算法）
const MINOR_TARGET_PX: f64 = 24.0;
/// 主网格与次网格步长比（主网格线每隔 5 条次网格线一条）
const MAJOR_EVERY: i64 = 5;

/// 次网格线颜色（深灰）
const GRID_MINOR_COLOR: [f32; 4] = [0.16, 0.16, 0.17, 1.0];
/// 主网格线颜色（亮灰）
const GRID_MAJOR_COLOR: [f32; 4] = [0.30, 0.30, 0.32, 1.0];
/// 坐标轴颜色（白）与线宽（像素）
const AXIS_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const AXIS_WIDTH_PX: f32 = 1.5;

/// 网格线等级
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridLineKind {
    /// 次网格线
    Minor,
    /// 主网格线（步长 = 5 × 次步长）
    Major,
}

/// 美观步长（nice numbers）算法 —— Heckbert, "Nice Numbers for Graph Labels",
/// Graphics Gems I (1990)。
///
/// 把任意正数舍入到 1-2-5 序列：`{1, 2, 5} × 10ᵏ`。
/// 配合"目标像素间距"使用即可保证：任意缩放级别下网格步长都是
/// 0.1 / 0.2 / 0.5 / 1 / 2 / 5 / 10 / … 这类"整"值，且线密度稳定。
///
/// ```text
/// exp  = ⌊log10(raw)⌋，frac = raw / 10^exp ∈ [1, 10)
/// nice = { 1（frac<1.5）, 2（frac<3）, 5（frac<7）, 10（否则）} × 10^exp
/// ```
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
///
/// 这是刻意的契约：射影变换只扭曲网格（地平线处线条汇聚），不改变网格密度；
/// 密度只随缩放（`half_height`）或窗口尺寸变化。
///
/// 步长 = `nice_step(世界单位/像素 × MINOR_TARGET_PX)`，
/// 保证任意缩放下网格像素间距落在 `[12, 48]px`。
pub fn grid_minor_step(camera: &ProjectiveCamera, aspect: f64, viewport_px: [f32; 2]) -> f64 {
    let w_px = f64::from(viewport_px[0].max(1.0));
    let h_px = f64::from(viewport_px[1].max(1.0));
    let world_per_px =
        (2.0 * camera.half_height * aspect / w_px).max(2.0 * camera.half_height / h_px);
    nice_step(world_per_px * MINOR_TARGET_PX)
}

/// 生成世界坐标系下的网格线：竖线 `x = k·s` 与横线 `y = k·s`（`k ∈ ℤ`，`k ≠ 0`）。
///
/// 返回 `(线系数 [a, b, c]，a·x + b·y + c = 0，等级)`；
/// `k = 0` 的两条轴由调用方单独绘制（白色加粗），避免重复。
fn generate_grid_lines(
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    minor_step: f64,
) -> Vec<([f64; 3], GridLineKind)> {
    let mut out = Vec::new();
    let k0 = (xmin / minor_step).ceil() as i64;
    let k1 = (xmax / minor_step).floor() as i64;
    for k in k0..=k1 {
        if k == 0 {
            continue;
        }
        let kind = if k % MAJOR_EVERY == 0 {
            GridLineKind::Major
        } else {
            GridLineKind::Minor
        };
        out.push(([1.0, 0.0, -(k as f64) * minor_step], kind));
    }
    let k0 = (ymin / minor_step).ceil() as i64;
    let k1 = (ymax / minor_step).floor() as i64;
    for k in k0..=k1 {
        if k == 0 {
            continue;
        }
        let kind = if k % MAJOR_EVERY == 0 {
            GridLineKind::Major
        } else {
            GridLineKind::Minor
        };
        out.push(([0.0, 1.0, -(k as f64) * minor_step], kind));
    }
    out
}

/// 把世界坐标系直线（系数 `[a, b, c]`：`a·x + b·y + c = 0`）经
/// `h_invᵀ` 变换到 NDC、截断并挤出为三角形顶点。
/// 用户几何、网格与坐标轴共用这一条流水线。
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

/// 命令式几何绘图器：每帧重置缓冲、执行构造更新、打包提交给 wgpu。
pub struct Plotter {
    /// 对象主缓冲
    // 基本
    dots: Vec<Dot>,
    lines: Vec<Line>,
    conics: Vec<Conic>,
    // 复合
    d_dots: Vec<Ddot>,

    // 对应的三个绘图细节缓冲，索引同步
    dot_styles: Vec<DotStyle>,
    line_styles: Vec<LineStyle>,
    conic_styles: Vec<ConicStyle>,
    d_dot_styles: Vec<DdotStyle>,

    // 3. 视角数据与当前动画时间（秒）
    pub camera: ProjectiveCamera,
    /// 是否显示世界坐标系网格（默认开启）
    pub show_grid: bool,
    /// 是否显示世界坐标系坐标轴（默认开启）
    pub show_axes: bool,
    /// 网格线数量上限（防御；步长解耦后线数 = 范围/步长，调大可观察射影网格）
    pub max_grid_lines: usize,
    /// 网格生成范围边距：仿射可见窗口半宽/半高的倍数（≥ 1）。
    /// 射影变换会把远处的网格线拉进视野，加大边距可保证地平线附近也有网格。
    pub grid_extent_margin: f64,
    time_sec: f64,
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
            //
            dot_styles: Vec::new(),
            line_styles: Vec::new(),
            conic_styles: Vec::new(),
            d_dot_styles: Vec::new(),
            //
            camera: ProjectiveCamera::default(),
            show_grid: true,
            show_axes: true,
            max_grid_lines: 16384,
            grid_extent_margin: 25.0,
            time_sec: 0.0,
        }
    }

    /// 获取当前时间（秒）
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
        self.dot_styles.clear();
        self.line_styles.clear();
        self.conic_styles.clear();
    }

    /// 绘图器帧更新入口：清缓冲 -> 更新时间 t -> 执行用户的命令式构造闭包
    pub fn update(&mut self, t: f64, f: impl FnOnce(&mut Self)) {
        self.clear();
        self.time_sec = t;
        f(self);
    }

    // ------------------------------------------------------------------
    // 命令式提交接口（返回索引，方便后续拓展或局部修改样式）
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
        self.draw_dot_with(d_dot.p(), DotStyle::default());
        self.draw_dot_with(d_dot.q(), DotStyle::default())
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

        let mut line_vertices = Vec::with_capacity(self.lines.len() * 6 + 512);

        // 0. 世界坐标系坐标轴与网格（最先提交 → 渲染在用户几何底层）
        if self.show_axes {
            // x 轴（y = 0）与 y 轴（x = 0）：白色，略粗
            for l_world in [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0]] {
                push_world_line(
                    l_world,
                    &h_inv,
                    w_px,
                    h_px,
                    0.5 * AXIS_WIDTH_PX * pixels_per_point,
                    AXIS_COLOR,
                    &mut line_vertices,
                );
            }
        }
        if self.show_grid {
            // 步长只由仿射视图决定（与 proj_matrix 解耦）：射影变换只扭曲网格，不改变密度
            let minor_step = grid_minor_step(&self.camera, aspect, viewport_px);
            // 生成范围 = 仿射可见窗口 × 边距系数：
            // 射影变换会把远处的网格线拉进视野，边距保证地平线附近也有网格
            let margin = self.grid_extent_margin.max(1.0);
            let hx = margin * self.camera.half_height * aspect;
            let hy = margin * self.camera.half_height;
            let grid_lines = generate_grid_lines(
                self.camera.center[0] - hx,
                self.camera.center[0] + hx,
                self.camera.center[1] - hy,
                self.camera.center[1] + hy,
                minor_step,
            );
            if grid_lines.len() <= self.max_grid_lines {
                for (l_world, kind) in &grid_lines {
                    let color = match kind {
                        GridLineKind::Minor => GRID_MINOR_COLOR,
                        GridLineKind::Major => GRID_MAJOR_COLOR,
                    };
                    push_world_line(
                        *l_world,
                        &h_inv,
                        w_px,
                        h_px,
                        0.5 * pixels_per_point, // 1px
                        color,
                        &mut line_vertices,
                    );
                }
            }
        }

        // 底层（网格+坐标轴）顶点数：渲染器据此把直线缓冲拆成两段绘制，
        // 次序为 网格层 → 圆锥曲线 → 用户直线 → 点
        let grid_vertex_count = line_vertices.len();

        // 1. 直线 + 细节信息 -> 截断挤出
        for (line, style) in self.lines.iter().zip(&self.line_styles) {
            if !style.visible {
                continue;
            }
            //
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

        // 2. 圆锥曲线 + 细节信息 -> 隐式二次型系数
        let mut conic_instances = Vec::with_capacity(self.conics.len());
        for (conic, style) in self.conics.iter().zip(&self.conic_styles) {
            if !style.visible {
                continue;
            }
            //
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

        // 3. 点 + 细节信息 -> 透视除法得到 NDC
        let mut point_instances = Vec::with_capacity(self.dots.len());
        for (dot, style) in self.dots.iter().zip(&self.dot_styles) {
            if !style.visible {
                continue;
            }
            //
            let dot_re = dot.to_3f64();
            let p = h.mul_vec(dot_re);
            if p[2].abs() > 1e-12 {
                let x = (p[0] / p[2]) as f32;
                let y = (p[1] / p[2]) as f32;
                if x.abs() <= 1.2 && y.abs() <= 1.2 {
                    point_instances.push(PointInstance {
                        center_ndc: [x, y],
                        radius_px: style.radius_px * pixels_per_point,
                        color: style.color,
                    });
                }
            }
        }

        PreparedDrawData {
            uniforms: CanvasUniforms {
                viewport_px: [w_px, h_px],
                _pad: [0.0, 0.0],
            },
            grid_vertex_count,
            line_vertices,
            conic_instances,
            point_instances,
        }
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

    // ---- nice numbers 算法 ----

    #[test]
    fn nice_step_1_2_5() {
        // 各量级舍入到 1-2-5 序列
        assert_eq!(nice_step(0.00013), 1e-4); // frac 1.3 → 1
        assert_eq!(nice_step(0.23), 0.2); // frac 2.3 → 2
        assert_eq!(nice_step(0.47), 0.5); // frac 4.7 → 5
        assert_eq!(nice_step(0.9), 1.0); // frac 9 → 进位 10
        assert_eq!(nice_step(1.0), 1.0);
        assert_eq!(nice_step(2.0), 2.0);
        assert_eq!(nice_step(2.9), 2.0);
        assert_eq!(nice_step(3.1), 5.0);
        assert_eq!(nice_step(5.0), 5.0);
        assert_eq!(nice_step(6.0), 5.0);
        assert_eq!(nice_step(8.0), 10.0);
        assert_eq!(nice_step(23.0), 20.0);
        assert_eq!(nice_step(50.0), 50.0);
        // 非法输入防御
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(nice_step(-3.0), 1.0);
        assert_eq!(nice_step(f64::NAN), 1.0);
        assert_eq!(nice_step(f64::INFINITY), 1.0);
    }

    #[test]
    fn nice_step_density() {
        // 密度性质：目标 24px 时，实际像素间距必落在 [12, 48]px
        for world_per_px in [1e-4, 0.0013, 0.013333, 0.27, 3.7, 123.0] {
            let s = nice_step(world_per_px * MINOR_TARGET_PX);
            let px = s / world_per_px;
            assert!(
                (12.0..=48.0).contains(&px),
                "world_per_px={world_per_px}, step={s}, px={px}"
            );
        }
    }

    // ---- 网格线生成 ----

    #[test]
    fn grid_lines_exclude_axes_and_mark_major() {
        // 窗口 [-4,4]×[-3,3]，次步长 1：竖线 ±1..±4（8 条）+ 横线 ±1..±3（6 条）
        let lines = generate_grid_lines(-4.0, 4.0, -3.0, 3.0, 1.0);
        assert_eq!(lines.len(), 14);
        // 轴（x=0 / y=0）被排除，由调用方单独绘制
        for (l, _) in &lines {
            assert_ne!(*l, [1.0, 0.0, 0.0]);
            assert_ne!(*l, [0.0, 1.0, 0.0]);
        }
        // 范围内没有 5 的倍数 → 全部为次网格
        assert!(lines.iter().all(|(_, k)| *k == GridLineKind::Minor));

        // 窗口扩大到 ±6：k = ±5 是主网格
        let lines2 = generate_grid_lines(-6.0, 6.0, -6.0, 6.0, 1.0);
        assert_eq!(lines2.len(), 24);
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [1.0, 0.0, -5.0] && *k == GridLineKind::Major)
        );
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [1.0, 0.0, 5.0] && *k == GridLineKind::Major)
        );
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [0.0, 1.0, -5.0] && *k == GridLineKind::Major)
        );
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [0.0, 1.0, 5.0] && *k == GridLineKind::Major)
        );
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [1.0, 0.0, -1.0] && *k == GridLineKind::Minor)
        );
        assert!(
            lines2
                .iter()
                .any(|(l, k)| *l == [0.0, 1.0, 2.0] && *k == GridLineKind::Minor)
        );

        // 非整数步长：主网格线位于 k = ±5 的次步长倍处（x = ±2.5）
        let lines3 = generate_grid_lines(-3.0, 3.0, -3.0, 3.0, 0.5);
        assert!(
            lines3
                .iter()
                .any(|(l, k)| *l == [1.0, 0.0, -2.5] && *k == GridLineKind::Major)
        );
    }

    // ---- 步长与射影变换解耦 ----

    #[test]
    fn grid_step_independent_of_proj_matrix() {
        let c1 = ProjectiveCamera::default();
        let mut c2 = ProjectiveCamera::default();
        c2.proj_matrix = Mat3F64 {
            m: [[1.0, 0.2, 0.0], [0.1, 1.0, 0.3], [0.0, 0.5, 1.0]],
        };
        // 契约：改变 proj_matrix 不改变网格步长（只扭曲，不改变密度）
        let s1 = grid_minor_step(&c1, 4.0 / 3.0, [800.0, 600.0]);
        let s2 = grid_minor_step(&c2, 4.0 / 3.0, [800.0, 600.0]);
        assert_eq!(s1, s2);
        // 缩放（half_height）才改变步长
        let mut c3 = ProjectiveCamera::default();
        c3.half_height = 40.0; // 缩小 10 倍 → 步长变大
        let s3 = grid_minor_step(&c3, 4.0 / 3.0, [800.0, 600.0]);
        assert!(s3 > s1);
        // 平移不改变步长
        let mut c4 = ProjectiveCamera::default();
        c4.center = [100.0, -50.0];
        assert_eq!(grid_minor_step(&c4, 4.0 / 3.0, [800.0, 600.0]), s1);
    }

    // ---- 绘制流水线集成 ----

    #[test]
    fn prepare_draws_grid_and_axes() {
        let p = Plotter::new(); // 默认开启网格与坐标轴
        assert!(p.show_grid && p.show_axes);
        let data = p.prepare_draw_data([800.0, 600.0], 1.0);
        assert!(!data.line_vertices.is_empty());
        // 无用户几何时，全部顶点都属于底层（网格+坐标轴）
        assert_eq!(data.grid_vertex_count, data.line_vertices.len());
        // 坐标轴：白色顶点
        assert!(data.line_vertices.iter().any(|v| v.color == AXIS_COLOR));
        // 网格：灰色顶点（当前缩放下主/次网格线都存在）
        assert!(
            data.line_vertices
                .iter()
                .any(|v| v.color == GRID_MINOR_COLOR)
        );
        assert!(
            data.line_vertices
                .iter()
                .any(|v| v.color == GRID_MAJOR_COLOR)
        );

        // 关闭网格与坐标轴后（且无用户对象）直线缓冲为空
        let mut p2 = Plotter::new();
        p2.show_grid = false;
        p2.show_axes = false;
        let data2 = p2.prepare_draw_data([800.0, 600.0], 1.0);
        assert!(data2.line_vertices.is_empty());
        assert_eq!(data2.grid_vertex_count, 0);
    }

    #[test]
    fn grid_layer_precedes_user_geometry() {
        // 用户直线必须排在网格层之后（渲染器据此分成两段绘制）
        let mut p = Plotter::new();
        p.draw_line(Line::from_f64s(1.0, 0.0, 0.0)); // x = 0
        let data = p.prepare_draw_data([800.0, 600.0], 1.0);
        assert!(data.grid_vertex_count > 0);
        assert!(data.grid_vertex_count < data.line_vertices.len());
        // 网格层内只有灰/白（底层）颜色
        let bottom_ok = data.line_vertices[..data.grid_vertex_count]
            .iter()
            .all(|v| {
                v.color == AXIS_COLOR || v.color == GRID_MINOR_COLOR || v.color == GRID_MAJOR_COLOR
            });
        assert!(bottom_ok);
    }

    #[test]
    fn grid_margin_brings_distant_lines_into_view() {
        // 射影变换把地平线附近的远处网格拉进视野：边距越大，可见网格越多
        let proj = Mat3F64 {
            m: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.5, 1.0], // 地平线 y = −2
            ],
        };
        let mut p1 = Plotter::new();
        p1.grid_extent_margin = 1.0;
        p1.camera_mut().proj_matrix = proj;
        let mut p2 = Plotter::new();
        p2.grid_extent_margin = 8.0;
        p2.camera_mut().proj_matrix = proj;
        let d1 = p1.prepare_draw_data([800.0, 600.0], 1.0);
        let d2 = p2.prepare_draw_data([800.0, 600.0], 1.0);
        assert!(d2.grid_vertex_count > d1.grid_vertex_count);
    }
}
