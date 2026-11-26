use crate::canvas::camera::{
    CanvasUniforms, ConicInstance, LineVertex, PointInstance, PreparedDrawData,
};
use crate::canvas::styles::CanvasColors;
use eframe::egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use egui::PaintCallbackInfo;
use wgpu::util::DeviceExt;

struct DynamicGpuBuffer {
    buffer: wgpu::Buffer,
    capacity_bytes: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl DynamicGpuBuffer {
    fn new(
        device: &wgpu::Device,
        label: &'static str,
        usage: wgpu::BufferUsages,
        init_bytes: u64,
    ) -> Self {
        let cap = init_bytes.max(64);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: cap,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity_bytes: cap,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            label,
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let needed = bytes.len() as u64;
        if needed > self.capacity_bytes {
            let new_cap = needed.next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: new_cap,
                usage: self.usage,
                mapped_at_creation: false,
            });
            self.capacity_bytes = new_cap;
        }
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

pub struct CanvasGpuResources {
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    grid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    conic_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,

    line_vbo: DynamicGpuBuffer,
    conic_ibo: DynamicGpuBuffer,
    point_ibo: DynamicGpuBuffer,

    show_grid: bool,
    show_horizon: bool,
    line_vertex_count: u32,
    axis_vertex_count: u32,
    conic_instance_count: u32,
    point_instance_count: u32,
}

impl CanvasGpuResources {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gmk3_canvas_shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        let init_uniforms = CanvasUniforms {
            viewport_px: [800.0, 600.0],
            grid_mode: 2,
            minor_line_px: 1.0,
            grid_inv_col0: [1.0, 0.0, 0.0, 0.0],
            grid_inv_col1: [0.0, 1.0, 0.0, 0.0],
            grid_inv_col2: [0.0, 0.0, 1.0, 0.0],
            grid_minor_color: CanvasColors::DARK.grid_minor,
            grid_major_color: CanvasColors::DARK.grid_major,
            major_line_px: 1.15,
            major_every: 5.0,
            show_horizon: 0,
            horizon_line_px: 0.8,
            horizon_glow_px: 16.0,
            _pad0: [0.0, 0.0, 0.0],
            horizon_abc: [0.0, 0.0, 1.0, 0.0],
            horizon_color: CanvasColors::DARK.horizon,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gmk3_canvas_uniforms"),
            contents: bytemuck::bytes_of(&init_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gmk3_canvas_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gmk3_canvas_bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gmk3_canvas_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let color_target = Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        });

        let grid_pipeline = create_pipeline(
            device,
            "gmk3_grid_pipeline",
            &pipeline_layout,
            &shader,
            "vs_grid",
            "fs_grid",
            &[],
            color_target.clone(),
        );

        let line_pipeline = create_pipeline(
            device,
            "gmk3_line_pipeline",
            &pipeline_layout,
            &shader,
            "vs_line",
            "fs_line",
            &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 12,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 3,
                    },
                ],
            })],
            color_target.clone(),
        );

        let conic_pipeline = create_pipeline(
            device,
            "gmk3_conic_pipeline",
            &pipeline_layout,
            &shader,
            "vs_conic",
            "fs_conic",
            &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ConicInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 24,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 28,
                        shader_location: 3,
                    },
                ],
            })],
            color_target.clone(),
        );

        let point_pipeline = create_pipeline(
            device,
            "gmk3_point_pipeline",
            &pipeline_layout,
            &shader,
            "vs_point",
            "fs_point",
            &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PointInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 12,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: 28,
                        shader_location: 3,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: 32,
                        shader_location: 4,
                    },
                ],
            })],
            color_target,
        );

        Self {
            uniform_buffer,
            uniform_bind_group,
            grid_pipeline,
            line_pipeline,
            conic_pipeline,
            point_pipeline,
            line_vbo: DynamicGpuBuffer::new(
                device,
                "gmk3_line_vbo",
                wgpu::BufferUsages::VERTEX,
                4096,
            ),
            conic_ibo: DynamicGpuBuffer::new(
                device,
                "gmk3_conic_ibo",
                wgpu::BufferUsages::VERTEX,
                1024,
            ),
            point_ibo: DynamicGpuBuffer::new(
                device,
                "gmk3_point_ibo",
                wgpu::BufferUsages::VERTEX,
                1024,
            ),
            show_grid: true,
            show_horizon: false,
            line_vertex_count: 0,
            axis_vertex_count: 0,
            conic_instance_count: 0,
            point_instance_count: 0,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &wgpu::Device,
    label: &'static str,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    vs_entry: &'static str,
    fs_entry: &'static str,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    color_target: Option<wgpu::ColorTargetState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs_entry),
            buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[color_target],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub struct CanvasCallback {
    pub data: PreparedDrawData,
}

impl CallbackTrait for CanvasCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(res) = callback_resources.get_mut::<CanvasGpuResources>() else {
            return Vec::new();
        };

        queue.write_buffer(
            &res.uniform_buffer,
            0,
            bytemuck::bytes_of(&self.data.uniforms),
        );

        res.show_grid = self.data.uniforms.grid_mode != 0;
        res.show_horizon = self.data.uniforms.show_horizon != 0;
        res.line_vertex_count = self.data.line_vertices.len() as u32;
        res.axis_vertex_count = self.data.axis_vertex_count as u32;
        res.line_vbo.write(
            device,
            queue,
            bytemuck::cast_slice(&self.data.line_vertices),
        );

        res.conic_instance_count = self.data.conic_instances.len() as u32;
        res.conic_ibo.write(
            device,
            queue,
            bytemuck::cast_slice(&self.data.conic_instances),
        );

        res.point_instance_count = self.data.point_instances.len() as u32;
        res.point_ibo.write(
            device,
            queue,
            bytemuck::cast_slice(&self.data.point_instances),
        );

        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let Some(res) = callback_resources.get::<CanvasGpuResources>() else {
            return;
        };

        // 0. 底图层：无限射影网格 + 灭线光晕（O(1) 全屏着色器）
        if res.show_grid || res.show_horizon {
            render_pass.set_pipeline(&res.grid_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // 1. 世界坐标轴
        if res.axis_vertex_count > 0 {
            render_pass.set_pipeline(&res.line_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.line_vbo.buffer.slice(..));
            render_pass.draw(0..res.axis_vertex_count, 0..1);
        }

        // 2. 圆锥曲线
        if res.conic_instance_count > 0 {
            render_pass.set_pipeline(&res.conic_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.conic_ibo.buffer.slice(..));
            render_pass.draw(0..6, 0..res.conic_instance_count);
        }

        // 3. 用户直线
        if res.line_vertex_count > res.axis_vertex_count {
            render_pass.set_pipeline(&res.line_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.line_vbo.buffer.slice(..));
            render_pass.draw(res.axis_vertex_count..res.line_vertex_count, 0..1);
        }

        // 4. 点与骈点
        if res.point_instance_count > 0 {
            render_pass.set_pipeline(&res.point_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.point_ibo.buffer.slice(..));
            render_pass.draw(0..6, 0..res.point_instance_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// 用 naga 在编译期校验着色器（设备上才会暴露的校验错误提前在这里报出来），
    /// 并把 WGSL 的 `CanvasUniforms` 结构大小与 Rust 侧逐一比对，
    /// 防止 vec3 对齐（16）/ padding 等跨语言布局错位。
    #[test]
    fn shaders_wgsl_validates() {
        let source = include_str!("shaders.wgsl");
        let module =
            wgpu::naga::front::wgsl::parse_str(source).expect("WGSL 解析失败");
        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .expect("WGSL 校验失败（顶点/片元 I/O 不匹配等）");

        // 跨语言布局一致性：WGSL 结构大小必须等于 Rust 结构大小
        let mut layouter = wgpu::naga::proc::Layouter::default();
        layouter
            .update(wgpu::naga::proc::GlobalCtx {
                types: &module.types,
                constants: &module.constants,
                overrides: &module.overrides,
                global_expressions: &module.global_expressions,
            })
            .expect("WGSL 布局计算失败");
        let (handle, ty) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("CanvasUniforms"))
            .expect("应存在 CanvasUniforms 结构");
        assert!(matches!(ty.inner, wgpu::naga::TypeInner::Struct { .. }));
        assert_eq!(
            layouter[handle].size as usize,
            size_of::<CanvasUniforms>(),
            "WGSL 与 Rust 的 CanvasUniforms 结构大小不一致（检查 vec3 对齐 / padding）"
        );
    }
}
