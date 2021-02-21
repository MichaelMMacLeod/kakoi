use std::io::Write;
use svg::node::element::Circle as SVGCircle;
use svg::node::element::Rectangle;
use svg::Document;

use crate::circle::{Circle, CirclePositioner, Point};

pub fn print_circle_svg<W: Write>(
    out: W,
    enclosing_radius: f64,
    enclosed_circles: u64,
    zoom: f64,
    focus_angle: f64,
) {
    let document = make_document(enclosing_radius, enclosed_circles, zoom, focus_angle);
    svg::write(out, &document).unwrap();
}

fn make_document(
    enclosing_radius: f64,
    enclosed_circles: u64,
    zoom: f64,
    focus_angle: f64,
) -> Document {
    let center = enclosing_radius;

    let mut document = Document::new().set("viewBox", (0, 0, center * 2.0, center * 2.0));

    // let bg = Rectangle::new()
    //     .set("fill", "none")
    //     .set("stroke", "#000000")
    //     .set("x", 0)
    //     .set("y", 0)
    //     .set("width", center * 2.0)
    //     .set("height", center * 2.0);

    let enclosing_circle = SVGCircle::new()
        .set("fill", "none")
        .set("stroke", "#000000")
        .set("cx", center)
        .set("cy", center)
        .set("r", enclosing_radius);

    document = document /*.add(bg)*/
        .add(enclosing_circle);

    for Circle {
        center: Point { x, y },
        radius,
    } in CirclePositioner::new(
        enclosing_radius,
        enclosed_circles,
        zoom,
        Point {
            x: center,
            y: center,
        },
        focus_angle,
    ) {
        document = document.add(
            SVGCircle::new()
                .set("fill", "none")
                .set("stroke", "#000000")
                .set("cx", x)
                .set("cy", y)
                .set("r", radius),
        );
    }

    document
}
