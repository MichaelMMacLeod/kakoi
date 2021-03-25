use crate::sphere::Sphere;

pub trait InstanceRenderer<D> {
    fn new<'a>(
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) -> Self;

    fn with_instance<'a>(&mut self, bounds: Sphere, data: &'a D);

    fn update<'a>(
        &mut self,
        queue: &'a mut wgpu::Queue,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn resize<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn post_render(&mut self);
}
