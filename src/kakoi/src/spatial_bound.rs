use crate::{sphere::Sphere, square_cuboid::SquareCuboid};

#[derive(Clone, Copy)]
pub enum SpatialBound {
    Sphere(Sphere),
    SquareCuboid(SquareCuboid),
}

impl SpatialBound {
    fn as_bound_of(&self, other: &SpatialBound) -> SpatialBound {
        match self {
            SpatialBound::Sphere(self_sphere) => match other {
                SpatialBound::Sphere(other_sphere) => self.clone(),
                SpatialBound::SquareCuboid(other_square_cuboid) => {
                    let (width, height) = other_square_cuboid.dimensions_2d();
                    if width > height {
                        
                    }
                    // use crate::render::circle;

                    // if rectangle_aspect_ratio > 1.0 {
                    //     let aspect_ratio_inverse = 1.0 / rectangle_aspect_ratio;
                    //     let scale = circle::MIN_RADIUS * 2.0 * self.radius
                    //         / (aspect_ratio_inverse * aspect_ratio_inverse + 1.0).sqrt();
                    //     (scale, scale * aspect_ratio_inverse)
                    // } else {
                    //     let scale = circle::MIN_RADIUS * 2.0 * self.radius
                    //         / (rectangle_aspect_ratio * rectangle_aspect_ratio + 1.0).sqrt();
                    //     (scale * rectangle_aspect_ratio, scale)
                    // }
                }
            },
        }
    }
}
