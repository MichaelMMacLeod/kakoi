pub enum SamplingConfig {
    Single,
    Multi {
        sample_count: u32,
        multisampled_framebuffer: wgpu::TextureView,
    },
}