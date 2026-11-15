struct CanvasUniforms {
    viewport_px: vec2<f32>,
    line_half_width_px: f32,
    point_radius_px: f32,
};

@group(0) @binding(0)
var<uniform> u: CanvasUniforms;

// ------------------------------------------------------------------
// 1. 直线（CPU NDC 截断 + 法线挤出矩形）
// ------------------------------------------------------------------
struct LineVertexIn {
    @location(0) pos_ndc: vec2<f32>,
    @location(1) edge_dist_px: f32,
};

struct LineVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) edge_dist_px: f32,
};

@vertex
fn vs_line(in: LineVertexIn) -> LineVertexOut {
    var out: LineVertexOut;
    out.clip_pos = vec4<f32>(in.pos_ndc, 0.0, 1.0);
    out.edge_dist_px = in.edge_dist_px;
    return out;
}

@fragment
fn fs_line(in: LineVertexOut) -> @location(0) vec4<f32> {
    let d = abs(in.edge_dist_px);
    let hw = u.line_half_width_px;
    let alpha = 1.0 - smoothstep(max(hw - 0.75, 0.0), hw + 0.75, d);
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, alpha);
}

// ------------------------------------------------------------------
// 2. 圆锥曲线（Instanced 全屏矩形 + 隐式二次型解析梯度距离场）
// ------------------------------------------------------------------
struct ConicInstanceIn {
    @location(0) quad_abc: vec3<f32>,
    @location(1) quad_def: vec3<f32>,
};

struct ConicVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
    @location(1) quad_abc: vec3<f32>,
    @location(2) quad_def: vec3<f32>,
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

    let hw = u.line_half_width_px;
    let alpha = 1.0 - smoothstep(max(hw - 0.75, 0.0), hw + 0.75, dist_px);
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, alpha);
}

// ------------------------------------------------------------------
// 3. 点（Instanced 像素矩形 + 圆形 SDF）
// ------------------------------------------------------------------
struct PointInstanceIn {
    @location(0) center_ndc: vec2<f32>,
};

struct PointVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_px: vec2<f32>,
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
    let pad = u.point_radius_px + 1.5;
    let local_px = offsets[vid] * pad;
    let offset_ndc = vec2<f32>(
        local_px.x * (2.0 / max(u.viewport_px.x, 1.0)),
        local_px.y * (2.0 / max(u.viewport_px.y, 1.0))
    );

    var out: PointVertexOut;
    out.clip_pos = vec4<f32>(inst.center_ndc + offset_ndc, 0.0, 1.0);
    out.local_px = local_px;
    return out;
}

@fragment
fn fs_point(in: PointVertexOut) -> @location(0) vec4<f32> {
    let dist = length(in.local_px);
    let r = u.point_radius_px;
    let alpha = 1.0 - smoothstep(max(r - 0.75, 0.0), r + 0.75, dist);
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, alpha);
}