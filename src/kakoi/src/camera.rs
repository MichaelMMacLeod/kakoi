use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            eye: (0.0, 0.0, 2.5).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: aspect,
            fovy: 45.0,
            znear: 0.0001,
            zfar: 100.0,
        }
    }
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let target = cgmath::Point3::new(self.eye.x, self.eye.y, 0.0);
        let view = Matrix4::look_at_rh(self.eye, target, self.up);
        let proj = if self.aspect > 1.0 {
            cgmath::Matrix4::from_nonuniform_scale(1.0, self.aspect, 1.0)
        } else {
            cgmath::Matrix4::from_nonuniform_scale(1.0 / self.aspect, 1.0, 1.0)
        };
        let proj = perspective(Deg(self.fovy), 1.0, self.znear, self.zfar) * proj;
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}
