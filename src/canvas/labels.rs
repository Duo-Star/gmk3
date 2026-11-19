use crate::canvas::camera::{Mat3F64, ProjectiveCamera};
use crate::canvas::plotter::{MAJOR_EVERY, grid_minor_step};

/// 标签文字基准颜色（与 COORDINATE / AXIS 协调的高可读浅灰白）
const TICK_LABEL_COLOR: [f32; 4] = [0.78, 0.78, 0.80, 0.92];
const AXIS_NAME_COLOR: [f32; 4] = [0.92, 0.92, 0.94, 0.96];

/// 字号（逻辑点 pt）
const TICK_FONT_SIZE_PT: f32 = 11.5;
const AXIS_NAME_FONT_SIZE_PT: f32 = 13.0;

/// 灭线渐隐区间（逻辑点 pt）：与 GPU Nyquist 均质淡出区间衔接
const FADE_START_PT: f64 = 12.0;
const FADE_FULL_PT: f64 = 28.0;

/// 射影求解器沿线段的硬性最小步距与单段迭代上限（防爆保险）
const MIN_SEG_SPACING_PT: f64 = 10.0;
const MAX_TICKS_PER_SEGMENT: usize = 256;
const MAX_GRID_INDEX: f64 = 1e14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    CenterCenter,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelKind {
    AxisTick,
    AxisName,
    ObjectName,
}

/// 纯数据画布标签（与 UI 框架解耦；egui / glyphon 均可直接消费）
#[derive(Clone, Debug)]
pub struct CanvasLabel {
    /// 锚点在 NDC [-1, 1]^2 中的位置（x: 左-1..右+1, y: 下-1..上+1）
    pub pos_ndc: [f32; 2],
    /// 屏幕空间偏移（逻辑点 pt，+x 向右，+y 向下）
    pub offset_pt: [f32; 2],
    pub anchor: LabelAnchor,
    pub text: String,
    /// 基础颜色 RGBA（sRGB 归一化 [0, 1]）
    pub color: [f32; 4],
    /// 灭线渐隐系数 [0.0, 1.0]，渲染端用 gamma_multiply(alpha) 叠加
    pub alpha: f32,
    pub font_size_pt: f32,
    pub kind: LabelKind,
}

#[derive(Clone, Copy)]
enum AxisKind {
    X,
    Y,
}

/// 在 f64 下将 NDC 齐次直线 a*x + b*y + c = 0 截断到 [-1.0, 1.0]^2
fn clip_line_to_unit_ndc(l: [f64; 3]) -> Option<([f64; 2], [f64; 2])> {
    let [a, b, c] = l;
    if a * a + b * b < 1e-24 {
        return None;
    }

    let mut pts = [[0.0_f64; 2]; 4];
    let mut count = 0usize;
    const M: f64 = 1.0;

    if b.abs() > 1e-12 {
        let y_left = -(c - a * M) / b;
        if y_left.abs() <= M + 1e-12 {
            pts[count] = [-M, y_left.clamp(-M, M)];
            count += 1;
        }
        let y_right = -(c + a * M) / b;
        if y_right.abs() <= M + 1e-12 {
            pts[count] = [M, y_right.clamp(-M, M)];
            count += 1;
        }
    }

    if a.abs() > 1e-12 {
        let x_bottom = -(c - b * M) / a;
        if x_bottom.abs() <= M + 1e-12 {
            pts[count] = [x_bottom.clamp(-M, M), -M];
            count += 1;
        }
        let x_top = -(c + b * M) / a;
        if x_top.abs() <= M + 1e-12 {
            pts[count] = [x_top.clamp(-M, M), M];
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
                best_pair = Some((pts[i], pts[j]));
            }
        }
    }

    if best_d2 > 1e-12 { best_pair } else { None }
}

/// NDC 点到逻辑屏幕像素（左上角为 (0,0)，+x 向右，+y 向下）
#[inline]
fn ndc_to_pt(ndc: [f64; 2], w_pt: f64, h_pt: f64) -> [f64; 2] {
    [(ndc[0] * 0.5 + 0.5) * w_pt, (0.5 - ndc[1] * 0.5) * h_pt]
}

#[inline]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 在给定 NDC 位置评估对应轴向主网格线的灭线透明度（与 shaders.wgsl 逻辑对称）
fn eval_vanishing_fade(
    ndc: [f64; 2],
    h_inv: &Mat3F64,
    w_pt: f64,
    h_pt: f64,
    major_step: f64,
    axis: AxisKind,
) -> f32 {
    let m = &h_inv.m;
    let c0 = [m[0][0], m[1][0], m[2][0]];
    let c1 = [m[0][1], m[1][1], m[2][1]];
    let c2 = [m[0][2], m[1][2], m[2][2]];

    let wx = c0[0] * ndc[0] + c1[0] * ndc[1] + c2[0];
    let wy = c0[1] * ndc[0] + c1[1] * ndc[1] + c2[1];
    let wz = c0[2] * ndc[0] + c1[2] * ndc[1] + c2[2];

    let ndc_per_pt_x = 2.0 / w_pt.max(1.0);
    let ndc_per_pt_y = 2.0 / h_pt.max(1.0);

    let dw_dx = [
        c0[0] * ndc_per_pt_x,
        c0[1] * ndc_per_pt_x,
        c0[2] * ndc_per_pt_x,
    ];
    let dw_dy = [
        c1[0] * ndc_per_pt_y,
        c1[1] * ndc_per_pt_y,
        c1[2] * ndc_per_pt_y,
    ];

    let grad_wz_pt = (dw_dx[2] * dw_dx[2] + dw_dy[2] * dw_dy[2]).sqrt();
    if wz.abs() <= grad_wz_pt.max(1e-12) {
        return 0.0;
    }

    let inv_wz2 = 1.0 / (wz * wz);
    let grad_coord_pt = match axis {
        AxisKind::X => {
            let du_dx = (wz * dw_dx[0] - wx * dw_dx[2]) * inv_wz2;
            let du_dy = (wz * dw_dy[0] - wx * dw_dy[2]) * inv_wz2;
            (du_dx * du_dx + du_dy * du_dy).sqrt()
        }
        AxisKind::Y => {
            let dv_dx = (wz * dw_dx[1] - wy * dw_dx[2]) * inv_wz2;
            let dv_dy = (wz * dw_dy[1] - wy * dw_dy[2]) * inv_wz2;
            (dv_dx * dv_dx + dv_dy * dv_dy).sqrt()
        }
    };

    if grad_coord_pt <= 1e-18 {
        return 1.0;
    }

    let spacing_pt = major_step / grad_coord_pt;
    smoothstep(FADE_START_PT, FADE_FULL_PT, spacing_pt) as f32
}

/// 紧凑格式化坐标数值（消除浮点尾差与 -0）
pub fn format_tick_value(k: i64, major_step: f64, minor_step: f64) -> String {
    if k == 0 {
        return "0".to_owned();
    }
    let val = (k as f64) * major_step;
    if !val.is_finite() || val.abs() < major_step * 1e-9 {
        return "0".to_owned();
    }

    let abs_val = val.abs();
    if abs_val >= 1e6 || abs_val < 1e-4 {
        let exp = abs_val.log10().floor() as i32;
        let scaled = val / 10f64.powi(exp);
        let mut s = format!("{scaled:.2}");
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        return format!("{s}e{exp}");
    }

    // minor_step = {1, 2, 5} * 10^e，由此确定最多需要保留的小数位数
    let minor_exp = minor_step.log10().floor() as i32;
    let decimals = (-minor_exp).max(0) as usize;
    let mut s = format!("{val:.decimals$}");
    if decimals > 0 && s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" { "0".to_owned() } else { s }
}

/// 估计标签在屏幕空间（逻辑点）的碰撞包围盒 [min_x, min_y, max_x, max_y]
fn estimate_label_rect(
    pos_ndc: [f32; 2],
    offset_pt: [f32; 2],
    anchor: LabelAnchor,
    text: &str,
    font_size_pt: f32,
    w_pt: f32,
    h_pt: f32,
) -> [f32; 4] {
    let px = (pos_ndc[0] * 0.5 + 0.5) * w_pt + offset_pt[0];
    let py = (0.5 - pos_ndc[1] * 0.5) * h_pt + offset_pt[1];

    // ASCII 数字与符号的等宽/比例平均字宽近似，留 6pt 横向安全间隔
    let est_w = (text.chars().count().max(1) as f32) * (font_size_pt * 0.60) + 6.0;
    let est_h = font_size_pt + 4.0;

    let (min_x, max_x) = match anchor {
        LabelAnchor::TopLeft | LabelAnchor::CenterLeft | LabelAnchor::BottomLeft => {
            (px, px + est_w)
        }
        LabelAnchor::TopCenter | LabelAnchor::CenterCenter | LabelAnchor::BottomCenter => {
            (px - 0.5 * est_w, px + 0.5 * est_w)
        }
        LabelAnchor::TopRight | LabelAnchor::CenterRight | LabelAnchor::BottomRight => {
            (px - est_w, px)
        }
    };

    let (min_y, max_y) = match anchor {
        LabelAnchor::TopLeft | LabelAnchor::TopCenter | LabelAnchor::TopRight => (py, py + est_h),
        LabelAnchor::CenterLeft | LabelAnchor::CenterCenter | LabelAnchor::CenterRight => {
            (py - 0.5 * est_h, py + 0.5 * est_h)
        }
        LabelAnchor::BottomLeft | LabelAnchor::BottomCenter | LabelAnchor::BottomRight => {
            (py - est_h, py)
        }
    };

    [min_x, min_y, max_x, max_y]
}

#[inline]
fn rects_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

fn try_push_label(
    label: CanvasLabel,
    w_pt: f32,
    h_pt: f32,
    placed_rects: &mut Vec<[f32; 4]>,
    out: &mut Vec<CanvasLabel>,
) -> bool {
    let rect = estimate_label_rect(
        label.pos_ndc,
        label.offset_pt,
        label.anchor,
        &label.text,
        label.font_size_pt,
        w_pt,
        h_pt,
    );
    if placed_rects.iter().any(|&r| rects_overlap(rect, r)) {
        return false;
    }
    placed_rects.push(rect);
    out.push(label);
    true
}

#[allow(clippy::too_many_arguments)]
fn emit_ticks_on_segment(
    p0: [f64; 2],
    p1: [f64; 2],
    h_inv: &Mat3F64,
    w_pt: f64,
    h_pt: f64,
    major_step: f64,
    minor_step: f64,
    axis: AxisKind,
    offset_pt: [f32; 2],
    anchor: LabelAnchor,
    skip_zero: bool,
    placed_rects: &mut Vec<[f32; 4]>,
    out: &mut Vec<CanvasLabel>,
) {
    let pt0 = ndc_to_pt(p0, w_pt, h_pt);
    let pt1 = ndc_to_pt(p1, w_pt, h_pt);
    let total_len_pt = ((pt1[0] - pt0[0]).powi(2) + (pt1[1] - pt0[1]).powi(2)).sqrt();
    if total_len_pt < 4.0 {
        return;
    }

    let w0 = h_inv.mul_vec([p0[0], p0[1], 1.0]);
    let w1 = h_inv.mul_vec([p1[0], p1[1], 1.0]);

    let (a0, a1) = match axis {
        AxisKind::X => (w0[0] / major_step, w1[0] / major_step),
        AxisKind::Y => (w0[1] / major_step, w1[1] / major_step),
    };
    let (b0, b1) = (w0[2], w1[2]);

    // 若线段跨过地平线 w_z = 0，拆成两段单调区间分别求解
    let mut sub_ranges = [(0.0_f64, 1.0_f64); 2];
    let mut sub_count = 1usize;
    if b0 * b1 < 0.0 && (b0 - b1).abs() > 1e-18 {
        let t_inf = b0 / (b0 - b1);
        let eps = 1e-6;
        sub_count = 0;
        if t_inf - eps > 0.0 {
            sub_ranges[sub_count] = (0.0, t_inf - eps);
            sub_count += 1;
        }
        if t_inf + eps < 1.0 {
            sub_ranges[sub_count] = (t_inf + eps, 1.0);
            sub_count += 1;
        }
    }

    for &(t_raw_a, t_raw_b) in &sub_ranges[..sub_count] {
        let a_a = (1.0 - t_raw_a) * a0 + t_raw_a * a1;
        let b_a = (1.0 - t_raw_a) * b0 + t_raw_a * b1;
        let a_b = (1.0 - t_raw_b) * a0 + t_raw_b * a1;
        let b_b = (1.0 - t_raw_b) * b0 + t_raw_b * b1;

        let sub_len_pt = total_len_pt * (t_raw_b - t_raw_a).abs();
        let delta = a_b * b_a - a_a * b_b;
        if delta.abs() < 1e-15 || sub_len_pt < 2.0 {
            continue;
        }

        // 保证只保留 |b(t)| >= b_min 的可视非爆炸区（总整数刻度严格有界）
        let b_min = (MIN_SEG_SPACING_PT * delta.abs() / sub_len_pt).sqrt();

        // 在 s in [0, 1] 上 b(s) = b_a + (b_b - b_a) * s 保持同号
        let sign_b = if b_a.abs() >= b_b.abs() {
            b_a.signum()
        } else {
            b_b.signum()
        };
        if sign_b == 0.0 {
            continue;
        }
        let ba_s = b_a * sign_b;
        let bb_s = b_b * sign_b;

        let (s_lo, s_hi) = if (bb_s - ba_s).abs() < 1e-15 {
            if ba_s < b_min {
                continue;
            }
            (0.0, 1.0)
        } else {
            let s_cut = (b_min - ba_s) / (bb_s - ba_s);
            if bb_s > ba_s {
                if bb_s < b_min {
                    continue;
                }
                (s_cut.clamp(0.0, 1.0), 1.0)
            } else {
                if ba_s < b_min {
                    continue;
                }
                (0.0, s_cut.clamp(0.0, 1.0))
            }
        };

        if s_hi - s_lo <= 1e-9 {
            continue;
        }

        let t0 = t_raw_a + s_lo * (t_raw_b - t_raw_a);
        let t1 = t_raw_a + s_hi * (t_raw_b - t_raw_a);

        // 从稀疏端（|b| 较大）向密集端（|b| 较小）推进，优先保留最清晰的刻度
        let b_t0 = ((1.0 - t0) * b0 + t0 * b1).abs();
        let b_t1 = ((1.0 - t1) * b0 + t1 * b1).abs();
        let (t_sparse, t_dense) = if b_t0 >= b_t1 { (t0, t1) } else { (t1, t0) };

        let a_s = (1.0 - t_sparse) * a0 + t_sparse * a1;
        let b_s = (1.0 - t_sparse) * b0 + t_sparse * b1;
        let a_d = (1.0 - t_dense) * a0 + t_dense * a1;
        let b_d = (1.0 - t_dense) * b0 + t_dense * b1;

        if b_s.abs() < 1e-15 || b_d.abs() < 1e-15 {
            continue;
        }

        let u_s = a_s / b_s;
        let u_d = a_d / b_d;
        if !u_s.is_finite()
            || !u_d.is_finite()
            || u_s.abs() > MAX_GRID_INDEX
            || u_d.abs() > MAX_GRID_INDEX
        {
            continue;
        }

        let dir: i64 = if u_d >= u_s { 1 } else { -1 };
        let mut k = if dir > 0 {
            u_s.ceil() as i64
        } else {
            u_s.floor() as i64
        };

        for _ in 0..MAX_TICKS_PER_SEGMENT {
            let kf = k as f64;
            if (dir > 0 && kf > u_d + 1e-9) || (dir < 0 && kf < u_d - 1e-9) {
                break;
            }
            if !(skip_zero && k == 0) {
                let denom = (a_d - a_s) - kf * (b_d - b_s);
                if denom.abs() > 1e-15 {
                    let s_tick = (kf * b_s - a_s) / denom;
                    if (-1e-6..=1.0 + 1e-6).contains(&s_tick) {
                        let s_clamped = s_tick.clamp(0.0, 1.0);
                        let t = t_sparse + s_clamped * (t_dense - t_sparse);
                        let ndc = [
                            (1.0 - t) * p0[0] + t * p1[0],
                            (1.0 - t) * p0[1] + t * p1[1],
                        ];
                        let fade = eval_vanishing_fade(ndc, h_inv, w_pt, h_pt, major_step, axis);
                        if fade > 0.01 {
                            let text = format_tick_value(k, major_step, minor_step);
                            let label = CanvasLabel {
                                pos_ndc: [ndc[0] as f32, ndc[1] as f32],
                                offset_pt,
                                anchor,
                                text,
                                color: TICK_LABEL_COLOR,
                                alpha: fade,
                                font_size_pt: TICK_FONT_SIZE_PT,
                                kind: LabelKind::AxisTick,
                            };
                            try_push_label(
                                label,
                                w_pt as f32,
                                h_pt as f32,
                                placed_rects,
                                out,
                            );
                        }
                    }
                }
            }
            k = k.saturating_add(dir);
        }
    }
}

/// 根据世界坐标轴在 NDC 中的可见段，计算贴轴偏移（X 轴下方 / Y 轴左侧，近边缘自动内缩防裁切）
fn compute_axis_tick_placement(
    p0: [f64; 2],
    p1: [f64; 2],
    w_pt: f64,
    h_pt: f64,
    axis: AxisKind,
) -> ([f32; 2], LabelAnchor) {
    let pt0 = ndc_to_pt(p0, w_pt, h_pt);
    let pt1 = ndc_to_pt(p1, w_pt, h_pt);
    let dx = pt1[0] - pt0[0];
    let dy = pt1[1] - pt0[1];
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);

    // 垂直于轴的单位法向量（屏幕 pt 空间，+y 向下）
    let mut nx = -dy / len;
    let mut ny = dx / len;

    match axis {
        AxisKind::X => {
            // 优先选朝向屏幕下方（ny > 0）；近垂直时选右侧
            if ny < -1e-3 || (ny.abs() <= 1e-3 && nx < 0.0) {
                nx = -nx;
                ny = -ny;
            }
            // 若轴中点贴近屏幕底边，自动翻到轴上方以防文字被视口裁掉
            let mid_y_pt = 0.5 * (pt0[1] + pt1[1]);
            if mid_y_pt > h_pt - 20.0 && ny > 0.0 {
                nx = -nx;
                ny = -ny;
            }
            let offset = [(nx * 6.0) as f32, (ny * 6.0) as f32];
            let anchor = if ny > 0.35 {
                LabelAnchor::TopCenter
            } else if ny < -0.35 {
                LabelAnchor::BottomCenter
            } else if nx > 0.0 {
                LabelAnchor::CenterLeft
            } else {
                LabelAnchor::CenterRight
            };
            (offset, anchor)
        }
        AxisKind::Y => {
            // 优先选朝向屏幕左侧（nx < 0）；近水平时选下方
            if nx > 1e-3 || (nx.abs() <= 1e-3 && ny < 0.0) {
                nx = -nx;
                ny = -ny;
            }
            // 若轴中点贴近屏幕左边，自动翻到轴右侧以防长数字被裁切
            let mid_x_pt = 0.5 * (pt0[0] + pt1[0]);
            if mid_x_pt < 38.0 && nx < 0.0 {
                nx = -nx;
                ny = -ny;
            }
            let offset = [(nx * 7.0) as f32, (ny * 7.0) as f32];
            let anchor = if nx < -0.35 {
                LabelAnchor::CenterRight
            } else if nx > 0.35 {
                LabelAnchor::CenterLeft
            } else if ny > 0.0 {
                LabelAnchor::TopCenter
            } else {
                LabelAnchor::BottomCenter
            };
            (offset, anchor)
        }
    }
}

/// 在可见轴正端内侧生成 "X" / "Y" 轴名称标签（放在刻度数字的对侧以免碰撞）
fn emit_axis_name_label(
    p0: [f64; 2],
    p1: [f64; 2],
    h: &Mat3F64,
    h_inv: &Mat3F64,
    w_pt: f64,
    h_pt: f64,
    major_step: f64,
    axis: AxisKind,
    placed_rects: &mut Vec<[f32; 4]>,
    out: &mut Vec<CanvasLabel>,
) {
    let pt0 = ndc_to_pt(p0, w_pt, h_pt);
    let pt1 = ndc_to_pt(p1, w_pt, h_pt);

    // 优先放在屏幕右端（X轴）或上端（Y轴），若近乎垂直/水平则回退另一坐标
    let pick_p1 = match axis {
        AxisKind::X => {
            if (p1[0] - p0[0]).abs() > 1e-3 {
                p1[0] > p0[0]
            } else {
                p1[1] > p0[1]
            }
        }
        AxisKind::Y => {
            if (p1[1] - p0[1]).abs() > 1e-3 {
                p1[1] > p0[1]
            } else {
                p1[0] > p0[0]
            }
        }
    };

    let (end_ndc, other_ndc, end_pt, other_pt) = if pick_p1 {
        (p1, p0, pt1, pt0)
    } else {
        (p0, p1, pt0, pt1)
    };

    let dx = end_pt[0] - other_pt[0];
    let dy = end_pt[1] - other_pt[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 28.0 {
        return;
    }

    // 沿轴向内退 14pt，保证不贴死视口边框
    let back_t = (14.0 / len).min(0.25);
    let pos_ndc = [
        end_ndc[0] + back_t * (other_ndc[0] - end_ndc[0]),
        end_ndc[1] + back_t * (other_ndc[1] - end_ndc[1]),
    ];

    let fade = eval_vanishing_fade(pos_ndc, h_inv, w_pt, h_pt, major_step, axis);
    if fade <= 0.01 {
        return;
    }

    // 与刻度反侧放置："X" 放轴上方，"Y" 放轴右侧
    let ux = dx / len;
    let uy = dy / len;
    let mut nx = -uy;
    let mut ny = ux;
    let (text, offset, anchor) = match axis {
        AxisKind::X => {
            if ny > 1e-3 || (ny.abs() <= 1e-3 && nx > 0.0) {
                nx = -nx;
                ny = -ny;
            }
            if end_pt[1] < 22.0 && ny < 0.0 {
                nx = -nx;
                ny = -ny;
            }
            let off = [(nx * 7.0) as f32, (ny * 7.0) as f32];
            let anc = if ny < -0.35 {
                LabelAnchor::BottomCenter
            } else if ny > 0.35 {
                LabelAnchor::TopCenter
            } else if nx < 0.0 {
                LabelAnchor::CenterRight
            } else {
                LabelAnchor::CenterLeft
            };
            ("X".to_owned(), off, anc)
        }
        AxisKind::Y => {
            if nx < -1e-3 || (nx.abs() <= 1e-3 && ny > 0.0) {
                nx = -nx;
                ny = -ny;
            }
            if end_pt[0] > w_pt - 22.0 && nx > 0.0 {
                nx = -nx;
                ny = -ny;
            }
            let off = [(nx * 8.0) as f32, (ny * 8.0) as f32];
            let anc = if nx > 0.35 {
                LabelAnchor::CenterLeft
            } else if nx < -0.35 {
                LabelAnchor::CenterRight
            } else if ny < 0.0 {
                LabelAnchor::BottomCenter
            } else {
                LabelAnchor::TopCenter
            };
            ("Y".to_owned(), off, anc)
        }
    };

    // 若原世界正轴（+x / +y）也可见，优先作为小细节保留一致；此处仅确保位置与亮度一致
    let _ = h;
    let label = CanvasLabel {
        pos_ndc: [pos_ndc[0] as f32, pos_ndc[1] as f32],
        offset_pt: offset,
        anchor,
        text,
        color: AXIS_NAME_COLOR,
        alpha: fade,
        font_size_pt: AXIS_NAME_FONT_SIZE_PT,
        kind: LabelKind::AxisName,
    };
    try_push_label(label, w_pt as f32, h_pt as f32, placed_rects, out);
}

/// 在轴不可见时，根据 NDC 线到对应两条相对屏幕边的距离挑选更近的一条边
fn pick_closer_horizontal_edge(l_x_ndc: [f64; 3], h_inv: &Mat3F64) -> ([f64; 2], [f64; 2], [f32; 2], LabelAnchor) {
    let [a, b, c] = l_x_ndc;
    let norm2 = a * a + b * b;
    let use_bottom = if norm2 > 1e-18 {
        let d_bottom = (c - b).abs();
        let d_top = (c + b).abs();
        d_bottom <= d_top
    } else {
        let cw = h_inv.mul_vec([0.0, 0.0, 1.0]);
        cw[2] == 0.0 || (cw[1] / cw[2]) >= 0.0
    };

    if use_bottom {
        ([-1.0, -1.0], [1.0, -1.0], [0.0, -4.0], LabelAnchor::BottomCenter)
    } else {
        ([-1.0, 1.0], [1.0, 1.0], [0.0, 4.0], LabelAnchor::TopCenter)
    }
}

fn pick_closer_vertical_edge(l_y_ndc: [f64; 3], h_inv: &Mat3F64) -> ([f64; 2], [f64; 2], [f32; 2], LabelAnchor) {
    let [a, b, c] = l_y_ndc;
    let norm2 = a * a + b * b;
    let use_left = if norm2 > 1e-18 {
        let d_left = (c - a).abs();
        let d_right = (c + a).abs();
        d_left <= d_right
    } else {
        let cw = h_inv.mul_vec([0.0, 0.0, 1.0]);
        cw[2] == 0.0 || (cw[0] / cw[2]) >= 0.0
    };

    if use_left {
        ([-1.0, -1.0], [-1.0, 1.0], [5.0, 0.0], LabelAnchor::CenterLeft)
    } else {
        ([1.0, -1.0], [1.0, 1.0], [-5.0, 0.0], LabelAnchor::CenterRight)
    }
}

/// 生成本帧全部画布文字标签（追加到 `out` 中，便于外层复用内存）
pub fn collect_canvas_labels(
    camera: &ProjectiveCamera,
    show_grid: bool,
    show_axes: bool,
    viewport_px: [f32; 2],
    pixels_per_point: f32,
    out: &mut Vec<CanvasLabel>,
) {
    out.clear();
    if !show_grid && !show_axes {
        return;
    }

    let w_px = viewport_px[0].max(1.0);
    let h_px = viewport_px[1].max(1.0);
    let ppp = pixels_per_point.max(1e-3);
    let w_pt = (w_px / ppp) as f64;
    let h_pt = (h_px / ppp) as f64;
    if w_pt < 24.0 || h_pt < 24.0 {
        return;
    }

    let aspect = (w_px as f64) / (h_px as f64);
    let (h, h_inv) = camera.ndc_transforms(aspect);
    let minor_step = grid_minor_step(camera, aspect, viewport_px);
    let major_step = minor_step * (MAJOR_EVERY as f64);

    let l_x_ndc = h_inv.mul_cov([0.0, 1.0, 0.0]);
    let l_y_ndc = h_inv.mul_cov([1.0, 0.0, 0.0]);

    let x_seg = clip_line_to_unit_ndc(l_x_ndc);
    let y_seg = clip_line_to_unit_ndc(l_y_ndc);

    let mut placed_rects: Vec<[f32; 4]> = Vec::with_capacity(48);

    // 1. 若坐标轴开启，优先放置 "X" / "Y" 轴名称（放在刻度对侧）
    if show_axes {
        if let Some((p0, p1)) = x_seg {
            emit_axis_name_label(
                p0,
                p1,
                &h,
                &h_inv,
                w_pt,
                h_pt,
                major_step,
                AxisKind::X,
                &mut placed_rects,
                out,
            );
        }
        if let Some((p0, p1)) = y_seg {
            emit_axis_name_label(
                p0,
                p1,
                &h,
                &h_inv,
                w_pt,
                h_pt,
                major_step,
                AxisKind::Y,
                &mut placed_rects,
                out,
            );
        }
    }

    if !show_grid {
        return;
    }

    // 2. 若原点同时位于两可见轴交点处，只在第一条轴绘制一次 "0"，避免重复打架
    let origin_ndc = h.mul_vec([0.0, 0.0, 1.0]);
    let origin_in_view = origin_ndc[2].abs() > 1e-12
        && (origin_ndc[0] / origin_ndc[2]).abs() <= 1.0
        && (origin_ndc[1] / origin_ndc[2]).abs() <= 1.0;
    let skip_y_zero = x_seg.is_some() && y_seg.is_some() && origin_in_view;

    // 3. X 坐标数字：轴可见则贴 X 轴下方；不可见则贴更近的上/下边缘
    if let Some((p0, p1)) = x_seg {
        let (off, anc) = compute_axis_tick_placement(p0, p1, w_pt, h_pt, AxisKind::X);
        emit_ticks_on_segment(
            p0,
            p1,
            &h_inv,
            w_pt,
            h_pt,
            major_step,
            minor_step,
            AxisKind::X,
            off,
            anc,
            false,
            &mut placed_rects,
            out,
        );
    } else {
        let (p0, p1, off, anc) = pick_closer_horizontal_edge(l_x_ndc, &h_inv);
        emit_ticks_on_segment(
            p0,
            p1,
            &h_inv,
            w_pt,
            h_pt,
            major_step,
            minor_step,
            AxisKind::X,
            off,
            anc,
            false,
            &mut placed_rects,
            out,
        );
    }

    // 4. Y 坐标数字：轴可见则贴 Y 轴左侧；不可见则贴更近的左/右边缘
    if let Some((p0, p1)) = y_seg {
        let (off, anc) = compute_axis_tick_placement(p0, p1, w_pt, h_pt, AxisKind::Y);
        emit_ticks_on_segment(
            p0,
            p1,
            &h_inv,
            w_pt,
            h_pt,
            major_step,
            minor_step,
            AxisKind::Y,
            off,
            anc,
            skip_y_zero,
            &mut placed_rects,
            out,
        );
    } else {
        let (p0, p1, off, anc) = pick_closer_vertical_edge(l_y_ndc, &h_inv);
        emit_ticks_on_segment(
            p0,
            p1,
            &h_inv,
            w_pt,
            h_pt,
            major_step,
            minor_step,
            AxisKind::Y,
            off,
            anc,
            skip_y_zero,
            &mut placed_rects,
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrant_two_places_ticks_on_bottom_and_right_edges() {
        let mut cam = ProjectiveCamera::default();
        cam.center = [-20.0, 20.0];
        cam.half_height = 4.0;

        let mut labels = Vec::new();
        collect_canvas_labels(&cam, true, true, [800.0, 600.0], 1.0, &mut labels);

        assert!(!labels.is_empty());
        // 第二象限：所有 X 刻度均贴下边缘 (y_ndc = -1)，所有 Y 刻度均贴右边缘 (x_ndc = +1)
        let ticks: Vec<_> = labels
            .iter()
            .filter(|l| l.kind == LabelKind::AxisTick)
            .collect();
        assert!(!ticks.is_empty());
        for t in ticks {
            let on_bottom = (t.pos_ndc[1] + 1.0).abs() < 1e-4;
            let on_right = (t.pos_ndc[0] - 1.0).abs() < 1e-4;
            assert!(
                on_bottom || on_right,
                "unexpected edge pos {:?} for '{}'",
                t.pos_ndc,
                t.text
            );
            if on_bottom {
                let val: f64 = t.text.parse().unwrap();
                assert!(val < 0.0, "expected negative x in Q2, got {val}");
            }
            if on_right {
                let val: f64 = t.text.parse().unwrap();
                assert!(val > 0.0, "expected positive y in Q2, got {val}");
            }
        }
        // 轴在屏幕外时不绘制 X / Y 名称
        assert!(labels.iter().all(|l| l.kind != LabelKind::AxisName));
    }

    #[test]
    fn origin_view_draws_axes_names_and_single_zero() {
        let cam = ProjectiveCamera::default();
        let mut labels = Vec::new();
        collect_canvas_labels(&cam, true, true, [800.0, 600.0], 1.0, &mut labels);

        let zeros = labels.iter().filter(|l| l.text == "0").count();
        assert_eq!(zeros, 1);
        assert!(labels.iter().any(|l| l.kind == LabelKind::AxisName && l.text == "X"));
        assert!(labels.iter().any(|l| l.kind == LabelKind::AxisName && l.text == "Y"));
    }

    #[test]
    fn projective_horizon_in_view_does_not_explode_and_fades_out() {
        let mut cam = ProjectiveCamera::default();
        // 强射影畸变：把地平线拉进视口内
        cam.proj_matrix = Mat3F64 {
            m: [[1.0, 0.15, 0.0], [0.05, 1.0, 0.0], [0.35, 0.45, 1.0]],
        };
        let mut labels = Vec::new();
        collect_canvas_labels(&cam, true, true, [1200.0, 900.0], 1.0, &mut labels);

        assert!(labels.len() < 128);
        assert!(labels.iter().all(|l| l.alpha > 0.0 && l.alpha <= 1.0));
    }

    #[test]
    fn format_tick_clean() {
        assert_eq!(format_tick_value(0, 0.25, 0.05), "0");
        assert_eq!(format_tick_value(1, 0.25, 0.05), "0.25");
        assert_eq!(format_tick_value(2, 0.25, 0.05), "0.5");
        assert_eq!(format_tick_value(-4, 0.25, 0.05), "-1");
        assert_eq!(format_tick_value(3, 2.5, 0.5), "7.5");
        assert_eq!(format_tick_value(-2, 5e5, 1e5), "-1e6");
    }
}