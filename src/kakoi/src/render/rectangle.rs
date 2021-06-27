use crate::{sampling_config::SamplingConfig, square_cuboid::SquareCuboid};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new(view_projection_matrix: cgmath::Matrix4<f32>) -> Self {
        Self {
            view_proj: view_projection_matrix.into(),
        }
    }
}

pub const THICKNESS: f32 = 0.02;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float3,
            }],
        }
    }

    fn make_rectangle(square_cuboid: &SquareCuboid) -> Vec<Vertex> {
        let (width, height) = square_cuboid.dimensions_2d();

        let x11 = 0.5 * width - THICKNESS;
        let y11 = 0.5 * height - THICKNESS;
        let x12 = x11 + THICKNESS;
        let y12 = y11;
        let x13 = x12;
        let y13 = y11 + THICKNESS;

        let t11 = vec![(x13, y13), (-x13, y13), (-x12, y12)];
        let t12 = vec![(x13, y13), (-x12, y12), (x12, y12)];
        let t21 = vec![(x12, y12), (x11, y11), (x11, -y11)];
        let t22 = vec![(x12, y12), (x11, -y11), (x12, -y12)];

        fn flip_horizontal(points: &Vec<(f32, f32)>) -> Vec<(f32, f32)> {
            points.iter().copied().map(|(x, y)| (-x, y)).rev().collect()
        }
        fn flip_vertical(points: &Vec<(f32, f32)>) -> Vec<(f32, f32)> {
            points.iter().copied().map(|(x, y)| (x, -y)).rev().collect()
        }

        let t11f: Vec<_> = flip_vertical(&t11);
        let t12f: Vec<_> = flip_vertical(&t12);
        let t21f = flip_horizontal(&t21);
        let t22f = flip_horizontal(&t22);

        let mut result = Vec::with_capacity(24);
        for mut v in [t11, t12, t21, t22, t11f, t12f, t21f, t22f] {
            result.append(&mut v);
        }

        result
            .into_iter()
            .map(|(x, y)| Vertex {
                position: [x, y, 0.0],
            })
            .collect()
    }
}

pub struct RectangleRenderer {
    constraints: Vec<SquareCuboid>,
    instances_cache: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_buffer_stale: bool,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer_data: Vec<Vertex>,
    sampling_config: SamplingConfig,
}

impl RectangleRenderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        
    }
}
