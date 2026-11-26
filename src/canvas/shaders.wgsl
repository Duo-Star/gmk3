struct CanvasUniforms {
    viewport_px: vec2<f32>,
    grid_mode: u32,
    minor_line_px: f32,
    grid_inv_col0: vec4<f32>,
    grid_inv_col1: vec4<f32>,
    grid_inv_col2: vec4<f32>,
    grid_minor_color: vec4<f32>,
    grid_major_color: vec4<f32>,
    major_line_px: f32,
    major_every: f32,
    show_horizon: u32,
    horizon_line_px: f32,
    horizon_glow_px: f32,
    // 三个 f32 标量把 horizon_abc 顶到 16 对齐（勿用 vec3：WGSL vec3 对齐 16，会撑大结构）
    _pad0a: f32,
    _pad0b: f32,
    _pad0c: f32,
    horizon_abc: vec4<f32>,
    horizon_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: CanvasUniforms;

// ------------------------------------------------------------------
// 0. 无限射影网格（全屏 Quad + 解析导数 + 盒式滤波）
// ------------------------------------------------------------------
struct GridVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_grid(@builtin(vertex_index) vid: u32) -> GridVertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    let ndc = corners[vid];
    var out: GridVertexOut;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.ndc = ndc;
    return out;
}

/// 精确 1D 盒式滤波覆盖率 + Nyquist 渐近均质混合：
/// - 在正常视口下输出恒定 `line_px` 宽度的无闪烁抗锯齿线；
/// - 在接近无穷远直线 `w_z -> 0` 时，平滑收敛到周期平均覆盖率 `max_frac`，无摩尔纹与接缝。
fn eval_grid_coverage(uv: vec2<f32>, uv_deriv: vec2<f32>, line_px: f32, max_frac: f32) -> f32 {
    // 每像素在格子坐标下的覆盖尺度（乘以 1.15 作为轻微抗锯齿羽化）
    let fp = max(uv_deriv * 1.15, vec2<f32>(1e-6));

    // 1. 真正绘制清晰线条时，允许线宽维持完整亮度直到接近 2px 间距（最高 50% 占空比）
    let line_w = min(line_px * uv_deriv, vec2<f32>(0.3));
    // 2. 进入亚像素过密区后的渐近平均亮度上限（防止地平线泛白过曝）
    let avg_w = min(line_px * uv_deriv, vec2<f32>(max_frac));

    let d_uv = abs(fract(uv - 0.5) - 0.5);
    let hw = 0.5 * line_w;
    let hp = 0.5 * fp;

    let overlap = max(min(d_uv + hp, hw) - max(d_uv - hp, -hw), vec2<f32>(0.0)) / fp;
    // 在间距 2px (uv_deriv = 0.5) 到 1px (uv_deriv = 1.0) 之间平滑过渡到均质雾化
    let nyquist_blend = clamp(uv_deriv * 2.0 - 1.0, vec2<f32>(0.0), vec2<f32>(1.0));
    let cov = mix(overlap, avg_w, nyquist_blend);
    return clamp(cov.x + cov.y - cov.x * cov.y, 0.0, 1.0);
}

@fragment
fn fs_grid(in: GridVertexOut) -> @location(0) vec4<f32> {
    // ---- 灭线光晕（最底层）：无穷远直线处的细核心线 + 静态光晕 ----
    var h_a: f32 = 0.0;
    var h_rgb: vec3<f32> = vec3<f32>(0.0);
    if (u.show_horizon == 1u) {
        let ab2 = dot(u.horizon_abc.xy, u.horizon_abc.xy);
        if (ab2 > 1e-12) {
            let d_ndc = dot(u.horizon_abc.xy, in.ndc) + u.horizon_abc.z;
            let d_px = d_ndc * (0.5 * max(u.viewport_px.y, 1.0));
            let g_w = max(u.horizon_glow_px, 1.0);
            let glow = exp(-(d_px * d_px) / (g_w * g_w));
            let c_w = max(u.horizon_line_px, 0.5);
            let core = exp(-(d_px * d_px) / (c_w * c_w));
            h_a = u.horizon_color.a * min(0.22 * glow + 0.85 * core, 1.0);
            h_rgb = u.horizon_color.rgb;
        }
    }

    // ---- 网格（画在灭线之上）----
    var g_a: f32 = 0.0;
    var g_rgb: vec3<f32> = vec3<f32>(0.0);
    if (u.grid_mode > 0u) {
        let c0 = u.grid_inv_col0.xyz;
        let c1 = u.grid_inv_col1.xyz;
        let c2 = u.grid_inv_col2.xyz;

        let w = c0 * in.ndc.x + c1 * in.ndc.y + c2;
        let ndc_per_px = 2.0 / max(u.viewport_px, vec2<f32>(1.0));
        let dw_dx = c0 * ndc_per_px.x;
        let dw_dy = c1 * ndc_per_px.y;

        let wz = w.z;
        let wz_safe = select(wz, select(1e-7, -1e-7, wz < 0.0), abs(wz) < 1e-7);
        let uv = w.xy / wz_safe;

        // 商法则解析导数：跨越 w_z = 0（地平线）时无 2x2 quad 块状伪影且永不归零
        let inv_wz2 = 1.0 / max(wz * wz, 1e-12);
        let du_dx = (wz * dw_dx.x - w.x * dw_dx.z) * inv_wz2;
        let du_dy = (wz * dw_dy.x - w.x * dw_dy.z) * inv_wz2;
        let dv_dx = (wz * dw_dx.y - w.y * dw_dx.z) * inv_wz2;
        let dv_dy = (wz * dw_dy.y - w.y * dw_dy.z) * inv_wz2;

        var uv_deriv = vec2<f32>(
            length(vec2<f32>(du_dx, du_dy)),
            length(vec2<f32>(dv_dx, dv_dy))
        );

        // 若像素距离地平线 w_z = 0 不足 1 像素（含灭点 w_x = w_z = 0），强制进入过密均质区
        let grad_wz_px = length(vec2<f32>(dw_dx.z, dw_dy.z));
        if (grad_wz_px > 1e-8 && abs(wz) < grad_wz_px) {
            uv_deriv = max(uv_deriv, vec2<f32>(u.major_every));
        }

        var minor_a: f32 = 0.0;
        if (u.grid_mode == 2u) {
            let minor_cov = eval_grid_coverage(uv, uv_deriv, u.minor_line_px, 0.085);
            minor_a = u.grid_minor_color.a * minor_cov;
        }
        let major_uv = uv / u.major_every;
        let major_deriv = uv_deriv / u.major_every;
        let major_cov = eval_grid_coverage(major_uv, major_deriv, u.major_line_px, 0.05);
        let major_a = u.grid_major_color.a * major_cov;

        g_a = major_a + minor_a * (1.0 - major_a);
        g_rgb = (u.grid_major_color.rgb * major_a
            + u.grid_minor_color.rgb * minor_a * (1.0 - major_a))
            / max(g_a, 1e-6);
    }

    // 合成：灭线光晕在下，网格在上
    let a = g_a + h_a * (1.0 - g_a);
    if (a <= 0.001) {
        discard;
    }
    let rgb = (g_rgb * g_a + h_rgb * h_a * (1.0 - g_a)) / a;
    return vec4<f32>(rgb, a);
}

// ------------------------------------------------------------------
// 1. 直线（NDC 截断 + 法线挤出矩形）
// ------------------------------------------------------------------
struct LineVertexIn {
    @location(0) pos_ndc: vec2<f32>,
    @location(1) edge_dist_px: f32,
    @location(2) half_width_px: f32,
    @location(3) color: vec4<f32>,
};

struct LineVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) edge_dist_px: f32,
    @location(1) half_width_px: f32,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_line(in: LineVertexIn) -> LineVertexOut {
    var out: LineVertexOut;
    out.clip_pos = vec4<f32>(in.pos_ndc, 0.0, 1.0);
    out.edge_dist_px = in.edge_dist_px;
    out.half_width_px = in.half_width_px;
    out.color = in.color;
    return out;
}

@fragment
fn fs_line(in: LineVertexOut) -> @location(0) vec4<f32> {
    let d = abs(in.edge_dist_px);
    let hw = in.half_width_px;
    let alpha = 1.0 - smoothstep(max(hw - 0.75, 0.0), hw + 0.75, d);
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}

// ------------------------------------------------------------------
// 2. 圆锥曲线（Instanced 全屏矩形 + 隐式二次型解析梯度距离场）
// ------------------------------------------------------------------
struct ConicInstanceIn {
    @location(0) quad_abc: vec3<f32>,
    @location(1) quad_def: vec3<f32>,
    @location(2) half_width_px: f32,
    @location(3) color: vec4<f32>,
};

struct ConicVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
    @location(1) quad_abc: vec3<f32>,
    @location(2) quad_def: vec3<f32>,
    @location(3) half_width_px: f32,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_conic(@builtin(vertex_index) vid: u32, inst: ConicInstanceIn) -> ConicVertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    let ndc = corners[vid];

    var out: ConicVertexOut;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.ndc = ndc;
    out.quad_abc = inst.quad_abc;
    out.quad_def = inst.quad_def;
    out.half_width_px = inst.half_width_px;
    out.color = inst.color;
    return out;
}

@fragment
fn fs_conic(in: ConicVertexOut) -> @location(0) vec4<f32> {
    let x = in.ndc.x;
    let y = in.ndc.y;
    let a = in.quad_abc.x;
    let b = in.quad_abc.y;
    let c = in.quad_abc.z;
    let d = in.quad_def.x;
    let e = in.quad_def.y;
    let f = in.quad_def.z;

    let val = a * x * x + 2.0 * b * x * y + c * y * y + 2.0 * d * x + 2.0 * e * y + f;
    let dv_dx = 2.0 * (a * x + b * y + d);
    let dv_dy = 2.0 * (b * x + c * y + e);

    let grad_px = vec2<f32>(
        dv_dx * (2.0 / max(u.viewport_px.x, 1.0)),
        dv_dy * (2.0 / max(u.viewport_px.y, 1.0))
    );
    let dist_px = abs(val) / max(length(grad_px), 1e-6);

    let hw = in.half_width_px;
    let alpha = 1.0 - smoothstep(max(hw - 0.75, 0.0), hw + 0.75, dist_px);
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}

// ------------------------------------------------------------------
// 3. 点（Instanced 像素矩形 + 形状 SDF + 选中光晕）
// ------------------------------------------------------------------
struct PointInstanceIn {
    @location(0) center_ndc: vec2<f32>,
    @location(1) radius_px: f32,
    @location(2) color: vec4<f32>,
    @location(3) shape: u32,
    @location(4) selected: u32,
};

struct PointVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_px: vec2<f32>,
    @location(1) radius_px: f32,
    @location(2) color: vec4<f32>,
    @location(3) @interpolate(flat) shape: u32,
    @location(4) @interpolate(flat) selected: u32,
};

@vertex
fn vs_point(@builtin(vertex_index) vid: u32, inst: PointInstanceIn) -> PointVertexOut {
    var offsets = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    // 选中时矩形需覆盖 1.6 倍半径的光晕
    let halo_scale = select(1.0, 1.6, inst.selected == 1u);
    let pad = inst.radius_px * halo_scale + 1.5;
    let local_px = offsets[vid] * pad;
    let offset_ndc = vec2<f32>(
        local_px.x * (2.0 / max(u.viewport_px.x, 1.0)),
        local_px.y * (2.0 / max(u.viewport_px.y, 1.0))
    );

    var out: PointVertexOut;
    out.clip_pos = vec4<f32>(inst.center_ndc + offset_ndc, 0.0, 1.0);
    out.local_px = local_px;
    out.radius_px = inst.radius_px;
    out.color = inst.color;
    out.shape = inst.shape;
    out.selected = inst.selected;
    return out;
}

@fragment
fn fs_point(in: PointVertexOut) -> @location(0) vec4<f32> {
    let r = in.radius_px;
    let dist = length(in.local_px);

    // ---- 基础形状覆盖率（0 solid / 1 hollow / 2 cross / 3 dotted）----
    var a: f32;
    if (in.shape == 1u) {
        // 空心圆环：|dist - r| < 环宽
        let hw = max(1.0, 0.22 * r);
        let d = abs(dist - r);
        a = 1.0 - smoothstep(max(hw - 0.75, 0.0), hw + 0.75, d);
    } else if (in.shape == 2u) {
        // 十字：横竖两盒 SDF 的并集
        let hw = max(0.8, 0.25 * r);
        let dx = abs(in.local_px.x);
        let dy = abs(in.local_px.y);
        let d = min(max(dy - hw, dx - r), max(dx - hw, dy - r));
        a = 1.0 - smoothstep(-0.75, 0.75, d);
    } else {
        // solid / dotted：实心圆（dotted 缩小半径）
        let rr = select(r, 0.55 * r, in.shape == 3u);
        a = 1.0 - smoothstep(max(rr - 0.75, 0.0), rr + 0.75, dist);
    }

    // ---- 选中：同色半透明光晕 + 光晕边缘细环（在点之下合成）----
    var a_sel: f32 = 0.0;
    if (in.selected == 1u) {
        let hr = r * 1.6;
        let halo = 1.0 - smoothstep(max(hr - 0.75, 0.0), hr + 0.75, dist);
        let a_halo = halo * 0.25;
        let ring_d = abs(dist - hr);
        let a_ring = 1.0 - smoothstep(max(0.6 - 0.5, 0.0), 0.6 + 0.5, ring_d);
        a_sel = a_halo + a_ring * (1.0 - a_halo);
    }

    let a_total = a + a_sel * (1.0 - a);
    if (a_total <= 0.001) {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * a_total);
}
