#[macro_use] extern crate serde_derive;
#[macro_use] extern crate failure;
#[macro_use] extern crate lazy_static;

extern crate clap;
use clap::{App,SubCommand,Arg};

mod version;
mod config;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(version::VERSION)
        .about("Footage archiver")
        .author("richö butts")
        .arg(Arg::with_name("config")
             .takes_value(true)
             .help("Path to configuration file"))
        .subcommand(SubCommand::with_name("daemon")
                    .version(version::VERSION)
                    .author("richö butts")
                    .about("Runs archiver in persistent mode"))
        .subcommand(SubCommand::with_name("run")
                    .about("Runs archiver in persistent mode")
                    .version(version::VERSION)
                    .author("richö butts")
                    .arg(Arg::with_name("device")
                         .short("d")
                         .takes_value(true)
                         .multiple(true)
                         .help("Upload only from device")))
}

// fn load_config(path: &str) -> config::Config {
//     match config::Config::from_file(path) {
//         Ok(cfg) => cfg,
//         _ => unimplemented!(),
//     }
// }

fn main() {
    let matches = cli_opts().get_matches();

    // Loading config here lets us bail at a convenient time before we get in the weeds

    // let config = load_config(matches.value_of("config").unwrap_or("archiver.toml"));

    match matches.subcommand() {
        ("daemon", Some(subm))  => {
            unimplemented!();
        },
        ("run", Some(subm)) => {


        },
        _ => {unreachable!()}, // Either no subcommand or one not tested for...
    }
}
