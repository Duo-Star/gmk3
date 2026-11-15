use crate::canvas::camera::{
    CanvasUniforms, ConicInstance, LineVertex, PointInstance, PreparedDrawData,
};
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};

use egui::PaintCallbackInfo;

use wgpu::util::DeviceExt;

struct DynamicGpuBuffer {
    buffer: wgpu::Buffer,
    capacity_bytes: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl DynamicGpuBuffer {
    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, init_bytes: u64) -> Self {
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

    line_pipeline: wgpu::RenderPipeline,
    conic_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,

    line_vbo: DynamicGpuBuffer,
    conic_ibo: DynamicGpuBuffer,
    point_ibo: DynamicGpuBuffer,

    line_vertex_count: u32,
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
            _pad: [0.0, 0.0],
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
                ],
            })],
            color_target,
        );

        Self {
            uniform_buffer,
            uniform_bind_group,
            line_pipeline,
            conic_pipeline,
            point_pipeline,
            line_vbo: DynamicGpuBuffer::new(device, "gmk3_line_vbo", wgpu::BufferUsages::VERTEX, 4096),
            conic_ibo: DynamicGpuBuffer::new(device, "gmk3_conic_ibo", wgpu::BufferUsages::VERTEX, 1024),
            point_ibo: DynamicGpuBuffer::new(device, "gmk3_point_ibo", wgpu::BufferUsages::VERTEX, 1024),
            line_vertex_count: 0,
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

        queue.write_buffer(&res.uniform_buffer, 0, bytemuck::bytes_of(&self.data.uniforms));

        res.line_vertex_count = self.data.line_vertices.len() as u32;
        res.line_vbo
            .write(device, queue, bytemuck::cast_slice(&self.data.line_vertices));

        res.conic_instance_count = self.data.conic_instances.len() as u32;
        res.conic_ibo
            .write(device, queue, bytemuck::cast_slice(&self.data.conic_instances));

        res.point_instance_count = self.data.point_instances.len() as u32;
        res.point_ibo
            .write(device, queue, bytemuck::cast_slice(&self.data.point_instances));

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

        if res.conic_instance_count > 0 {
            render_pass.set_pipeline(&res.conic_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.conic_ibo.buffer.slice(..));
            render_pass.draw(0..6, 0..res.conic_instance_count);
        }

        if res.line_vertex_count > 0 {
            render_pass.set_pipeline(&res.line_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.line_vbo.buffer.slice(..));
            render_pass.draw(0..res.line_vertex_count, 0..1);
        }

        if res.point_instance_count > 0 {
            render_pass.set_pipeline(&res.point_pipeline);
            render_pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.point_ibo.buffer.slice(..));
            render_pass.draw(0..6, 0..res.point_instance_count);
        }
    }
}