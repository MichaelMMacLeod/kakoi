#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: cgmath::Vector3<f32>,
    pub radius: f32,
}

impl Sphere {
    pub fn screen_radius(&self, screen_width: f32, screen_height: f32) -> f32 {
        self.radius * screen_width.max(screen_height) * 0.5
    }
}