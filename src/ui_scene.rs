extern crate lyon;
use std::ops::Deref;

use cgmath::{prelude::*, Matrix4};
use cgmath::{Quaternion, Rotation3, SquareMatrix, Transform};
use lyon::geom::Box2D;
use lyon::math::point;
use lyon::tessellation::*;
use wgpu::util::DeviceExt;
use winit::event::{ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent};

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

pub struct Player {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instances: Vec<InstanceRaw>,
    instance_buffer: wgpu::Buffer,
}

impl Player {
    pub fn new(ctx: &wgpu::Device) -> Self {
        let m_box = Box2D::new(point(0.0, 0.0), point(50.0, 50.0));
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
                        color: [0.0, 0.0, 0.5],
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

        let instance = Instance {
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(45.0)),
        };
        // Instance index buffer
        let instance_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[instance.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertices: geometry.vertices.clone(),
            indices: geometry.indices.clone(),
            vertex_buffer,
            instances: vec![instance.to_raw()],
            index_buffer,
            instance_buffer,
        }
    }
}

pub struct OrtographicCamera {
    projection: cgmath::Matrix4<f32>,
    translation: cgmath::Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: f32,
}

impl OrtographicCamera {
    fn new(
        projection: cgmath::Matrix4<f32>,
        translation: cgmath::Vector3<f32>,
        rotation: Quaternion<f32>,
        scale: f32,
    ) -> Self {
        Self {
            projection,
            translation,
            rotation,
            scale,
        }
    }

    fn get_view_proj(&self) -> Matrix4<f32> {
        let transform = cgmath::Matrix4::from_translation(self.translation)
            * cgmath::Matrix4::from(self.rotation)
            * cgmath::Matrix4::from_scale(self.scale);
        let view = Matrix4::invert(&transform).unwrap();
        self.projection * view
    }

    fn set_projection(&mut self, proj: cgmath::Matrix4<f32>) {
        self.projection = proj;
    }
    fn set_position(&mut self, pos: cgmath::Vector3<f32>) {
        self.translation = pos;
    }

    fn set_rotation(&mut self, rot: cgmath::Quaternion<f32>) {
        self.rotation = rot;
    }

    fn add_scale(&mut self, scale: f32) {
        if self.scale + scale <= 0.0 {
            self.scale = 0.1;
        } else {
            self.scale += scale;
        }
    }
}



pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}




pub struct UIScene {
    pub elements: Vec<Player>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub camera: OrtographicCamera,
    pub camera_buffer: wgpu::Buffer,
}

impl UIScene {
    pub async fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let aspect = (config.width as f32 / config.height as f32);
        let half_height = config.height as f32 / 2.0; // also called ortho size
        let half_width = half_height * aspect;

        let elements = vec![Player::new(device)];



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
                buffers: &[Vertex::desc(), InstanceRaw::desc()], // 2.
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

        let camera = OrtographicCamera {
            projection: cgmath::ortho(
                -half_width,
                half_width,
                -half_height,
                half_height,
                -1.0,
                1.0,
            ),
            translation: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0),
            ),
            scale: 1.0,
        };

        let view_projection_matrix: [[f32; 4]; 4] = camera.get_view_proj().into();
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
            camera_buffer: ortho_proj_buffer,
            camera,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        let aspect = config.width as f32 / config.height as f32;
        let half_height = config.height as f32 / 2.0;
        let half_width = half_height as f32 * aspect;
        let new_projection = cgmath::ortho(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -1.0,
            1.0,
        );
        self.camera.set_projection(new_projection);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                ..
            } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.camera.add_scale(y.to_owned());
                }
                _ => {}
            },
            _ => (),
        }

        false
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        let new_view_proj: [[f32; 4]; 4] = self.camera.get_view_proj().into();
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&new_view_proj));
    }

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
            render_pass.set_vertex_buffer(1, element.instance_buffer.slice(..));
            render_pass.set_index_buffer(element.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..element.indices.len() as u32, 0, 0..1);
        }
    }
}
