use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        println!("arguments: RADIUS COUNT ZOOM ANGLE");
        std::process::exit(1);
    }

    let radius = args[1].parse::<f64>().unwrap();
    let count = args[2].parse::<u64>().unwrap();
    let zoom = args[3].parse::<f64>().unwrap();
    let angle = args[4].parse::<f64>().unwrap();

    kakoi::circle::print_circle_svg(std::io::stdout(), radius, count, zoom, angle);
}
