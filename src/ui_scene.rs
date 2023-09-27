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

fn screen_to_cartesian(
    screen_x: f32,
    screen_y: f32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    screen_width: f32,
    screen_height: f32,
) -> (f32, f32) {
    let cartesian_x = (screen_x / screen_width) * (x_max - x_min) + x_min;
    let cartesian_y = ((screen_height - screen_y) / screen_height) * (y_max - y_min) + y_min;

    (cartesian_x, cartesian_y)
}

pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::ortho(0.0, 0.0, 0.0, 0.0, 0.1, 100.0);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }

    fn from_camera(camera: &Camera) -> Self {
        Self {
            view_proj: camera.build_view_projection_matrix().into(),
        }
    }
}

pub struct UIScene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl UIScene {
    pub async fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let m_box = Box2D::new(point(0.0, 0.0), point(-50.0, -50.0));

        let aspect = (config.width / config.height) as f32;
        let half_height = config.height as f32 / 2.0; // also called ortho size
        let half_width = half_height * aspect;

        let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();

        {
            // Compute the tessellation.
            tessellator
                .tessellate_rectangle(
                    &m_box,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                        position: [vertex.position().x, vertex.position().y, -0.0],
                        color: [0.5, 0.0, 0.0],
                    }),
                )
                .unwrap();
        }
        let vertices = geometry.vertices.to_vec();
        let indices = geometry.indices.to_vec();

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

        let projection = cgmath::ortho(-half_width, half_width,-half_height, half_height, -1.0, 1.0);
        let rotation = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0));
        let transform = cgmath::Matrix4::from_translation(cgmath::Vector3::new(0.0, 0.0, 0.0)) * cgmath::Matrix4::from(rotation);
        let view_matrix = cgmath::Matrix4::invert(&transform).unwrap();
        let view_projection_matrix: [[f32; 4]; 4] = (projection * view_matrix).into();

        let ortho_proj_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uOrthoProj"),
            contents: bytemuck::cast_slice(&view_projection_matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            contents: bytemuck::cast_slice(geometry.vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Index buffer"),
            contents: bytemuck::cast_slice(geometry.indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
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
            vertices,
            indices,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {}

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> (Vec<Vertex>, Vec<u16>) {
        let screen_start_pos = cgmath::Vector2::new(x, y);
        let screen_end_pos = screen_start_pos + cgmath::Vector2::new(w, h);

        // Start point
        let xy1 = cgmath::Vector2::from(screen_to_cartesian(
            screen_start_pos.x,
            screen_start_pos.y,
            -1.0,
            1.0,
            -1.0,
            1.0,
            800.0,
            600.0,
        ));

        // End point (in cartesian)
        let xy2 = cgmath::Vector2::from(screen_to_cartesian(
            screen_end_pos.x,
            screen_end_pos.y,
            -1.0,
            1.0,
            -1.0,
            1.0,
            800.0,
            600.0,
        ));

        let mut vertices = vec![];
        vertices.push(Vertex {
            position: [xy1.x, xy1.y, 0.0],
            color: [0.5, 0.0, 0.0],
        }); // -0.5, -0.5
        vertices.push(Vertex {
            position: [xy2.x, xy1.y, 0.0],
            color: [0.5, 0.0, 0.0],
        }); // 0.5, -0.5
        vertices.push(Vertex {
            position: [xy2.x, xy2.y, 0.0],
            color: [0.5, 0.0, 0.0],
        }); // 0.5, 0.5
        vertices.push(Vertex {
            position: [xy1.x, xy2.y, 0.0],
            color: [0.5, 0.0, 0.0],
        }); // -0.5, 0.5

        let indices = vec![2, 1, 0, 2, 0, 3];
        return (vertices, indices);
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
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}
