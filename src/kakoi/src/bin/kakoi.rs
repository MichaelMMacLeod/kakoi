use clap::{App, Arg};

fn main() {
    let matches = App::new("kakoi")
        .version("0.1.0")
        .arg(
            Arg::with_name("create-window")
                .long("create-window")
                .short("c")
                .help("Opens a new window"),
        )
        .get_matches();

    if matches.is_present("create-window") {
        kakoi::window::create_window();
    }

    // let args: Vec<String> = env::args().collect();

    // if args.len() != 5 {
    //     println!("arguments: RADIUS COUNT ZOOM ANGLE");
    //     std::process::exit(1);
    // }

    // let radius = args[1].parse::<f64>().unwrap();
    // let count = args[2].parse::<u64>().unwrap();
    // let zoom = args[3].parse::<f64>().unwrap();
    // let angle = args[4].parse::<f64>().unwrap();

    // kakoi::svg::print_circle_svg(std::io::stdout(), radius, count, zoom, angle);
}
