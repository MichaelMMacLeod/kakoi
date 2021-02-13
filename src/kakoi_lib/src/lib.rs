pub mod math {
    struct EqualConfig {
        radius: f64,
        angle: f64,
    }

    struct ZoomedConfig {
        large_radius: f64,
        small_radius: f64,
        angle: f64,
    }

    enum CircleLayout {
        Equal(EqualConfig),
        Zoomed(ZoomedConfig),
    }

    fn make_circle_layout(enclosing_radius: f64, enclosed_circles: u64, zoom: f64) -> CircleLayout {
        if zoom == 0.0 {
            let (radius, angle) = fit_equal_circles(enclosing_radius, enclosed_circles);
            CircleLayout::Equal(EqualConfig { radius, angle })
        } else {
            let zoomed_radius = calculate_zoomed_radius(enclosing_radius, enclosed_circles, zoom);
            let (r, t) = find_r_theta(enclosing_radius, zoomed_radius, enclosed_circles);
            CircleLayout::Zoomed(ZoomedConfig {
                large_radius: zoomed_radius,
                small_radius: r,
                angle: t,
            })
        }
    }

    pub fn calculate_zoomed_radius(enclosing_radius: f64, enclosed_circles: u64, zoom: f64) -> f64 {
        let zoom = zoom.abs();
        let zoom = if zoom > 1.0 { 1.0 } else { zoom };
        let (radius, _) = fit_equal_circles(enclosing_radius, enclosed_circles);
        (enclosing_radius - radius) * zoom + radius
    }

    // See https://math.stackexchange.com/questions/4022525/placing-smaller_circle_count-equally-sized-circles-and-one-larger-circle-inside-the-circumference-o/4023200?noredirect=1#comment8307078_4023200
    pub fn find_r_theta(
        enclosing_radius: f64,
        zoomed_radius: f64,
        smaller_circle_count: u64,
    ) -> (f64, f64) {
        if smaller_circle_count == 0 {
            (0.0, 0.0)
        } else if smaller_circle_count == 1 {
            (enclosing_radius - zoomed_radius, 0.0)
        } else if smaller_circle_count == 2 {
            let r = 4.0 * enclosing_radius * zoomed_radius * (enclosing_radius - zoomed_radius)
                / (enclosing_radius + zoomed_radius).powf(2.0);
            (
                r,
                2.0 * r.atan2(
                    enclosing_radius * (3.0 * zoomed_radius - enclosing_radius)
                        / (enclosing_radius + zoomed_radius),
                ),
            )
        } else {
            let smaller_circle_count = smaller_circle_count as f64;
            let mut theta_max = 2.0 * std::f64::consts::PI / smaller_circle_count;
            let mut theta_min = 0.0;
            let ct = 0.5 * (smaller_circle_count - 1.0);
            let cs = 2.0 * zoomed_radius / (enclosing_radius - zoomed_radius);
            let mut theta = 0.0;
            for _ in 0..53 {
                theta = 0.5 * (theta_min + theta_max);
                let d = (ct * theta).cos() - cs * (0.5 * theta).sin() + 1.0;
                if d > 0.0 {
                    theta_min = theta;
                } else if d < 0.0 {
                    theta_max = theta;
                } else {
                    break;
                }
            }

            let s = (0.5 * theta).sin();
            let r = enclosing_radius * s / (s + 1.0);

            if zoomed_radius < r {
                panic!("find_r_theta: zoomed_radius < r");
            } else {
                (enclosing_radius * s / (s + 1.0), theta)
            }
        }
    }

    // This function fits several smaller circles (each of the same size) along
    // the inside circumference of a larger enclosing circle.
    //
    // Given the radius of a circle `outer_radius`, and a number
    // `inner_circle_count`, we return a pair `(radius, theta)`, where `radius`
    // is the radius of each of the inner circles, and `theta` is the angle in
    // radians between the circles.
    //
    // See
    // https://math.stackexchange.com/questions/3984340/formula-for-radius-of-circles-on-vertices-of-regular-polygon/3990915#3990915
    pub fn fit_equal_circles(outer_radius: f64, inner_circle_count: u64) -> (f64, f64) {
        if outer_radius <= 0.0 {
            (0.0, 0.0)
        } else if inner_circle_count == 0 {
            (0.0, 0.0)
        } else if inner_circle_count == 1 {
            (outer_radius, 0.0)
        } else {
            use std::f64::consts::PI;

            let inner_circle_count = inner_circle_count as f64;
            let s = 1.0 / (PI / 2.0 - PI / inner_circle_count).cos();

            let radius = outer_radius / (s + 1.0);
            let angle = 2.0 * PI / inner_circle_count;

            (radius, angle)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use float_cmp::approx_eq;

        #[test]
        fn two_circles() {
            use std::f64::consts::PI;

            let outer_radius = 100.0;
            let inner_circle_count = 2;

            let (radius, angle) = fit_equal_circles(outer_radius, inner_circle_count);
            assert!(approx_eq!(f64, radius, 50.0, ulps = 1, epsilon = 0.001));
            assert!(approx_eq!(f64, angle, PI, ulps = 1, epsilon = 0.001));
        }

        #[test]
        fn three_circles() {
            use std::f64::consts::PI;

            let outer_radius = 100.0;
            let inner_circle_count = 3;

            let (radius, angle) = fit_equal_circles(outer_radius, inner_circle_count);
            assert!(approx_eq!(f64, radius, 46.410, ulps = 1, epsilon = 0.001));
            assert!(approx_eq!(
                f64,
                angle,
                2.0 * PI / 3.0,
                ulps = 1,
                epsilon = 0.001
            ));
        }

        #[test]
        fn eight_circles() {
            use std::f64::consts::PI;

            let outer_radius = 100.0;
            let inner_circle_count = 8;

            let (radius, angle) = fit_equal_circles(outer_radius, inner_circle_count);
            assert!(approx_eq!(f64, radius, 27.676, ulps = 1, epsilon = 0.001));
            assert!(approx_eq!(
                f64,
                angle,
                2.0 * PI / 8.0,
                ulps = 1,
                epsilon = 0.001
            ));
        }

        #[test]
        fn one_smaller_circle() {
            let (r, t) = find_r_theta(100.0, 80.0, 1);

            dbg!(r, t);
            assert!(approx_eq!(f64, r, 100.0 - 80.0, ulps = 3, epsilon = 0.001));
            assert!(approx_eq!(f64, t, 0.0, ulps = 3, epsilon = 0.001));
        }

        #[test]
        fn two_smaller_circles() {
            let (r, t) = find_r_theta(100.0, 80.0, 2);

            dbg!(r, t);
            assert!(approx_eq!(f64, r, 19.753, ulps = 3, epsilon = 0.001));
            assert!(approx_eq!(f64, t, 0.497, ulps = 3, epsilon = 0.001));
        }

        #[test]
        fn ten_smaller_circles() {
            let (r, t) = find_r_theta(100.0, 80.0, 10);

            dbg!(r, t);
            assert!(approx_eq!(f64, r, 13.106, ulps = 3, epsilon = 0.001));
            assert!(approx_eq!(f64, t, 0.302, ulps = 3, epsilon = 0.001));
        }

        #[test]
        fn zoomed_radius_full_zoom() {
            let zoomed_radius = calculate_zoomed_radius(100.0, 3, 1.0);
            dbg!(zoomed_radius);
            assert!(approx_eq!(
                f64,
                zoomed_radius,
                100.0,
                ulps = 3,
                epsilon = 0.001
            ));
        }

        #[test]
        fn zoomed_radius_no_zoom() {
            let zoomed_radius = calculate_zoomed_radius(100.0, 3, 0.0);
            let (radius, _) = fit_equal_circles(100.0, 3);
            assert!(approx_eq!(
                f64,
                zoomed_radius,
                radius,
                ulps = 3,
                epsilon = 0.001
            ));
        }

        #[test]
        fn zoomed_radius_half_zoom() {
            let zoomed_radius = calculate_zoomed_radius(100.0, 3, 0.5);
            let (radius, _) = fit_equal_circles(100.0, 3);
            let expected = (100.0 + radius) / 2.0;
            assert!(approx_eq!(
                f64,
                zoomed_radius,
                expected,
                ulps = 3,
                epsilon = 0.001
            ));
        }
    }
}
