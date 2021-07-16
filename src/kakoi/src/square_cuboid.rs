#[derive(PartialEq, Eq, Clone, Copy)]
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
#[derive(Clone, Copy)]
pub struct SquareCuboid {
    pub length: f32,
    pub depth: f32,
    pub center: cgmath::Vector3<f32>,
    pub orientation: Orientation,
}

impl SquareCuboid {
    pub fn width(&self) -> f32 {
        match self.orientation {
            Orientation::Horizontal => {
                self.length
            }
            Orientation::Vertical => {
                self.depth
            }
        }
    }

    pub fn height(&self) -> f32 {
        match self.orientation {
            Orientation::Horizontal => {
                self.depth
            }
            Orientation::Vertical => {
                self.length
            }
        }
    }

    pub fn dimensions_2d(&self) -> (f32, f32) {
        match self.orientation {
            Orientation::Horizontal => {
                (self.length, self.depth)
            },
            Orientation::Vertical => {
                (self.depth, self.length)
            }
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