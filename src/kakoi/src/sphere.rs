#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: cgmath::Vector3<f32>,
    pub radius: f32,
}

impl Sphere {
    // Determines if the the circle is visible on a screen with the given aspect ratio.
    //
    // We assume the following about the coordinate system:
    // 1. (0,0) is in the center of the screen
    // 2. The length of the smaller screen dimension is 2.0 (ranging from -1 to +1)
    //
    // See https://stackoverflow.com/a/402010/8756390 for the algorithm.
    pub fn is_on_screen(&self, aspect_ratio: f32) -> bool {
        let (half_width, half_height) = if aspect_ratio > 1.0 {
            (aspect_ratio, 1.0)
        } else {
            (1.0, 1.0 / aspect_ratio)
        };

        let circle_distance_x = self.center.x.abs();
        let circle_distance_y = self.center.y.abs();

        if circle_distance_x > half_width + self.radius
            || circle_distance_y > half_height + self.radius
        {
            false
        } else if circle_distance_x <= half_width || circle_distance_y <= half_height {
            true
        } else {
            let dx = circle_distance_x - half_width;
            let dy = circle_distance_y - half_height;
            let corner_distance_square = dx * dx + dy * dy;
            corner_distance_square < self.radius * self.radius
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_on_screen_1() {
        assert!(Sphere {
            center: cgmath::vec3(0.0, 0.0, 0.0),
            radius: 1.0
        }
        .is_on_screen(16.0 / 9.0));
    }

    #[test]
    fn is_on_screen_2() {
        assert!(Sphere {
            center: cgmath::vec3(0.0, 0.0, 0.0),
            radius: 0.1
        }
        .is_on_screen(2.0));
    }

    #[test]
    fn is_on_screen_3() {
        assert!(Sphere {
            center: cgmath::vec3(0.0, 0.0, 0.0),
            radius: 100.0
        }
        .is_on_screen(2.0));
    }

    #[test]
    fn is_on_screen_4() {
        assert!(Sphere {
            center: cgmath::vec3(0.0, 0.0, 0.0),
            radius: 0.000001
        }
        .is_on_screen(1.0));
    }

    #[test]
    fn is_on_screen_5() {
        assert!(Sphere {
            center: cgmath::vec3(-1.0, 0.0, 0.0),
            radius: 1.0
        }
        .is_on_screen(1.0));
    }

    #[test]
    fn is_on_screen_6() {
        assert!(!Sphere {
            center: cgmath::vec3(-1.5, 0.0, 0.0),
            radius: 0.49
        }
        .is_on_screen(1.0));
    }

    #[test]
    fn is_on_screen_7() {
        assert!(Sphere {
            center: cgmath::vec3(-1.5, 0.0, 0.0),
            radius: 0.5
        }
        .is_on_screen(1.0));
    }

    #[test]
    fn is_on_screen_8() {
        assert!(Sphere {
            center: cgmath::vec3(-2.5, 0.0, 0.0),
            radius: 0.5
        }
        .is_on_screen(2.0));
        assert!(!Sphere {
            center: cgmath::vec3(-2.5, 0.0, 0.0),
            radius: 0.49
        }
        .is_on_screen(2.0));
        assert!(Sphere {
            center: cgmath::vec3(0.0, 1.4, 0.0),
            radius: 0.49
        }.is_on_screen(2.0));
        assert!(!Sphere {
            center: cgmath::vec3(0.0, 1.5, 0.0),
            radius: 0.49
        }.is_on_screen(2.0));
    }
}
