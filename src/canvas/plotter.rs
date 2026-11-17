use crate::canvas::camera::{
    CanvasUniforms, ConicInstance, LineVertex, Mat3F64, PointInstance, PreparedDrawData,
    ProjectiveCamera, clip_line_to_ndc, extrude_line_quad,
};

use crate::pakoo_forest::geo::{conic::Conic, dot::Dot, line::Line};
use crate::pakoo_forest::geo::d_dot::Ddot;

use crate::canvas::styles::{DotStyle, LineStyle, ConicStyle, DdotStyle};

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

        // 1. 直线 + 细节信息 -> 截断挤出
        let mut line_vertices = Vec::with_capacity(self.lines.len() * 6);
        for (line, style) in self.lines.iter().zip(&self.line_styles) {
            if !style.visible {
                continue;
            }
            //
            let line_re = line.to_3f64();

            let l_ndc = h_inv.mul_cov(line_re);
            if let Some((p0, p1)) = clip_line_to_ndc(l_ndc) {
                let half_w = 0.5 * style.width_px * pixels_per_point;
                extrude_line_quad(p0, p1, w_px, h_px, half_w, style.color, &mut line_vertices);
            }
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
            line_vertices,
            conic_instances,
            point_instances,
        }
    }
}