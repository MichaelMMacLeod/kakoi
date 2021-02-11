#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;

    #[test]
    fn two_circles() {
        use std::f64::consts::PI;

        let outer_radius = 100.0;
        let inner_circle_count = 2;

        let result = crate::math::fit_equal_circles(outer_radius, inner_circle_count);

        assert_eq!(result.is_some(), true);

        if let Some((radius, angle)) = result {
            assert!(approx_eq!(f64, radius, 50.0, ulps = 1, epsilon = 0.001));
            assert!(approx_eq!(f64, angle, PI, ulps = 1, epsilon = 0.001));
        }
    }

    #[test]
    fn three_circles() {
        use std::f64::consts::PI;

        let outer_radius = 100.0;
        let inner_circle_count = 3;

        let result = crate::math::fit_equal_circles(outer_radius, inner_circle_count);

        assert_eq!(result.is_some(), true);

        if let Some((radius, angle)) = result {
            dbg!(radius);
            assert!(approx_eq!(f64, radius, 46.410, ulps = 1, epsilon = 0.001));
            dbg!(angle);
            assert!(approx_eq!(
                f64,
                angle,
                2.0 * PI / 3.0,
                ulps = 1,
                epsilon = 0.001
            ));
        }
    }

    #[test]
    fn eight_circles() {
        use std::f64::consts::PI;

        let outer_radius = 100.0;
        let inner_circle_count = 8;

        let result = crate::math::fit_equal_circles(outer_radius, inner_circle_count);

        assert_eq!(result.is_some(), true);

        if let Some((radius, angle)) = result {
            dbg!(radius);
            assert!(approx_eq!(f64, radius, 27.676, ulps = 1, epsilon = 0.001));
            dbg!(angle);
            assert!(approx_eq!(
                f64,
                angle,
                2.0 * PI / 8.0,
                ulps = 1,
                epsilon = 0.001
            ));
        }
    }
}

pub mod math {
    // This function fits several smaller circles (each of the same size) along
    // the inside circumference of a larger enclosing circle.
    //
    // Given the radius of a circle `outer_radius`, and a number
    // `inner_circle_count`, we return a pair `(radius, theta)`, where `radius`
    // is the radius of each of the inner circles, and `theta` is the angle in
    // radians between the circles.
    //
    // Returns None when the outer radius is not positive or the inner circle
    // count is less than two.
    //
    // See
    // https://math.stackexchange.com/questions/3984340/formula-for-radius-of-circles-on-vertices-of-regular-polygon/3990915#3990915
    pub fn fit_equal_circles(outer_radius: f64, inner_circle_count: u64) -> Option<(f64, f64)> {
        if outer_radius <= 0.0 || inner_circle_count <= 1 {
            None
        } else {
            use std::f64::consts::PI;

            let inner_circle_count = inner_circle_count as f64;
            let s = 1.0 / (PI / 2.0 - PI / inner_circle_count).cos();

            let radius = outer_radius / (s + 1.0);
            let angle = 2.0 * PI / inner_circle_count;

            Some((radius, angle))
        }
    }
}
