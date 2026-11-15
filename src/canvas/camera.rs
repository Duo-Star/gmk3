use crate::canvas::scene::CanvasScene;

/// 3x3 实矩阵（行优先），用于视图与射影变换。
#[derive(Clone, Copy, Debug)]
pub struct Mat3F64 {
    pub m: [[f64; 3]; 3],
}

impl Mat3F64 {
    pub const IDENTITY: Self = Self {
        m: [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
    };

    #[inline]
    pub fn mul_mat(&self, rhs: &Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = self.m[i][0] * rhs.m[0][j]
                    + self.m[i][1] * rhs.m[1][j]
                    + self.m[i][2] * rhs.m[2][j];
            }
        }
        Self { m: out }
    }

    #[inline]
    pub fn mul_vec(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.m[0][0] * v[0] + self.m[0][1] * v[1] + self.m[0][2] * v[2],
            self.m[1][0] * v[0] + self.m[1][1] * v[1] + self.m[1][2] * v[2],
            self.m[2][0] * v[0] + self.m[2][1] * v[1] + self.m[2][2] * v[2],
        ]
    }

    /// 行向量右乘：l * M
    #[inline]
    pub fn mul_cov(&self, l: [f64; 3]) -> [f64; 3] {
        [
            l[0] * self.m[0][0] + l[1] * self.m[1][0] + l[2] * self.m[2][0],
            l[0] * self.m[0][1] + l[1] * self.m[1][1] + l[2] * self.m[2][1],
            l[0] * self.m[0][2] + l[1] * self.m[1][2] + l[2] * self.m[2][2],
        ]
    }

    #[inline]
    pub fn transpose(&self) -> Self {
        Self {
            m: [
                [self.m[0][0], self.m[1][0], self.m[2][0]],
                [self.m[0][1], self.m[1][1], self.m[2][1]],
                [self.m[0][2], self.m[1][2], self.m[2][2]],
            ],
        }
    }

    pub fn inverse(&self) -> Option<Self> {
        let a = &self.m;
        let c00 = a[1][1] * a[2][2] - a[1][2] * a[2][1];
        let c01 = -(a[1][0] * a[2][2] - a[1][2] * a[2][0]);
        let c02 = a[1][0] * a[2][1] - a[1][1] * a[2][0];

        let det = a[0][0] * c00 + a[0][1] * c01 + a[0][2] * c02;
        if det.abs() < 1e-18 {
            return None;
        }
        let inv_det = 1.0 / det;

        let c10 = -(a[0][1] * a[2][2] - a[0][2] * a[2][1]);
        let c11 = a[0][0] * a[2][2] - a[0][2] * a[2][0];
        let c12 = -(a[0][0] * a[2][1] - a[0][1] * a[2][0]);

        let c20 = a[0][1] * a[1][2] - a[0][2] * a[1][1];
        let c21 = -(a[0][0] * a[1][2] - a[0][2] * a[1][0]);
        let c22 = a[0][0] * a[1][1] - a[0][1] * a[1][0];

        Some(Self {
            m: [
                [c00 * inv_det, c10 * inv_det, c20 * inv_det],
                [c01 * inv_det, c11 * inv_det, c21 * inv_det],
                [c02 * inv_det, c12 * inv_det, c22 * inv_det],
            ],
        })
    }
}

/// 射影画布相机：世界射影空间 -> 画布 NDC [-1, 1]^2
#[derive(Clone, Debug)]
pub struct ProjectiveCamera {
    /// 视口中心（世界坐标）
    pub center: [f64; 2],
    /// 视口半高（世界坐标单位）
    pub half_height: f64,
    /// 画布射影变换矩阵（默认单位阵 = 不做射影畸变）
    pub proj_matrix: Mat3F64,
}

impl Default for ProjectiveCamera {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            half_height: 4.0,
            proj_matrix: Mat3F64::IDENTITY,
        }
    }
}

impl ProjectiveCamera {
    pub fn pan_by_ndc_delta(&mut self, dx_ndc: f64, dy_ndc: f64, aspect: f64) {
        self.center[0] -= dx_ndc * self.half_height * aspect;
        self.center[1] -= dy_ndc * self.half_height;
    }

    pub fn zoom_by(&mut self, factor: f64) {
        self.half_height = (self.half_height / factor).clamp(1e-6, 1e6);
    }

    /// 计算 (H, H^-1)，将世界齐次点映射到 NDC [-1, 1]^2
    pub fn ndc_transforms(&self, aspect: f64) -> (Mat3F64, Mat3F64) {
        let sx = 1.0 / (self.half_height * aspect.max(1e-6));
        let sy = 1.0 / self.half_height;
        let view = Mat3F64 {
            m: [
                [sx, 0.0, -self.center[0] * sx],
                [0.0, sy, -self.center[1] * sy],
                [0.0, 0.0, 1.0],
            ],
        };
        let h = view.mul_mat(&self.proj_matrix);
        let h_inv = h.inverse().unwrap_or(Mat3F64::IDENTITY);
        (h, h_inv)
    }

    /// 在 CPU 上用 f64 完成射影变换、直线 NDC 截断挤出，并生成 GPU 帧数据
    pub fn prepare_draw_data(
        &self,
        scene: &CanvasScene,
        viewport_px: [f32; 2],
        pixels_per_point: f32,
    ) -> PreparedDrawData {
        let w_px = viewport_px[0].max(1.0);
        let h_px = viewport_px[1].max(1.0);
        let aspect = (w_px as f64) / (h_px as f64);

        let (h, h_inv) = self.ndc_transforms(aspect);
        let h_inv_t = h_inv.transpose();

        let line_half_width_px = 0.85 * pixels_per_point;
        let point_radius_px = 3.5 * pixels_per_point;

        // 1. 直线：l' = l * H^-1 -> NDC 矩形截断 -> 沿像素法线挤出为两个三角形
        let mut line_vertices = Vec::with_capacity(scene.lines.len() * 6);
        for &line in &scene.lines {
            let l_ndc = h_inv.mul_cov(line);
            if let Some((p0, p1)) = clip_line_to_ndc(l_ndc) {
                extrude_line_quad(p0, p1, w_px, h_px, line_half_width_px, &mut line_vertices);
            }
        }

        // 2. 圆锥曲线：Q' = (H^-1)^T * Q * H^-1 -> 归一化后传给 GPU 隐式片元着色器
        let mut conic_instances = Vec::with_capacity(scene.conics.len());
        for &q in &scene.conics {
            let q_mat = Mat3F64 { m: q };
            let q_ndc = h_inv_t.mul_mat(&q_mat).mul_mat(&h_inv);
            // 强制对称化，消除浮点误差
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
                });
            }
        }

        // 3. 点：p' = H * p -> 透视除法得到 NDC
        let mut point_instances = Vec::with_capacity(scene.dots.len());
        for &dot in &scene.dots {
            let p = h.mul_vec(dot);
            if p[2].abs() > 1e-12 {
                let x = (p[0] / p[2]) as f32;
                let y = (p[1] / p[2]) as f32;
                if x.abs() <= 1.2 && y.abs() <= 1.2 {
                    point_instances.push(PointInstance { center_ndc: [x, y] });
                }
            }
        }

        PreparedDrawData {
            uniforms: CanvasUniforms {
                viewport_px: [w_px, h_px],
                line_half_width_px,
                point_radius_px,
            },
            line_vertices,
            conic_instances,
            point_instances,
        }
    }
}

/// 将 NDC 齐次直线 a*x + b*y + c = 0 截断到 [-1.02, 1.02]^2
fn clip_line_to_ndc(l: [f64; 3]) -> Option<([f32; 2], [f32; 2])> {
    let [a, b, c] = l;
    if a * a + b * b < 1e-24 {
        return None; // 无穷远直线在当前视图下不可见
    }

    const M: f64 = 1.02;
    let mut pts = [[0.0_f64; 2]; 4];
    let mut count = 0usize;

    if b.abs() > 1e-12 {
        let y_left = -(c - a * M) / b;
        if y_left.abs() <= M {
            pts[count] = [-M, y_left];
            count += 1;
        }
        let y_right = -(c + a * M) / b;
        if y_right.abs() <= M {
            pts[count] = [M, y_right];
            count += 1;
        }
    }

    if a.abs() > 1e-12 {
        let x_bottom = -(c - b * M) / a;
        if x_bottom.abs() <= M {
            pts[count] = [x_bottom, -M];
            count += 1;
        }
        let x_top = -(c + b * M) / a;
        if x_top.abs() <= M {
            pts[count] = [x_top, M];
            count += 1;
        }
    }

    if count < 2 {
        return None;
    }

    // 挑出距离最远的两个交点（自动消除角点重复相交）
    let mut best_pair = None;
    let mut best_d2 = 0.0_f64;
    for i in 0..count {
        for j in (i + 1)..count {
            let dx = pts[j][0] - pts[i][0];
            let dy = pts[j][1] - pts[i][1];
            let d2 = dx * dx + dy * dy;
            if d2 > best_d2 {
                best_d2 = d2;
                best_pair = Some((
                    [pts[i][0] as f32, pts[i][1] as f32],
                    [pts[j][0] as f32, pts[j][1] as f32],
                ));
            }
        }
    }

    if best_d2 > 1e-12 { best_pair } else { None }
}

/// 沿像素法线方向将线段挤出为 2 个三角形（6 个顶点）
fn extrude_line_quad(
    p0: [f32; 2],
    p1: [f32; 2],
    w_px: f32,
    h_px: f32,
    half_stroke_px: f32,
    out: &mut Vec<LineVertex>,
) {
    let half_w = 0.5 * w_px;
    let half_h = 0.5 * h_px;

    let dx_px = (p1[0] - p0[0]) * half_w;
    let dy_px = (p1[1] - p0[1]) * half_h;
    let len_px = (dx_px * dx_px + dy_px * dy_px).sqrt();
    if len_px < 1e-4 {
        return;
    }

    let nx_px = -dy_px / len_px;
    let ny_px = dx_px / len_px;

    let half_ext_px = half_stroke_px + 1.0; // 留 1px 抗锯齿羽化边距
    let off_x = nx_px * half_ext_px / half_w;
    let off_y = ny_px * half_ext_px / half_h;

    let v0_pos = LineVertex {
        pos_ndc: [p0[0] + off_x, p0[1] + off_y],
        edge_dist_px: half_ext_px,
    };
    let v0_neg = LineVertex {
        pos_ndc: [p0[0] - off_x, p0[1] - off_y],
        edge_dist_px: -half_ext_px,
    };
    let v1_pos = LineVertex {
        pos_ndc: [p1[0] + off_x, p1[1] + off_y],
        edge_dist_px: half_ext_px,
    };
    let v1_neg = LineVertex {
        pos_ndc: [p1[0] - off_x, p1[1] - off_y],
        edge_dist_px: -half_ext_px,
    };

    out.extend_from_slice(&[v0_pos, v1_pos, v0_neg, v0_neg, v1_pos, v1_neg]);
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CanvasUniforms {
    pub viewport_px: [f32; 2],
    pub line_half_width_px: f32,
    pub point_radius_px: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub pos_ndc: [f32; 2],
    pub edge_dist_px: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ConicInstance {
    pub quad_abc: [f32; 3],
    pub quad_def: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    pub center_ndc: [f32; 2],
}

#[derive(Clone)]
pub struct PreparedDrawData {
    pub uniforms: CanvasUniforms,
    pub line_vertices: Vec<LineVertex>,
    pub conic_instances: Vec<ConicInstance>,
    pub point_instances: Vec<PointInstance>,
}