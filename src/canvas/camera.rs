/// 3x3 实矩阵（行优先），用于视图与射影变换。
#[derive(Clone, Copy, Debug)]
pub struct Mat3F64 {
    pub m: [[f64; 3]; 3],
}

impl Mat3F64 {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    /// 前视视角预设（底行 `[0, 0.35, 1]`）。
    pub const FRONT_VIEW: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.35, 1.0]],
    };

    /// 侧视视角预设（底行 `[0.35, 0, 1]`）。
    pub const SIDE_VIEW: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.35, 0.0, 1.0]],
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
    pub center: [f64; 2],
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

    /// 以给定屏幕 NDC 坐标 `focus_ndc` 为锚点进行缩放（保持该屏幕位置下的几何点不动）
    pub fn zoom_by_at_ndc(&mut self, factor: f64, focus_ndc: [f64; 2], aspect: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let old_hh = self.half_height;
        let new_hh = (old_hh / factor).clamp(1e-6, 1e6);
        let d_hh = old_hh - new_hh;
        self.center[0] += focus_ndc[0] * aspect.max(1e-6) * d_hh;
        self.center[1] += focus_ndc[1] * d_hh;
        self.half_height = new_hh;
    }

    /// 根据屏幕 NDC 拖拽增量转动射影视角（直接作用在 `proj_matrix` 本体，不引入隐藏矩阵）：
    /// `H_new = T_ndc * H = V * (V^-1 * T_ndc * V * proj_matrix)`
    pub fn tilt_by_ndc_delta(&mut self, dx_ndc: f64, dy_ndc: f64, aspect: f64) {
        if !dx_ndc.is_finite() || !dy_ndc.is_finite() {
            return;
        }
        let sx = 1.0 / (self.half_height * aspect.max(1e-6));
        let sy = 1.0 / self.half_height;
        let view = Mat3F64 {
            m: [
                [sx, 0.0, -self.center[0] * sx],
                [0.0, sy, -self.center[1] * sy],
                [0.0, 0.0, 1.0],
            ],
        };
        let view_inv = Mat3F64 {
            m: [
                [1.0 / sx, 0.0, self.center[0]],
                [0.0, 1.0 / sy, self.center[1]],
                [0.0, 0.0, 1.0],
            ],
        };

        // 灵敏度：拖拽半个画布宽度 (dx_ndc = 1.0) 使灭线恰好进入画面边缘 (x_ndc = 1.0)
        const TILT_SENSITIVITY: f64 = 1.0;
        let kx = dx_ndc * TILT_SENSITIVITY;
        let ky = dy_ndc * TILT_SENSITIVITY;

        let t_ndc = Mat3F64 {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [kx, ky, 1.0]],
        };

        let mut p_new = view_inv
            .mul_mat(&t_ndc)
            .mul_mat(&view)
            .mul_mat(&self.proj_matrix);

        // 保持齐次矩阵量级处于数值健康区间（兼容 UI DragValue [-100, 100]，不改变几何含义）
        let mut max_abs = 0.0_f64;
        for row in p_new.m {
            for v in row {
                max_abs = max_abs.max(v.abs());
            }
        }
        if max_abs > 20.0 || (max_abs > 1e-18 && max_abs < 0.05) {
            let s = 1.0 / max_abs;
            for row in &mut p_new.m {
                for v in row {
                    *v *= s;
                }
            }
        }

        self.proj_matrix = p_new;
    }

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
}

/// 将 NDC 齐次直线 a*x + b*y + c = 0 截断到 [-1.02, 1.02]^2
pub fn clip_line_to_ndc(l: [f64; 3]) -> Option<([f32; 2], [f32; 2])> {
    let [a, b, c] = l;
    if a * a + b * b < 1e-24 {
        return None;
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
pub fn extrude_line_quad(
    p0: [f32; 2],
    p1: [f32; 2],
    w_px: f32,
    h_px: f32,
    half_width_px: f32,
    color: [f32; 4],
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

    let half_ext_px = half_width_px + 1.0;
    let off_x = nx_px * half_ext_px / half_w;
    let off_y = ny_px * half_ext_px / half_h;

    let v0_pos = LineVertex {
        pos_ndc: [p0[0] + off_x, p0[1] + off_y],
        edge_dist_px: half_ext_px,
        half_width_px,
        color,
    };
    let v0_neg = LineVertex {
        pos_ndc: [p0[0] - off_x, p0[1] - off_y],
        edge_dist_px: -half_ext_px,
        half_width_px,
        color,
    };
    let v1_pos = LineVertex {
        pos_ndc: [p1[0] + off_x, p1[1] + off_y],
        edge_dist_px: half_ext_px,
        half_width_px,
        color,
    };
    let v1_neg = LineVertex {
        pos_ndc: [p1[0] - off_x, p1[1] - off_y],
        edge_dist_px: -half_ext_px,
        half_width_px,
        color,
    };

    out.extend_from_slice(&[v0_pos, v1_pos, v0_neg, v0_neg, v1_pos, v1_neg]);
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CanvasUniforms {
    pub viewport_px: [f32; 2],
    pub show_grid: u32,
    pub minor_line_px: f32,
    /// NDC -> 次网格单位坐标系的 3x3 逆变换矩阵（按列存储，xyz 有效）
    pub grid_inv_col0: [f32; 4],
    pub grid_inv_col1: [f32; 4],
    pub grid_inv_col2: [f32; 4],
    pub grid_minor_color: [f32; 4],
    pub grid_major_color: [f32; 4],
    pub major_line_px: f32,
    pub major_every: f32,
    pub _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub pos_ndc: [f32; 2],
    pub edge_dist_px: f32,
    pub half_width_px: f32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ConicInstance {
    pub quad_abc: [f32; 3],
    pub quad_def: [f32; 3],
    pub half_width_px: f32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    pub center_ndc: [f32; 2],
    pub radius_px: f32,
    pub color: [f32; 4],
    /// 形状编号（见 `DotShape::shader_code`）：0 实心圆 1 空心圆环 2 十字 3 小圆。
    pub shape: u32,
    /// 选中态：绘制同色半透明光晕 + 边缘细环。
    pub selected: u32,
}

#[derive(Clone)]
pub struct PreparedDrawData {
    pub uniforms: CanvasUniforms,
    /// 坐标轴顶点数（渲染器据此把直线缓冲拆成两段：坐标轴 → 圆锥曲线 → 用户直线 → 点）
    pub axis_vertex_count: usize,
    pub line_vertices: Vec<LineVertex>,
    pub conic_instances: Vec<ConicInstance>,
    pub point_instances: Vec<PointInstance>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_respects_existing_rotation_and_keeps_center_pinned() {
        let mut cam = ProjectiveCamera::default();
        cam.center = [10.0, -5.0];
        cam.half_height = 2.0;
        // 预先在 proj_matrix 中放入 90 度逆时针旋转
        cam.proj_matrix = Mat3F64 {
            m: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        };

        let aspect = 1.5;
        let (_, h_inv_before) = cam.ndc_transforms(aspect);
        // 当前屏幕中心 [0, 0] 对应的世界坐标
        let center_w = h_inv_before.mul_vec([0.0, 0.0, 1.0]);

        // 向屏幕右方拖拽右键
        cam.tilt_by_ndc_delta(1.0, 0.0, aspect);
        let (h_after, _) = cam.ndc_transforms(aspect);

        // 1. 屏幕中心世界点依然停留在屏幕中心 [0, 0]
        let p_center = h_after.mul_vec(center_w);
        assert!((p_center[0] / p_center[2]).abs() < 1e-12);
        assert!((p_center[1] / p_center[2]).abs() < 1e-12);

        // 2. 沿屏幕 +X 方向（未旋转前对应世界 -Y）的无穷远点应落在屏幕右边缘 x_ndc = +1.0
        let (h_before, _) = {
            let mut c0 = cam.clone();
            c0.proj_matrix = Mat3F64 {
                m: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            };
            c0.ndc_transforms(aspect)
        };
        let inf_dir_world = h_before.inverse().unwrap().mul_vec([1.0, 0.0, 0.0]);
        let vp_ndc = h_after.mul_vec(inf_dir_world);
        assert!((vp_ndc[0] / vp_ndc[2] - 1.0).abs() < 1e-12);
        assert!((vp_ndc[1] / vp_ndc[2]).abs() < 1e-12);
    }

    #[test]
    fn zoom_by_at_ndc_pins_focus_point() {
        let mut cam = ProjectiveCamera::default();
        let aspect = 4.0 / 3.0;
        let focus = [0.6, -0.4];
        let (_, h_inv) = cam.ndc_transforms(aspect);
        let target_w = h_inv.mul_vec([focus[0], focus[1], 1.0]);

        cam.zoom_by_at_ndc(2.5, focus, aspect);
        let (h_new, _) = cam.ndc_transforms(aspect);
        let mapped = h_new.mul_vec(target_w);
        assert!((mapped[0] / mapped[2] - focus[0]).abs() < 1e-12);
        assert!((mapped[1] / mapped[2] - focus[1]).abs() < 1e-12);
    }
}
