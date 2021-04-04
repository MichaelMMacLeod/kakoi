use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

pub struct Camera {
    eye: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    view_projection_matrix_cache: Option<cgmath::Matrix4<f32>>,
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
            view_projection_matrix_cache: None,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.view_projection_matrix_cache = None;
    } 

    pub fn eye(&self) -> &Point3<f32> {
        &self.eye
    }

    pub fn target(&self) -> &Point3<f32> {
        &self.target
    }

    pub fn view_projection_matrix(&mut self) -> &Matrix4<f32> {
        if self.view_projection_matrix_cache.is_none() {
            let view = if self.aspect() > 1.0 {
                Matrix4::look_at_rh(*self.eye(), self.target, self.up)
            } else {
                Matrix4::look_at_rh(
                    (self.eye.x, self.eye.y, 2.5 / self.aspect).into(),
                    self.target,
                    self.up,
                )
            };
            let proj = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);
            self.view_projection_matrix_cache = Some(OPENGL_TO_WGPU_MATRIX * proj * view);
        }

        self.view_projection_matrix_cache.as_ref().unwrap()
    }
}
