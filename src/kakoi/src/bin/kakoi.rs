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
}
