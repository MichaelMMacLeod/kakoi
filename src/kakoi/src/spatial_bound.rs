use crate::{
    sphere::Sphere,
    square_cuboid::{Orientation, SquareCuboid},
};

#[derive(Debug, Clone, Copy)]
pub enum SpatialBound {
    Sphere(Sphere),
    SquareCuboid(SquareCuboid),
}

impl SpatialBound {
    /// Returns a [Sphere] that fits inside of the specified `SpatialBound`.
    pub fn sphere_inside_bound(spatial_bound: &SpatialBound) -> Sphere {
        match spatial_bound {
            SpatialBound::Sphere(sphere) => *sphere,
            SpatialBound::SquareCuboid(square_cuboid) => {
                SpatialBound::sphere_inside_cuboid(square_cuboid)
            }
        }
    }

    /// Returns a [SquareCuboid] with the given `aspect_ratio` that fits
    /// inside of the specified `SpatialBound`.
    pub fn cuboid_inside_bound(spatial_bound: &SpatialBound, aspect_ratio: f32) -> SquareCuboid {
        match spatial_bound {
            SpatialBound::Sphere(sphere) => {
                SpatialBound::cuboid_inside_sphere(sphere, aspect_ratio)
            }
            SpatialBound::SquareCuboid(square_cuboid) => {
                SpatialBound::cuboid_inside_cuboid(square_cuboid, aspect_ratio)
            }
        }
    }

    /// Returns a [Sphere] that fits inside of the specified [SquareCuboid]. The
    /// center of the sphere is the same as the cuboid's center. The sphere's
    /// radius is made as large as it can be---without overflowing the cuboid.
    pub fn sphere_inside_cuboid(cuboid: &SquareCuboid) -> Sphere {
        Sphere {
            center: cuboid.center,
            radius: cuboid.depth * 0.5,
        }
    }

    /// Returns a [Sphere] that fits inside of the specified sphere. The centers
    /// of both spheres are the same. The returned sphere is made as large as
    /// possible.
    pub fn sphere_inside_sphere(sphere: &Sphere) -> Sphere {
        *sphere
    }

    /// Returns a [SquareCuboid] with the given `aspect_ratio` that fits inside
    /// of the specified [Sphere]. The center of the cuboid is the same as the
    /// sphere's center. The cuboid is made as large as it can be---without
    /// overflowing the sphere.
    pub fn cuboid_inside_sphere(sphere: &Sphere, aspect_ratio: f32) -> SquareCuboid {
        // The height and width calculations came from the following Maxima code:
        //
        // solve([radius^2=(width/2)^2+(height/2)^2,
        //        width/height=aspect_ratio],
        //       [width,height]);
        //
        // ... which returns two answers, one of which has positive 'height' and
        // 'width':
        //
        // height=(2*radius)/sqrt(aspect_ratio^2+1)
        // width=(2*aspect_ratio*radius)/sqrt(aspect_ratio^2+1)
        //
        // Looking at the answers, we can see that 'width' is just
        // 'height * aspect_ratio'.

        let height = 2.0 * sphere.radius / (aspect_ratio.powf(2.0) + 1.0).sqrt();
        let width = height * aspect_ratio;

        let (length, depth, orientation) = if aspect_ratio >= 1.0 {
            (width, height, Orientation::Horizontal)
        } else {
            (height, width, Orientation::Vertical)
        };

        SquareCuboid {
            center: sphere.center,
            length,
            depth,
            orientation,
        }
    }

    /// Fits a [SquareCuboid] with a given `aspect_ratio` inside of another
    /// cuboid (which may have a different aspect ratio).
    pub fn cuboid_inside_cuboid(cuboid: &SquareCuboid, aspect_ratio: f32) -> SquareCuboid {
        let (width, height) = if aspect_ratio < cuboid.aspect_ratio() {
            let height = cuboid.height();
            let width = height * aspect_ratio;
            (width, height)
        } else {
            let width = cuboid.width();
            let height = width / aspect_ratio;
            (width, height)
        };

        let (length, depth, orientation) = if aspect_ratio >= 1.0 {
            (width, height, Orientation::Horizontal)
        } else {
            (height, width, Orientation::Vertical)
        };

        SquareCuboid {
            center: cuboid.center,
            length,
            depth,
            orientation,
        }
    }

    /// Determines if the bounds are large enough to be visible on screen.
    /// Returns true if they are, and false otherwise.
    pub fn is_visible(&self, screen_width: f32, screen_height: f32) -> bool {
        match self {
            SpatialBound::Sphere(s) => s.screen_radius(screen_width, screen_height) > 1.0,
            SpatialBound::SquareCuboid(s) => {
                s.min_screen_dimension(screen_width, screen_height) > 1.0
            }
        }
    }
}
