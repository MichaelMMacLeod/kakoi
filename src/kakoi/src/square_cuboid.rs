#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

/// Represents the 3D shape formed by taking two equal-sized squares and connecting
/// their vertices with four straight parallel lines.
///
/// Here is a picture of a horizontal cuboid (a cuboid whose 'orientatin' is
/// Horizontal).
///
///```
///               length
///    |----------------------------|
///
///    ------------------------------  ---
///   /|            /              /|   |
///  / |           /|             / |   | depth
/// ------------------------------  |   |
/// |  |         center          |  |   |
/// |  -------------|------------|---  ---
/// | /             |/           | /   /
/// |/              /            |/   /  depth
/// ------------------------------  ---
///```
#[derive(Debug, Clone, Copy)]
pub struct SquareCuboid {
    pub length: f32,
    pub depth: f32,
    pub center: cgmath::Vector3<f32>,
    pub orientation: Orientation,
}

impl SquareCuboid {
    /// Splits a cuboid vertically into `n` separate cuboids stacked ontop of
    /// each other. The returned list is ordered from top to bottom.
    pub fn split_vertically(cuboid: SquareCuboid, n: usize) -> Vec<SquareCuboid> {
        let (width, height) = cuboid.dimensions_2d();
        let mut result = Vec::with_capacity(n);
        let offset = height / (2 * n) as f32;
        let h = height / n as f32;

        let x = cuboid.center.x;
        let mut y = cuboid.center.y + offset * (n - 1) as f32;

        for _ in 0..n {
            result.push(Self::from_dimensions(width, h, (x, y, 0.0).into()));
            y -= 2.0 * offset;
        }

        result
    }

    pub fn from_dimensions(width: f32, height: f32, center: cgmath::Vector3<f32>) -> Self {
        if width >= height {
            Self {
                length: width,
                depth: height,
                center,
                orientation: Orientation::Horizontal,
            }
        } else {
            Self {
                length: height,
                depth: width,
                center,
                orientation: Orientation::Vertical,
            }
        }
    }

    pub fn width(&self) -> f32 {
        match self.orientation {
            Orientation::Horizontal => self.length,
            Orientation::Vertical => self.depth,
        }
    }

    pub fn height(&self) -> f32 {
        match self.orientation {
            Orientation::Horizontal => self.depth,
            Orientation::Vertical => self.length,
        }
    }

    pub fn dimensions_2d(&self) -> (f32, f32) {
        match self.orientation {
            Orientation::Horizontal => (self.length, self.depth),
            Orientation::Vertical => (self.depth, self.length),
        }
    }

    /// Returns the cuboid's length or width (whichever is smaller), times its
    /// corresponding dimension (depending on 'orientation').
    pub fn min_screen_dimension(&self, screen_width: f32, screen_height: f32) -> f32 {
        let (x, y) = self.dimensions_2d();
        if x > y {
            y * screen_height
        } else {
            x * screen_width
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.dimensions_2d();
        width / height
    }

    // pub const fn is_horizontal(&self) -> bool {
    //     self.orientation == Orientation::Horizontal
    // }
}
