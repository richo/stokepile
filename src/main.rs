#[macro_use]
extern crate serde_derive;

extern crate clap;
use clap::{App,SubCommand,Arg};

mod version;
mod config;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(version::VERSION)
        .about("Footage archiver")
        .author("richö butts")
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
                         .help("Upload only from device")))
}

fn main() {
    cli_opts().get_matches();
}
