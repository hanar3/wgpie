extern crate lyon;
use cgmath::{Rotation3, SquareMatrix};
use lyon::geom::Box2D;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::*;
use wgpu::util::DeviceExt;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub trait Element {
    fn render(&mut self, render_pass: &mut wgpu::RenderPass);
}

pub struct Rect {
    pub pos: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl Rect {
    pub fn new(ctx: &wgpu::Device, pos: cgmath::Vector2<f32>, size: cgmath::Vector2<f32>) -> Self {
        let end_pos = pos + size;
        let m_box = Box2D::new(point(pos.x, pos.y), point(end_pos.x, end_pos.y));
        let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        {
            // Compute the tessellation.
            tessellator
                .tessellate_rectangle(
                    &m_box,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                        position: [vertex.position().x, vertex.position().y, 0.0],
                        color: [0.5, 0.0, 0.0],
                    }),
                )
                .unwrap();
        }
        // Vertex buffer
        let vertex_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(geometry.vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Index buffer
        let index_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(geometry.indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pos,
            size,
            vertices: geometry.vertices.clone(),
            indices: geometry.indices.clone(),
            vertex_buffer,
            index_buffer,
        }
    }


}

pub struct UIScene {
    pub elements: Vec<Rect>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
}

impl UIScene {
    pub async fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let aspect = (config.width / config.height) as f32;
        let half_height = config.height as f32 / 2.0; // also called ortho size
        let half_width = half_height * aspect;
        let elements = vec![
            Rect::new(
                device,
                cgmath::Vector2::new(-200.0, -200.0),
                cgmath::Vector2::new(200.0, 200.0),
            ),
            Rect::new(
                device,
                cgmath::Vector2::new(20.0, 20.0),
                cgmath::Vector2::new(200.0, 200.0),
            ),
            Rect::new(
                device,
                cgmath::Vector2::new(40.0, 40.0),
                cgmath::Vector2::new(200.0, 200.0),
            ),
        ];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui_bind_group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // surface size buffer
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // orthographic matrix
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Render pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",     // 1.
                buffers: &[Vertex::desc()], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        });

        // Creating uniforms
        let screen_size_uniform = &[config.width as f32, config.height as f32];
        let screen_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screensize_uniform"),
            contents: bytemuck::cast_slice(screen_size_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let projection = cgmath::ortho(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -1.0,
            1.0,
        );
        let rotation =
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0));
        let transform = cgmath::Matrix4::from_translation(cgmath::Vector3::new(0.0, 0.0, 0.0))
            * cgmath::Matrix4::from(rotation);
        let view_matrix = cgmath::Matrix4::invert(&transform).unwrap();
        let view_projection_matrix: [[f32; 4]; 4] = (projection * view_matrix).into();

        let ortho_proj_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uOrthoProj"),
            contents: bytemuck::cast_slice(&view_projection_matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: screen_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ortho_proj_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            render_pipeline,
            bind_group,
            elements,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {}

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {}

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        for element in self.elements.as_slice() {
            render_pass.set_vertex_buffer(0, element.vertex_buffer.slice(..));
            render_pass.set_index_buffer(element.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..element.indices.len() as u32, 0, 0..1);
        }
    }
}
